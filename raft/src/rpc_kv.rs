use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Rpc {
    Get { key: String },
    Put { key: String, value: String },
    Del { key: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize() {
        let data = r#"
        {
            "get": {
                "key": "foo"
            }

        }"#;

        let m: Rpc = serde_json::from_str(data).unwrap();
        assert!(matches!(m, Rpc::Get { .. }));

        let data = r#"
        {
            "put": {
                "key": "foo",
                "value": "bar"
            }

        }"#;

        let m: Rpc = serde_json::from_str(data).unwrap();
        assert!(matches!(m, Rpc::Put { .. }));

        let data = r#"
        {
            "del": {
                "key": "foo"
            }

        }"#;

        let m: Rpc = serde_json::from_str(data).unwrap();
        assert!(matches!(m, Rpc::Del { .. }));
    }

    #[test]
    fn deserialize() {
        let m = Rpc::Get {
            key: String::from("foo"),
        };
        let json = serde_json::to_string(&m).unwrap();
        assert_eq!(json, r#"{"get":{"key":"foo"}}"#);

        let m = Rpc::Put {
            key: String::from("foo"),
            value: String::from("bar"),
        };
        let json = serde_json::to_string(&m).unwrap();
        assert_eq!(json, r#"{"put":{"key":"foo","value":"bar"}}"#);

        let m = Rpc::Del {
            key: String::from("foo"),
        };
        let json = serde_json::to_string(&m).unwrap();
        assert_eq!(json, r#"{"del":{"key":"foo"}}"#);
    }
}
