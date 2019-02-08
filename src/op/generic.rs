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

//! # Generic operators
//!
//! Generic operators to control flow and manipulate event metadata

pub mod copy;
pub mod count;
pub mod route;

use crate::errors::*;
use crate::pipeline::prelude::*;

/// Enum of all offramp connectors we have implemented.
/// New connectors need to be added here.
#[derive(Debug)]
pub enum Generic {
    Copy(copy::Op),
    Count(count::Op),
    Route(route::Op),
}

opable!(Generic, Count, Copy, Route);

impl Generic {
    pub fn create(name: &str, opts: &ConfValue) -> Result<Generic> {
        match name {
            "copy" => Ok(Generic::Copy(copy::Op::create(opts)?)),
            "count" => Ok(Generic::Count(count::Op::create(opts)?)),
            "route" => Ok(Generic::Route(route::Op::create(opts)?)),
            _ => Err(ErrorKind::UnknownOp("op".into(), name.into()).into()),
        }
    }
}
