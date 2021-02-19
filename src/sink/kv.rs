// Copyright 2020-2021, The Tremor Team
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

#![cfg(not(tarpaulin_include))]
#![allow(dead_code)] // FIXME
use crate::sink::{prelude::*, Reply};
use crate::source::prelude::*;
use async_channel::{bounded, Sender};
use halfbrown::HashMap;
use serde::Deserialize;
use std::boxed::Box;

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    dir: String,
}
pub struct Kv {
    sink_url: TremorURL,
    event_origin_uri: EventOriginUri,
    config: Config,
    is_linked: bool,
    reply_tx: Sender<sink::Reply>,
    db: sled::Db,
}

impl Kv {
    fn execute(&self, cmd: Command) -> Result<Value<'static>> {
        match cmd {
            Command::Get { key } => {
                println!("get: {:?}", key);
                Ok(self
                    .db
                    .get(key)?
                    .map(|v| {
                        let v: &[u8] = &v;
                        Value::Bytes(Cow::from(Vec::from(v)))
                    })
                    .unwrap_or_default())
            }
            Command::Put { key, value } => {
                println!("put: {:?} = {:?}", key, value);
                Ok(self
                    .db
                    .insert(key, value)?
                    .map(|v| {
                        let v: &[u8] = &v;
                        Value::Bytes(Cow::from(Vec::from(v)))
                    })
                    .unwrap_or_default())
            }
        }
    }
}

impl offramp::Impl for Kv {
    fn from_config(config: &Option<OpConfig>) -> Result<Box<dyn Offramp>> {
        if let Some(config) = config {
            let config: Config = serde_yaml::from_value(config.clone())?;

            let (reply_tx, _) = bounded(1);

            let db = sled::open(&config.dir)?;

            Ok(SinkManager::new_box(Kv {
                sink_url: TremorURL::from_onramp_id("kv")?, // dummy value
                event_origin_uri: EventOriginUri::default(),
                config,
                is_linked: true,
                reply_tx,
                db,
            }))
        } else {
            Err("[KV Offramp] Offramp requires a config".into())
        }
    }
}

#[derive(Debug)]
enum Command {
    Get { key: Vec<u8> },
    Put { key: Vec<u8>, value: Vec<u8> },
}

/* FIXME
{"get": {"key": "snot"}}
{"put": {"key": "snot", "value": "badger"}}
{"get": {"key": "snot"}}
{"put": {"key": "snot", "value": "badger!"}}
{"get": {"key": "snot"}}
{"get": {"key": "badger"}}
*/

#[async_trait::async_trait]
impl Sink for Kv {
    async fn on_event(
        &mut self,
        _input: &str,
        _codec: &dyn Codec,
        _codec_map: &HashMap<String, Box<dyn Codec>>,
        event: Event,
    ) -> ResultVec {
        let mut r = Vec::with_capacity(10);

        let cmds = event.value_iter().filter_map(|v| {
            if let Some(g) = v.get("get") {
                Some(Command::Get {
                    key: g.get_bytes("key")?.to_vec(),
                })
            } else if let Some(p) = v.get("put") {
                Some(Command::Put {
                    key: p.get_bytes("key")?.to_vec(),
                    value: p.get_bytes("value")?.to_vec(),
                })
            } else {
                eprintln!("failed to decode command: {}", v);
                error!("failed to decode command: {}", v);
                None
            }
        });

        for c in cmds {
            println!("command: {:?}", c);
            let data = self.execute(c)?;
            let e = Event {
                data: data.into(),
                ..Event::default()
            };
            r.push(Reply::Response(OUT, e))
        }
        Ok(Some(r.into()))
    }

    async fn on_signal(&mut self, _signal: Event) -> ResultVec {
        Ok(None)
    }

    async fn init(
        &mut self,
        _sink_uid: u64,
        _sink_url: &TremorURL,
        _codec: &dyn Codec,
        _codec_map: &HashMap<String, Box<dyn Codec>>,
        _processors: Processors<'_>,
        _is_linked: bool,
        _reply_channel: Sender<sink::Reply>,
    ) -> Result<()> {
        Ok(())
    }

    fn is_active(&self) -> bool {
        true
    }

    fn auto_ack(&self) -> bool {
        true
    }

    fn default_codec(&self) -> &str {
        "json"
    }

    async fn terminate(&mut self) {}
}
