use std::{error::Error, net::SocketAddr};

use serde::{Deserialize, Serialize};

use crate::NodeId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Peer {
    pub id: NodeId,
    pub addr: SocketAddr,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub id: NodeId,
    pub addr: SocketAddr,
    pub peers: Vec<Peer>,
}

impl Config {
    /// Loads a configuration from a JSON file
    ///
    /// # Errors
    ///
    /// This function will return an error in two cases:
    /// * reading the file errors
    /// * desearializing the content of the file errors
    pub fn from_file(path: &str) -> Result<Self, Box<dyn Error>> {
        let file_content = std::fs::read_to_string(path)?;
        let config: Config = serde_json::from_str(&file_content)?;
        Ok(config)
    }
}
