use crate::connectors::google::AuthInterceptor;
use crate::connectors::prelude::*;
use async_std::channel::{Receiver, Sender};
use async_std::prelude::FutureExt;
use async_std::sync::Mutex;
use dashmap::DashMap;
use googapis::google::pubsub::v1::subscriber_client::SubscriberClient;
use googapis::google::pubsub::v1::{
    AcknowledgeRequest, PubsubMessage, PullRequest, ReceivedMessage,
};
use gouth::Token;
use serde::Deserialize;
use std::fmt::Debug;
use std::sync::Arc;
use std::time::Duration;
use tonic::codegen::InterceptedService;
use tonic::transport::{Certificate, Channel, ClientTlsConfig};
use tonic::Status;
use tremor_pipeline::ConfigImpl;

#[derive(Deserialize, Clone)]
struct Config {
    pub connect_timeout: u64,
    pub subscription_id: String,
}
impl ConfigImpl for Config {}

#[derive(Debug, Default)]
pub(crate) struct Builder {}

#[async_trait::async_trait]
impl ConnectorBuilder for Builder {
    fn connector_type(&self) -> ConnectorType {
        "gsub".into()
    }

    async fn build(&self, alias: &str, raw_config: &ConnectorConfig) -> Result<Box<dyn Connector>> {
        if let Some(raw) = &raw_config.config {
            let config = Config::new(raw)?;
            Ok(Box::new(GSub { config }))
        } else {
            Err(ErrorKind::MissingConfiguration(alias.to_string()).into())
        }
    }
}

struct GSub {
    config: Config,
}

type PubSubClient = SubscriberClient<InterceptedService<Channel, AuthInterceptor>>;
type AsyncTaskMessage = Result<(u64, Vec<u8>)>;

struct GSubSource {
    config: Config,
    client: Option<Arc<Mutex<PubSubClient>>>,
    receiver: Option<Receiver<AsyncTaskMessage>>,
    ack_ids: Arc<DashMap<u64, String>>,
}

impl GSubSource {
    pub fn new(config: Config) -> Self {
        GSubSource {
            config,
            client: None,
            receiver: None,
            ack_ids: Arc::new(DashMap::new()),
        }
    }
}

async fn consumer_task(
    client: Arc<Mutex<PubSubClient>>,
    sender: Sender<AsyncTaskMessage>,
    ack_ids: Arc<DashMap<u64, String>>,
    subscription_id: String,
    request_timeout: Duration,
) {
    let mut ack_counter = 0;

    loop {
        let response_with_potential_timeout = client
            .lock()
            .await
            .pull(PullRequest {
                subscription: subscription_id.clone(),
                max_messages: 1000,
                ..PullRequest::default()
            })
            .timeout(request_timeout)
            .await;

        let response = match response_with_potential_timeout {
            Ok(response) => match response {
                Ok(response) => response.into_inner(),
                Err(error) => {
                    if let Err(e) = sender.send(Err(error.into())).await {
                        error!("Failed to send a PubSub error to the main task: {}", e);

                        // If we can't send errors to the main task, disconnect and let it restart
                        return;
                    }

                    // There was an error other than timeout, let the main task handle the failure, but
                    // do not drop the connection.
                    continue;
                }
            },
            Err(timeout_error) => {
                if let Err(e) = sender.send(Err(timeout_error.into())).await {
                    error!(
                        "Failed to send a PubSub timeout error to the main task: {}",
                        e
                    );

                    // If we can't send errors to the main task, disconnect and let it restart
                    return;
                }

                // If we have a timeout, exit the task and let the main task handle reconnection
                return;
            }
        };

        for message in response.received_messages {
            let ReceivedMessage {
                ack_id,
                message: msg,
                ..
            } = message;

            if let Some(PubsubMessage { data, .. }) = msg {
                ack_counter += 1;
                ack_ids.insert(ack_counter, ack_id);
                if let Err(e) = sender.send(Ok((ack_counter, data))).await {
                    error!("Failed to send a PubSub message to the main task: {}", e);

                    // If we can't send to the main task, disconnect and let it restart
                    return;
                }
            }
        }
    }
}

