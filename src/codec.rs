// Copyright 2018-2020, Wayfair GmbH
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

use crate::errors::Result;
use simd_json::BorrowedValue;
use tremor_script::LineValue;
pub(crate) mod binflux;
pub(crate) mod influx;
pub(crate) mod json;
pub(crate) mod msgpack;
pub(crate) mod null;
pub(crate) mod statsd;
pub(crate) mod string;
pub(crate) mod yaml;

mod prelude {
    pub use super::Codec;
    pub use crate::errors::*;
    pub use simd_json::prelude::*;
    pub use tremor_script::prelude::*;
}

/// The codec trait, to encode and decode data
pub trait Codec: Send + Sync {
    /// The canonical name for this codec
    fn name(&self) -> std::string::String;
    /// Decode a binary, into an Value
    ///
    /// # Errors
    ///  * if we cna't decode the data
    fn decode(&mut self, data: Vec<u8>, ingest_ns: u64) -> Result<Option<LineValue>>;
    /// Encodes a Value into a binary
    ///
    /// # Errors
    ///  * If the encoding fails
    fn encode(&self, data: &BorrowedValue) -> Result<Vec<u8>>;
    /// Encodes into an existing buffer
    ///
    /// # Errors
    ///  * when we can't write encode to the given vector
    fn encode_into(&self, data: &BorrowedValue, dst: &mut Vec<u8>) -> Result<()> {
        let mut res = self.encode(data)?;
        std::mem::swap(&mut res, dst);
        Ok(())
    }
}

/// Codec lookup function
///
/// # Errors
///  * if the codec doesn't exist
pub fn lookup(name: &str) -> Result<Box<dyn Codec>> {
    match name {
        "json" => Ok(Box::new(json::JSON {})),
        "msgpack" => Ok(Box::new(msgpack::MsgPack {})),
        "influx" => Ok(Box::new(influx::Influx {})),
        "binflux" => Ok(Box::new(binflux::BInflux {})),
        "null" => Ok(Box::new(null::Null {})),
        "string" => Ok(Box::new(string::String {})),
        "statsd" => Ok(Box::new(statsd::StatsD {})),
        "yaml" => Ok(Box::new(yaml::YAML {})),
        _ => Err(format!("Codec '{}' not found.", name).into()),
    }
}
