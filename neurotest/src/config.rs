use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("TOML parse error: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub cluster: ClusterConfig,
    pub paths: PathConfig,
    pub monitoring: MonitoringConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterConfig {
    pub base_raft_port: u16,
    pub base_metrics_port: u16,
    pub default_nodes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathConfig {
    pub logs_dir: PathBuf,
    pub state_file: PathBuf,
    pub test_cluster_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    pub enabled: bool,
    pub prometheus_port: u16,
    pub grafana_port: u16,
    pub compose_file: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    pub id: u64,
    pub raft_addr: String,
    pub metrics_addr: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterState {
    pub nodes: Vec<NodeState>,
    pub started_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeState {
    pub id: u64,
    pub pid: u32,
    pub raft_port: u16,
    pub metrics_port: u16,
}

impl Config {
    pub fn load() -> Result<Self, ConfigError> {
        let config_path = Self::config_path();

        if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            Ok(toml::from_str(&content)?)
        } else {
            let default = Self::default();
            default.save()?;
            Ok(default)
        }
    }

    pub fn save(&self) -> Result<(), ConfigError> {
        let config_path = Self::config_path();
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self).unwrap();
        fs::write(&config_path, content)?;
        Ok(())
    }

    fn config_path() -> PathBuf {
        Path::new("neurotest").join("cluster.toml")
    }

    pub fn generate_node_configs(&self, num_nodes: usize) -> Vec<NodeConfig> {
        (0..num_nodes)
            .map(|i| {
                let id = (i + 1) as u64;
                let i = u16::try_from(i).unwrap_or(0);
                let raft_port = self.cluster.base_raft_port + i;
                let metrics_port = self.cluster.base_metrics_port + i;

                NodeConfig {
                    id,
                    raft_addr: format!("127.0.0.1:{raft_port}"),
                    metrics_addr: format!("0.0.0.0:{metrics_port}"),
                }
            })
            .collect()
    }

    pub fn write_node_config(&self, node: &NodeConfig) -> Result<PathBuf, ConfigError> {
        let config_file = self
            .paths
            .test_cluster_dir
            .join(format!("node_{}.json", node.id));

        // Generate peers list (all nodes except this one)
        let all_nodes = self.generate_node_configs(self.cluster.default_nodes);
        let peers: Vec<_> = all_nodes
            .iter()
            .filter(|n| n.id != node.id)
            .map(|n| {
                serde_json::json!({
                    "id": n.id,
                    "addr": n.raft_addr
                })
            })
            .collect();

        let node_config = serde_json::json!({
            "id": node.id,
            "addr": node.raft_addr,
            "peers": peers
        });

        fs::write(&config_file, serde_json::to_string_pretty(&node_config)?)?;
        Ok(config_file)
    }

    pub fn state_file(&self) -> &Path {
        &self.paths.state_file
    }

    pub fn logs_dir(&self) -> &Path {
        &self.paths.logs_dir
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            cluster: ClusterConfig {
                base_raft_port: 3001,
                base_metrics_port: 9091,
                default_nodes: 3,
            },
            paths: PathConfig {
                logs_dir: PathBuf::from("logs"),
                state_file: PathBuf::from(".cluster_state.json"),
                test_cluster_dir: PathBuf::from("."),
            },
            monitoring: MonitoringConfig {
                enabled: true,
                prometheus_port: 9090,
                grafana_port: 3000,
                compose_file: PathBuf::from("docker-compose.yml"),
            },
        }
    }
}