#[async_trait::async_trait]
impl Source for GSubSource {
    async fn connect(&mut self, _ctx: &SourceContext, _attempt: &Attempt) -> Result<bool> {
        let token = Token::new()?;

        let tls_config = ClientTlsConfig::new()
            .ca_certificate(Certificate::from_pem(googapis::CERTIFICATES))
            .domain_name("pubsub.googleapis.com");

        let channel = Channel::from_static("https://pubsub.googleapis.com")
            .connect_timeout(Duration::from_nanos(self.config.connect_timeout))
            .tls_config(tls_config)?
            .connect()
            .await;

        let channel = dbg!(channel)?;

        let client = SubscriberClient::with_interceptor(
            channel,
            AuthInterceptor {
                token: Box::new(move || match token.header_value() {
                    Ok(val) => Ok(val),
                    Err(_) => Err(Status::unavailable(
                        "Failed to retrieve authentication token.",
                    )),
                }),
            },
        );

        let client = Arc::new(Mutex::new(client));

        let (tx, rx) = async_std::channel::bounded(QSIZE.load(Ordering::Relaxed));

        async_std::task::spawn(consumer_task(
            client.clone(),
            tx,
            self.ack_ids.clone(),
            self.config.subscription_id.clone(),
            Duration::from_secs(10), // fixme get this from the config
        ));

        self.receiver = Some(rx);
        self.client = Some(client);

        Ok(true)
    }

    async fn pull_data(&mut self, pull_id: &mut u64, ctx: &SourceContext) -> Result<SourceReply> {
        let receiver = self
            .receiver
            .as_mut()
            .ok_or(ErrorKind::BigQueryClientNotAvailable(
                // fixme use an error for GPubSub
                "The receiver is not connected",
            ))?;

        let (ack_id, data) = match receiver.recv().await? {
            Ok(response) => response,
            Err(error) => {
                let error_kind = error.kind();
                return if let ErrorKind::Timeout(_) = error_kind {
                    ctx.swallow_err(
                        ctx.notifier.connection_lost().await,
                        "Failed to notify about PubSub connection loss",
                    );

                    // fixme use the actual stream id
                    Ok(SourceReply::StreamFail(0))
                } else {
                    Err(error)
                };
            }
        };

        *pull_id = ack_id;

        Ok(SourceReply::Data {
            origin_uri: EventOriginUri::default(),
            data,
            meta: None,
            stream: None,
            port: None,
            codec_overwrite: None,
        })
    }

    fn is_transactional(&self) -> bool {
        true
    }

    fn asynchronous(&self) -> bool {
        true
    }

    async fn ack(&mut self, _stream_id: u64, pull_id: u64, _ctx: &SourceContext) -> Result<()> {
        let client = self
            .client
            .as_mut()
            .ok_or(ErrorKind::PubSubClientNotAvailable(
                "The client is not connected",
            ))?;

        let (_, ack_id) = self.ack_ids.remove(&pull_id).ok_or(ErrorKind::PubSubError(
            "Received an ACK for a message that does not exist",
        ))?;

        client
            .lock()
            .await
            .acknowledge(AcknowledgeRequest {
                subscription: self.config.subscription_id.clone(),
                ack_ids: vec![ack_id],
            })
            .await?;

        Ok(())
    }
}

#[async_trait::async_trait]
impl Connector for GSub {
    async fn create_source(
        &mut self,
        source_context: SourceContext,
        builder: SourceManagerBuilder,
    ) -> Result<Option<SourceAddr>> {
        let source = GSubSource::new(self.config.clone());
        builder.spawn(source, source_context).map(Some)
    }

    fn codec_requirements(&self) -> CodecReq {
        CodecReq::Required
    }
}