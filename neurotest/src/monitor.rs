use colored::Colorize;
use thiserror::Error;
use tokio::process::Command;

#[derive(Error, Debug)]
pub enum MonitorError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Docker not found. Please install Docker to use monitoring features")]
    DockerNotFound,
    #[error("docker-compose.yml not found")]
    ComposeFileNotFound,
}

pub async fn start() -> Result<(), MonitorError> {
    // Check if docker is available
    if which::which("docker").is_err() {
        return Err(MonitorError::DockerNotFound);
    }

    // Check if docker-compose.yml exists
    if !std::path::Path::new("docker-compose.yml").exists() {
        return Err(MonitorError::ComposeFileNotFound);
    }

    println!("Starting monitoring stack (Prometheus + Grafana)...");

    let output = Command::new("docker")
        .args(["compose", "up", "-d"])
        .output()
        .await?;

    if !output.status.success() {
        println!("{}", "Failed to start monitoring stack".red());
        println!("Error: {}", String::from_utf8_lossy(&output.stderr));
        return Ok(());
    }

    println!("{}", "✓ Monitoring stack started".green());
    println!("\nAccess points:");
    println!("  - Grafana: http://localhost:3000 (admin/admin)");
    println!("  - Prometheus: http://localhost:9090");

    Ok(())
}

pub async fn stop() -> Result<(), MonitorError> {
    // Check if docker is available
    if which::which("docker").is_err() {
        return Err(MonitorError::DockerNotFound);
    }

    println!("Stopping monitoring stack...");

    let output = Command::new("docker")
        .args(["compose", "down"])
        .output()
        .await?;

    if !output.status.success() {
        println!("{}", "Failed to stop monitoring stack".red());
        println!("Error: {}", String::from_utf8_lossy(&output.stderr));
        return Ok(());
    }

    println!("{}", "✓ Monitoring stack stopped".green());
    Ok(())
}

pub async fn status() -> Result<(), MonitorError> {
    // Check if docker is available
    if which::which("docker").is_err() {
        return Err(MonitorError::DockerNotFound);
    }

    println!("{}", "Checking monitoring stack status...".yellow());

    // Check docker compose status
    let output = Command::new("docker")
        .args(["compose", "ps", "--format", "json"])
        .output()
        .await?;

    if !output.status.success() {
        println!("{}", "Failed to check monitoring status".red());
        return Ok(());
    }

    // Parse docker compose output
    let output_str = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = output_str.lines().collect();

    let mut prometheus_running = false;
    let mut grafana_running = false;

    for line in lines {
        if line.is_empty() {
            continue;
        }

        if let Ok(container) = serde_json::from_str::<serde_json::Value>(line) {
            let name = container["Name"].as_str().unwrap_or("");
            let state = container["State"].as_str().unwrap_or("");

            if name.contains("prometheus") && state == "running" {
                prometheus_running = true;
            }
            if name.contains("grafana") && state == "running" {
                grafana_running = true;
            }
        }
    }

    println!("\nMonitoring services:");
    println!(
        "  {} Prometheus: {}",
        if prometheus_running {
            "✓".green()
        } else {
            "✗".red()
        },
        if prometheus_running {
            "Running at http://localhost:9090".green()
        } else {
            "Not running".red()
        }
    );
    println!(
        "  {} Grafana: {}",
        if grafana_running {
            "✓".green()
        } else {
            "✗".red()
        },
        if grafana_running {
            "Running at http://localhost:3000".green()
        } else {
            "Not running".red()
        }
    );

    // Check if services are accessible
    if prometheus_running {
        match reqwest::get("http://localhost:9090/api/v1/query?query=up").await {
            Ok(resp) if resp.status().is_success() => {
                println!("\n  {} Prometheus API is accessible", "✓".green());
            }
            _ => {
                println!("\n  {} Prometheus API is not accessible", "✗".red());
            }
        }
    }

    if grafana_running {
        match reqwest::get("http://localhost:3000/api/health").await {
            Ok(resp) if resp.status().is_success() => {
                println!("  {} Grafana API is accessible", "✓".green());
            }
            _ => {
                println!("  {} Grafana API is not accessible", "✗".red());
            }
        }
    }

    Ok(())
}
