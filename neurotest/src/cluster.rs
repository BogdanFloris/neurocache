use crate::config::{ClusterState, Config, NodeState};
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::{fs, num::TryFromIntError, process::Stdio, time::Duration};
use thiserror::Error;
use tokio::{process::Command, time::sleep};

#[derive(Error, Debug)]
pub enum ClusterError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Config error: {0}")]
    Config(#[from] crate::config::ConfigError),
    #[error("Cluster already running")]
    AlreadyRunning,
    #[error("No cluster is running")]
    NotRunning,
    #[error("Failed to start node {0}")]
    NodeStartFailed(u64),
    #[error("Dependency not found: {0}")]
    DependencyNotFound(String),
    #[error("failed to convert index to usize")]
    CastError(#[from] TryFromIntError),
}

pub struct ClusterManager {
    config: Config,
}

impl ClusterManager {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub async fn start(&self, num_nodes: usize, single_mode: bool) -> Result<(), ClusterError> {
        if self.is_running() {
            return Err(ClusterError::AlreadyRunning);
        }

        Self::check_dependencies()?;

        fs::create_dir_all(&self.config.paths.logs_dir)?;
        fs::create_dir_all(&self.config.paths.test_cluster_dir)?;

        println!("{}", "Building project...".yellow());
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .unwrap(),
        );
        pb.set_message("Running cargo build...");
        pb.enable_steady_tick(Duration::from_millis(100));

        let build_status = Command::new("cargo")
            .args(["build", "--bin", "neurod"])
            .status()
            .await?;

        pb.finish_and_clear();

        if !build_status.success() {
            println!("{}", "✗ Build failed!".red());
            return Err(ClusterError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Build failed",
            )));
        }
        println!("{}", "✓ Build successful".green());

        let nodes = self.config.generate_node_configs(num_nodes);
        let mut state = ClusterState {
            nodes: Vec::new(),
            started_at: chrono::Utc::now().to_rfc3339(),
        };

        // Start each node
        println!("\n{}", format!("Starting {num_nodes} nodes...").green());
        for node in &nodes {
            print!("Starting node {}... ", node.id);

            // Write node config file
            let config_file = self.config.write_node_config(node)?;

            // Start the node
            let log_file = self.config.logs_dir().join(format!("node_{}.log", node.id));
            let log_file_handle = fs::File::create(&log_file)?;

            let mut cmd = Command::new("cargo");
            cmd.args(["run", "--bin", "neurod", "--"])
                .args(["--config-file", config_file.to_str().unwrap()])
                .args(["--metrics-addr", &node.metrics_addr])
                .stdout(Stdio::from(log_file_handle.try_clone()?))
                .stderr(Stdio::from(log_file_handle));

            if single_mode && node.id == 1 {
                cmd.arg("--single");
            }

            let child = cmd.spawn()?;
            let pid = child.id().ok_or_else(|| {
                ClusterError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Failed to get process ID",
                ))
            })?;

            sleep(Duration::from_secs(2)).await;

            if !Self::is_process_running(pid)? {
                println!("{}", "✗ Failed".red());
                return Err(ClusterError::NodeStartFailed(node.id));
            }

            println!("{}", format!("✓ Started (PID: {pid})").green());

            state.nodes.push(NodeState {
                id: node.id,
                pid,
                raft_port: node
                    .raft_addr
                    .split(':')
                    .next_back()
                    .unwrap()
                    .parse()
                    .unwrap(),
                metrics_port: node
                    .metrics_addr
                    .split(':')
                    .next_back()
                    .unwrap()
                    .parse()
                    .unwrap(),
            });
        }

        self.save_state(&state)?;

        println!("\n{}", "Cluster started successfully!".green().bold());
        println!("Metrics available at:");
        for node in &state.nodes {
            println!(
                "  - Node {}: http://localhost:{}/metrics",
                node.id, node.metrics_port
            );
        }
        println!("\nLogs available in: {}", self.config.logs_dir().display());

        Ok(())
    }

    pub async fn stop(&self) -> Result<(), ClusterError> {
        let state = self.load_state()?;

        println!("Stopping {} nodes...", state.nodes.len());
        for node in &state.nodes {
            print!("Stopping node {} (PID: {})... ", node.id, node.pid);
            let pid = i32::try_from(node.pid)?;

            let _ = nix::sys::signal::kill(
                nix::unistd::Pid::from_raw(pid),
                nix::sys::signal::Signal::SIGTERM,
            );

            let mut stopped = false;
            for _ in 0..10 {
                if !Self::is_process_running(node.pid)? {
                    stopped = true;
                    break;
                }
                sleep(Duration::from_millis(500)).await;
            }

            if !stopped {
                let _ = nix::sys::signal::kill(
                    nix::unistd::Pid::from_raw(pid),
                    nix::sys::signal::Signal::SIGKILL,
                );
            }

            println!("{}", "✓ Stopped".green());
        }

        // Remove state file
        fs::remove_file(self.config.state_file())?;
        println!("\n{}", "Cluster stopped successfully!".green().bold());

        Ok(())
    }

    pub async fn status(&self) -> Result<(), ClusterError> {
        println!("{}", "Checking cluster status...".yellow().bold());

        if !self.is_running() {
            println!("{}", "No cluster is running".yellow());
            return Ok(());
        }

        let state = self.load_state()?;
        println!("\nCluster started at: {}", state.started_at);
        println!("Nodes:");

        let mut running_count = 0;
        for node in &state.nodes {
            let is_running = Self::is_process_running(node.pid)?;
            if is_running {
                running_count += 1;
            }

            println!(
                "  {} Node {} (PID: {}) - Raft: {}, Metrics: {}",
                if is_running {
                    "✓".green()
                } else {
                    "✗".red()
                },
                node.id,
                node.pid,
                node.raft_port,
                node.metrics_port,
            );
        }

        println!("\n{} of {} nodes running", running_count, state.nodes.len());

        // Check metrics endpoints
        println!("\nChecking metrics endpoints...");
        for node in &state.nodes {
            let url = format!("http://localhost:{}/metrics", node.metrics_port);
            match reqwest::get(&url).await {
                Ok(resp) if resp.status().is_success() => {
                    println!(
                        "  {} Metrics available on port {}",
                        "✓".green(),
                        node.metrics_port
                    );
                }
                _ => {
                    println!(
                        "  {} Metrics not available on port {}",
                        "✗".red(),
                        node.metrics_port
                    );
                }
            }
        }

        Ok(())
    }

    pub async fn show_logs(
        &self,
        node_id: Option<usize>,
        follow: bool,
    ) -> Result<(), ClusterError> {
        let log_files = if let Some(id) = node_id {
            vec![self.config.logs_dir().join(format!("node_{id}.log"))]
        } else {
            // Get all log files
            fs::read_dir(self.config.logs_dir())?
                .filter_map(std::result::Result::ok)
                .map(|entry| entry.path())
                .filter(|path| path.extension().is_some_and(|ext| ext == "log"))
                .collect()
        };

        if log_files.is_empty() {
            println!("{}", "No log files found".yellow());
            return Ok(());
        }

        if follow {
            println!("{}", "Following logs (Ctrl+C to stop)...".yellow());
            // Use tail -f equivalent
            let mut cmd = Command::new("tail");
            cmd.arg("-f");
            for file in &log_files {
                cmd.arg(file);
            }
            cmd.status().await?;
        } else {
            // Just cat the files
            for file in &log_files {
                println!("\n{}", format!("=== {} ===", file.display()).blue().bold());
                let content = fs::read_to_string(file)?;
                println!("{content}");
            }
        }

        Ok(())
    }

    pub async fn test(&self) -> Result<(), ClusterError> {
        if !self.is_running() {
            return Err(ClusterError::NotRunning);
        }

        let state = self.load_state()?;

        // Find a node to test against
        let test_port = state.nodes[0].raft_port;
        let endpoint = format!("127.0.0.1:{test_port}");

        println!("Testing against endpoint: {endpoint}");

        // Test PUT
        print!("Testing PUT operation... ");
        let put_result = Command::new("cargo")
            .args(["run", "--bin", "neuroctl", "--"])
            .args(["--endpoints", &endpoint])
            .args(["put", "test-key", "test-value"])
            .output()
            .await?;

        if put_result.status.success() {
            println!("{}", "✓ Success".green());
        } else {
            println!("{}", "✗ Failed".red());
            println!("Error: {}", String::from_utf8_lossy(&put_result.stderr));
            return Ok(());
        }

        // Test GET
        print!("Testing GET operation... ");
        let get_result = Command::new("cargo")
            .args(["run", "--bin", "neuroctl", "--"])
            .args(["--endpoints", &endpoint])
            .args(["get", "test-key"])
            .output()
            .await?;

        if get_result.status.success()
            && String::from_utf8_lossy(&get_result.stdout).contains("test-value")
        {
            println!("{}", "✓ Success".green());
        } else {
            println!("{}", "✗ Failed".red());
            println!("Error: {}", String::from_utf8_lossy(&get_result.stderr));
        }

        // Test DEL
        print!("Testing DEL operation... ");
        let del_result = Command::new("cargo")
            .args(["run", "--bin", "neuroctl", "--"])
            .args(["--endpoints", &endpoint])
            .args(["del", "test-key"])
            .output()
            .await?;

        if del_result.status.success() {
            println!("{}", "✓ Success".green());
        } else {
            println!("{}", "✗ Failed".red());
            println!("Error: {}", String::from_utf8_lossy(&del_result.stderr));
        }

        println!("\n{}", "Connectivity tests completed!".green().bold());
        Ok(())
    }

    pub async fn clean(&self, clean_logs: bool) -> Result<(), ClusterError> {
        // Stop cluster if running
        if self.is_running() {
            println!("Cluster is running, stopping it first...");
            self.stop().await?;
        }

        // Clean state file
        if self.config.state_file().exists() {
            fs::remove_file(self.config.state_file())?;
            println!("{}", "✓ Removed state file".green());
        }

        // Clean logs if requested
        if clean_logs && self.config.logs_dir().exists() {
            fs::remove_dir_all(self.config.logs_dir())?;
            println!("{}", "✓ Removed log files".green());
        }

        println!("\n{}", "Cleanup completed!".green().bold());
        Ok(())
    }

    fn check_dependencies() -> Result<(), ClusterError> {
        if which::which("cargo").is_err() {
            return Err(ClusterError::DependencyNotFound("cargo".to_string()));
        }

        Ok(())
    }

    fn is_running(&self) -> bool {
        self.config.state_file().exists()
    }

    fn load_state(&self) -> Result<ClusterState, ClusterError> {
        if !self.is_running() {
            return Err(ClusterError::NotRunning);
        }

        let content = fs::read_to_string(self.config.state_file())?;
        Ok(serde_json::from_str(&content)?)
    }

    fn save_state(&self, state: &ClusterState) -> Result<(), ClusterError> {
        let content = serde_json::to_string_pretty(state)?;
        fs::write(self.config.state_file(), content)?;
        Ok(())
    }

    fn is_process_running(pid: u32) -> Result<bool, ClusterError> {
        let pid = i32::try_from(pid)?;
        Ok(nix::sys::signal::kill(nix::unistd::Pid::from_raw(pid), None).is_ok())
    }
}
