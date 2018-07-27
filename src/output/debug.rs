use error::TSError;
use grouping::MaybeMessage;
use output::utils::{Output as OutputT, OUTPUT_DELIVERED, OUTPUT_DROPPED};
use std::collections::HashMap;
use std::time::{Duration, Instant};

struct DebugBucket {
    pass: u64,
    drop: u64,
}
pub struct Output {
    last: Instant,
    update_time: Duration,
    buckets: HashMap<String, DebugBucket>,
    drop: u64,
    pass: u64,
}

impl Output {
    pub fn new(_opts: &str) -> Self {
        Output {
            last: Instant::now(),
            update_time: Duration::from_secs(1),
            buckets: HashMap::new(),
            pass: 0,
            drop: 0,
        }
    }
}
impl OutputT for Output {
    fn send<'m>(&mut self, msg: MaybeMessage<'m>) -> Result<Option<f64>, TSError> {
        if self.last.elapsed() > self.update_time {
            self.last = Instant::now();
            println!("");
            println!(
                "|{:20}| {:7}| {:7}| {:7}|",
                "classification", "total", "pass", "drop"
            );
            println!(
                "|{:20}| {:7}| {:7}| {:7}|",
                "TOTAL",
                self.pass + self.drop,
                self.pass,
                self.drop
            );
            self.pass = 0;
            self.drop = 0;
            for (class, data) in self.buckets.iter() {
                println!(
                    "|{:20}| {:7}| {:7}| {:7}|",
                    class,
                    data.pass + data.drop,
                    data.pass,
                    data.drop
                );
            }
            println!("");
            self.buckets.clear();
        }
        let entry = self.buckets
            .entry(String::from(msg.classification))
            .or_insert(DebugBucket { pass: 0, drop: 0 });
        if msg.drop {
            OUTPUT_DROPPED.inc();
            entry.drop += 1;
            self.drop += 1;
        } else {
            OUTPUT_DELIVERED.inc();
            entry.pass += 1;
            self.pass += 1;
        };
        Ok(None)
    }
}
