// Copyright 2018, Wayfair GmbH
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! # Offramps to send data to external systems

#[cfg(feature = "bench")]
pub mod blackhole;
pub mod debug;
pub mod elastic;
pub mod file;
pub mod gcs;
pub mod gpub;
pub mod influx;
#[cfg(feature = "kafka")]
pub mod kafka;
pub mod null;
pub mod stdout;
use crate::errors::*;
use crate::pipeline::prelude::*;
use std::boxed::Box;

/// Enum of all offramp connectors we have implemented.
/// New connectors need to be added here.
#[derive(Debug)]
pub enum Offramp {
    #[cfg(feature = "bench")]
    Blackhole(Box<blackhole::Offramp>),
    #[cfg(not(feature = "bench"))]
    Blackhole(null::Offramp),
    #[cfg(feature = "kafka")]
    Kafka(kafka::Offramp),
    // We have to cheat a little here since the opable macro can't
    // have dependable compilation
    #[cfg(not(feature = "kafka"))]
    Kafka(null::Offramp),
    Elastic(Box<elastic::Offramp>),
    Influx(Box<influx::Offramp>),
    Stdout(stdout::Offramp),
    Debug(debug::Offramp),
    Null(null::Offramp),
    File(file::Offramp),
    GCS(Box<gcs::Offramp>),
    Gpub(Box<gpub::Offramp>),
}

opable!(Offramp, Blackhole, Kafka, Elastic, Influx, Stdout, Debug, Null, File, GCS, Gpub);

impl Offramp {
    pub fn create(name: &str, opts: &ConfValue) -> Result<Offramp> {
        match name {
            #[cfg(feature = "bench")]
            "blackhole" => Ok(Offramp::Blackhole(Box::new(blackhole::Offramp::create(
                opts,
            )?))),
            "debug" => Ok(Offramp::Debug(debug::Offramp::create(opts)?)),
            "elastic" => Ok(Offramp::Elastic(Box::new(elastic::Offramp::create(opts)?))),
            "file" => Ok(Offramp::File(file::Offramp::create(opts)?)),
            "gcs" => Ok(Offramp::GCS(Box::new(gcs::Offramp::create(opts)?))),
            "influx" => Ok(Offramp::Influx(Box::new(influx::Offramp::create(opts)?))),
            "kafka" => Ok(Offramp::Kafka(kafka::Offramp::create(opts)?)),
            "null" => Ok(Offramp::Null(null::Offramp::create(opts)?)),
            "stdout" => Ok(Offramp::Stdout(stdout::Offramp::create(opts)?)),
            "gpub" => Ok(Offramp::Gpub(Box::new(gpub::Offramp::create(opts)?))),
            #[cfg(feature = "kafka")]
            _ => Err(ErrorKind::UnknownOp("offramp".into(), name.into()).into()),
        }
    }
}
