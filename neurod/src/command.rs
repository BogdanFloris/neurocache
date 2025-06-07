use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Command {
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

        let m: Command = serde_json::from_str(data).unwrap();
        assert!(matches!(m, Command::Get { .. }));

        let data = r#"
        {
            "put": {
                "key": "foo",
                "value": "bar"
            }

        }"#;

        let m: Command = serde_json::from_str(data).unwrap();
        assert!(matches!(m, Command::Put { .. }));

        let data = r#"
        {
            "del": {
                "key": "foo"
            }

        }"#;

        let m: Command = serde_json::from_str(data).unwrap();
        assert!(matches!(m, Command::Del { .. }));
    }

    #[test]
    fn deserialize() {
        let m = Command::Get {
            key: String::from("foo"),
        };
        let json = serde_json::to_string(&m).unwrap();
        assert_eq!(json, r#"{"get":{"key":"foo"}}"#);

        let m = Command::Put {
            key: String::from("foo"),
            value: String::from("bar"),
        };
        let json = serde_json::to_string(&m).unwrap();
        assert_eq!(json, r#"{"put":{"key":"foo","value":"bar"}}"#);

        let m = Command::Del {
            key: String::from("foo"),
        };
        let json = serde_json::to_string(&m).unwrap();
        assert_eq!(json, r#"{"del":{"key":"foo"}}"#);
    }
}
