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

//! # JSON parser
//!
//! Parses JSON data.
//!
//! ## Configuration
//!
//! This operator takes no configuration

use crate::errors::*;
use crate::pipeline::prelude::*;
use serde_json;

#[derive(Debug)]
pub struct Parser {}
impl Parser {
    pub fn create(_opts: &ConfValue) -> Result<Self> {
        Ok(Parser {})
    }
}

impl Opable for Parser {
    fn on_event(&mut self, event: EventData) -> EventResult {
        ensure_type!(event, "parse::json", ValueType::Raw);
        let res = event.replace_value(|val| {
            if let EventValue::Raw(raw) = val {
                let doc = serde_json::from_slice(raw)?;
                Ok(EventValue::JSON(doc))
            } else {
                unreachable!()
            }
        });

        match res {
            Ok(n) => next!(n),
            Err(e) => e,
        }
    }
    opable_types!(ValueType::Raw, ValueType::JSON);
}

#[derive(Debug)]
pub struct Renderer {}
impl Renderer {
    pub fn create(_opts: &ConfValue) -> Result<Self> {
        Ok(Self {})
    }
}

impl Opable for Renderer {
    fn on_event(&mut self, event: EventData) -> EventResult {
        ensure_type!(event, "render::json", ValueType::JSON);

        let res = event.replace_value(|val| {
            if let EventValue::JSON(ref val) = val {
                let json = serde_json::to_vec(val)?;
                Ok(EventValue::Raw(json))
            } else {
                unreachable!()
            }
        });
        match res {
            Ok(n) => next!(n),
            Err(e) => e,
        }
    }
    opable_types!(ValueType::JSON, ValueType::Raw);
}
