use std::{collections::HashMap, error::Error};

use raft::StateMachine;
use serde::{Deserialize, Serialize};
use tracing::subscriber::SetGlobalDefaultError;

#[derive(Debug, thiserror::Error)]
pub enum NeuroError {
    #[error("tracing: {0}")]
    Tracing(#[from] SetGlobalDefaultError),
    #[error("config: {0}")]
    Config(#[from] Box<dyn Error>)
}

const MAX_KEY_LEN: usize = 256;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum KvCommand {
    Get { key: String },
    Put { key: String, value: String },
    Del { key: String },
    Noop
}

impl Default for KvCommand {
    fn default() -> Self {
        Self::Noop
    }
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
            KvCommand::Noop => KvResponse::ok(None),
            KvCommand::Get { key } => self
                .store
                .get(&key)
                .map_or(KvResponse::NotFound, |v| KvResponse::ok(Some(v.clone()))),
            KvCommand::Put { key, value } => {
                if key.is_empty() || key.len() > MAX_KEY_LEN || !key.is_ascii() {
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
mod serde_tests {
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

#[cfg(test)]
mod store_tests {
    use super::*;
    use proptest::prelude::*;

    fn valid_key_strategy() -> impl Strategy<Value = String> {
        "[a-zA-Z0-9]{1,255}"
    }

    fn invalid_key_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            Just(String::new()),     // empty
            "[a-zA-Z0-9]{257,1000}", // over max len
            ".*[^\x00-\x7F]+.*",     // non-ascii
        ]
    }

    fn value_strategy() -> impl Strategy<Value = String> {
        prop::string::string_regex(".{0,10000}").unwrap()
    }

    proptest! {
        #[test]
        fn test_put_get_invariant(key in valid_key_strategy(), value in value_strategy()) {
            let mut store = KvStore::new();
            let put_resp = store.apply(KvCommand::Put { key: key.clone(), value: value.clone() });
            prop_assert_eq!(put_resp, KvResponse::ok(None));

            let get_resp = store.apply(KvCommand::Get { key: key.clone() });
            prop_assert_eq!(get_resp, KvResponse::ok(Some(value)));
        }

        #[test]
        fn test_put_del_get_invariant(key in valid_key_strategy(), value in value_strategy()) {
            let mut store = KvStore::new();

            store.apply(KvCommand::Put { key: key.clone(), value });
            let del_resp = store.apply(KvCommand::Del { key: key.clone() });
            match del_resp {
                KvResponse::Ok { value: Some(_) } => {},
                _ => prop_assert!(false, "Delete should return Ok with value"),
            }

            let get_resp = store.apply(KvCommand::Get { key });
            prop_assert_eq!(get_resp, KvResponse::NotFound);
        }

        #[test]
        fn test_invalid_key_rejection(key in invalid_key_strategy(), value in value_strategy()) {
            let mut store = KvStore::new();

            let put_resp = store.apply(KvCommand::Put { key, value });
            prop_assert_eq!(put_resp, KvResponse::InvalidKey);
        }

        #[test]
        fn test_get_nonexistent_key(key in valid_key_strategy()) {
            let mut store = KvStore::new();

            let get_resp = store.apply(KvCommand::Get { key });
            prop_assert_eq!(get_resp, KvResponse::NotFound);
        }

        #[test]
        fn test_del_nonexistent_key(key in valid_key_strategy()) {
            let mut store = KvStore::new();

            let del_resp = store.apply(KvCommand::Del { key });
            prop_assert_eq!(del_resp, KvResponse::NotFound);
        }

        #[test]
        fn test_two_puts_same_key(key in valid_key_strategy(), values in prop::collection::vec(value_strategy(), 2)) {
            let mut store = KvStore::new();

            for value in &values {
                let put_resp = store.apply(KvCommand::Put { key: key.clone(), value: value.clone() });
                prop_assert_eq!(put_resp, KvResponse::ok(None));
            }

            // Should have the last value
            let get_resp = store.apply(KvCommand::Get { key });
            prop_assert_eq!(get_resp, KvResponse::ok(Some(values.last().unwrap().clone())));
        }
    }
}
