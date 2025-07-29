use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::task::JoinSet;

#[derive(Error, Debug)]
pub enum BenchError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("No cluster is running")]
    NotRunning,
}

#[derive(Default)]
struct BenchStats {
    total: AtomicU64,
    successful: AtomicU64,
    failed: AtomicU64,
    read: AtomicU64,
    write: AtomicU64,
}

pub async fn run_benchmark(
    duration: u64,
    threads: usize,
    read_ratio: f64,
) -> Result<(), BenchError> {
    let state_file = std::path::Path::new(".cluster_state.json");
    if !state_file.exists() {
        return Err(BenchError::NotRunning);
    }

    // Load cluster state to get endpoint
    let state_content = std::fs::read_to_string(state_file)?;
    let state: serde_json::Value = serde_json::from_str(&state_content)?;
    let nodes = state["nodes"].as_array().unwrap();
    let first_node = &nodes[0];
    let port = first_node["raft_port"].as_u64().unwrap();
    let endpoint = format!("127.0.0.1:{port}");

    println!("Benchmark configuration:");
    println!("  - Duration: {duration}s");
    println!("  - Threads: {threads}");
    println!("  - Read ratio: {:.0}%", read_ratio * 100.0);
    println!("  - Endpoint: {endpoint}");
    println!();

    let bench_stats = Arc::new(BenchStats::default());
    let start_time = Instant::now();
    let end_time = start_time + Duration::from_secs(duration);

    // Progress bar
    let pb = ProgressBar::new(duration);
    pb.set_style(
        ProgressStyle::default_bar()
            .template(
                "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len}s ({msg})",
            )
            .unwrap()
            .progress_chars("#>-"),
    );

    // Spawn worker tasks
    let mut tasks = JoinSet::new();
    for thread_id in 0..threads {
        let bench_stats = Arc::clone(&bench_stats);
        let endpoint = endpoint.clone();

        tasks.spawn(async move {
            worker_task(thread_id, endpoint, read_ratio, end_time, bench_stats).await;
        });
    }

    // Update progress bar
    let stats_clone = Arc::clone(&bench_stats);
    let progress_task = tokio::spawn(async move {
        while Instant::now() < end_time {
            let elapsed = start_time.elapsed().as_secs();
            let ops_per_sec = stats_clone.total.load(Ordering::Relaxed) / elapsed.max(1);
            pb.set_position(elapsed);
            pb.set_message(format!("{ops_per_sec} ops/s"));
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        pb.finish();
    });

    // Wait for all workers to complete
    while (tasks.join_next().await).is_some() {}
    progress_task.await.unwrap();

    // Print results
    let total_ops = bench_stats.total.load(Ordering::Relaxed);
    let successful_ops = bench_stats.successful.load(Ordering::Relaxed);
    let failed_ops = bench_stats.failed.load(Ordering::Relaxed);
    let read_ops = bench_stats.read.load(Ordering::Relaxed);
    let write_ops = bench_stats.write.load(Ordering::Relaxed);
    let actual_duration = start_time.elapsed().as_secs_f64();

    println!("\n{}", "Benchmark Results".green().bold());
    println!("{}", "=================".green());
    println!("Total operations: {total_ops}");
    println!(
        "Successful: {} ({:.1}%)",
        successful_ops,
        (successful_ops as f64 / total_ops as f64) * 100.0
    );
    println!(
        "Failed: {} ({:.1}%)",
        failed_ops,
        (failed_ops as f64 / total_ops as f64) * 100.0
    );
    println!("Read operations: {read_ops}");
    println!("Write operations: {write_ops}");
    println!(
        "Throughput: {:.0} ops/sec",
        total_ops as f64 / actual_duration
    );
    println!(
        "Average latency: {:.2} ms",
        (actual_duration * 1000.0) / total_ops as f64
    );

    Ok(())
}

async fn worker_task(
    thread_id: usize,
    endpoint: String,
    read_ratio: f64,
    end_time: Instant,
    stats: Arc<BenchStats>,
) {
    use rand::{Rng, SeedableRng};
    let mut rng = rand::rngs::StdRng::from_entropy();

    while Instant::now() < end_time {
        let is_read = rng.gen::<f64>() < read_ratio;
        let key = format!("bench_key_{}", rng.gen_range(0..1000));

        stats.total.fetch_add(1, Ordering::Relaxed);

        let result = if is_read {
            stats.read.fetch_add(1, Ordering::Relaxed);
            // Simulate GET operation
            tokio::process::Command::new("cargo")
                .args(["run", "--bin", "neuroctl", "--"])
                .args(["--endpoints", &endpoint])
                .args(["get", &key])
                .output()
                .await
        } else {
            stats.write.fetch_add(1, Ordering::Relaxed);
            let value = format!("value_{}_{}", thread_id, rng.gen::<u64>());
            // Simulate PUT operation
            tokio::process::Command::new("cargo")
                .args(["run", "--bin", "neuroctl", "--"])
                .args(["--endpoints", &endpoint])
                .args(["put", &key, &value])
                .output()
                .await
        };

        match result {
            Ok(output) if output.status.success() => {
                stats.successful.fetch_add(1, Ordering::Relaxed);
            }
            _ => {
                stats.failed.fetch_add(1, Ordering::Relaxed);
            }
        }
    }
}
