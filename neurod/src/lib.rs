use std::collections::HashMap;

use raft::StateMachine;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum KvCommand {
    Get { key: String },
    Put { key: String, value: String },
    Del { key: String },
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case", tag = "status")]
pub enum KvResponse {
    Ok {
        #[serde(skip_serializing_if = "Option::is_none")]
        value: Option<String>,
    },
    NotFound,
    NotLeader {
        leader_addr: String,
        members: Vec<String>,
    },
    InvalidKey,
}

impl KvResponse {
    #[must_use]
    pub fn ok(value: Option<String>) -> Self {
        Self::Ok { value }
    }
}

#[derive(Default, Debug)]
pub struct KvStore {
    store: HashMap<String, String>,
}

impl KvStore {
    #[must_use]
    pub fn new() -> Self {
        Self {
            store: HashMap::new(),
        }
    }
}

impl StateMachine for KvStore {
    type Command = KvCommand;
    type Response = KvResponse;

    fn apply(&mut self, command: Self::Command) -> Self::Response {
        match command {
            KvCommand::Get { key } => self
                .store
                .get(&key)
                .map_or(KvResponse::NotFound, |v| KvResponse::ok(Some(v.clone()))),
            KvCommand::Put { key, value } => {
                if key.is_empty() || key.len() > 256 {
                    return KvResponse::InvalidKey;
                }
                self.store.insert(key, value);
                KvResponse::ok(None)
            }
            KvCommand::Del { key } => self
                .store
                .remove(&key)
                .map_or(KvResponse::NotFound, |v| KvResponse::ok(Some(v))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_serde() {
        // Serialization
        assert_eq!(
            serde_json::to_string(&KvCommand::Get { key: "foo".into() }).unwrap(),
            r#"{"get":{"key":"foo"}}"#
        );
        assert_eq!(
            serde_json::to_string(&KvCommand::Put {
                key: "foo".into(),
                value: "bar".into()
            })
            .unwrap(),
            r#"{"put":{"key":"foo","value":"bar"}}"#
        );
        assert_eq!(
            serde_json::to_string(&KvCommand::Del { key: "foo".into() }).unwrap(),
            r#"{"del":{"key":"foo"}}"#
        );

        // Deserialization
        assert_eq!(
            serde_json::from_str::<KvCommand>(r#"{"get":{"key":"foo"}}"#).unwrap(),
            KvCommand::Get { key: "foo".into() }
        );
        assert_eq!(
            serde_json::from_str::<KvCommand>(r#"{"put":{"key":"foo","value":"bar"}}"#).unwrap(),
            KvCommand::Put {
                key: "foo".into(),
                value: "bar".into()
            }
        );
        assert_eq!(
            serde_json::from_str::<KvCommand>(r#"{"del":{"key":"foo"}}"#).unwrap(),
            KvCommand::Del { key: "foo".into() }
        );
    }

    #[test]
    fn response_serde() {
        // Serialization
        assert_eq!(
            serde_json::to_string(&KvResponse::Ok {
                value: Some("bar".into())
            })
            .unwrap(),
            r#"{"status":"ok","value":"bar"}"#
        );
        assert_eq!(
            serde_json::to_string(&KvResponse::Ok { value: None }).unwrap(),
            r#"{"status":"ok"}"#
        );
        assert_eq!(
            serde_json::to_string(&KvResponse::NotFound).unwrap(),
            r#"{"status":"not-found"}"#
        );
        assert_eq!(
            serde_json::to_string(&KvResponse::InvalidKey).unwrap(),
            r#"{"status":"invalid-key"}"#
        );
        assert_eq!(
            serde_json::to_string(&KvResponse::NotLeader {
                leader_addr: "127.0.0.1:7000".into(),
                members: vec!["127.0.0.1:7000".into(), "127.0.0.1:7001".into()]
            })
            .unwrap(),
            r#"{"status":"not-leader","leader_addr":"127.0.0.1:7000","members":["127.0.0.1:7000","127.0.0.1:7001"]}"#
        );

        // Deserialization
        assert_eq!(
            serde_json::from_str::<KvResponse>(r#"{"status":"ok","value":"bar"}"#).unwrap(),
            KvResponse::Ok {
                value: Some("bar".into())
            }
        );
        assert_eq!(
            serde_json::from_str::<KvResponse>(r#"{"status":"ok"}"#).unwrap(),
            KvResponse::Ok { value: None }
        );
        assert_eq!(
            serde_json::from_str::<KvResponse>(r#"{"status":"not-found"}"#).unwrap(),
            KvResponse::NotFound
        );
        assert_eq!(
            serde_json::from_str::<KvResponse>(r#"{"status":"invalid-key"}"#).unwrap(),
            KvResponse::InvalidKey
        );
        assert_eq!(
            serde_json::from_str::<KvResponse>(
                r#"{"status":"not-leader","leader_addr":"127.0.0.1:7000","members":["127.0.0.1:7000","127.0.0.1:7001"]}"#
            ).unwrap(),
            KvResponse::NotLeader {
                leader_addr: "127.0.0.1:7000".into(),
                members: vec!["127.0.0.1:7000".into(), "127.0.0.1:7001".into()]
            }
        );
    }
}
