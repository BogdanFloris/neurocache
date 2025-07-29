use crate::node::NodeState;
use metrics::{counter, describe_counter, describe_gauge, describe_histogram, gauge, histogram};
use std::time::Instant;

pub fn init_metrics() {
    // Raft Consensus Metrics
    describe_gauge!("raft_current_term", "Current term number");
    describe_gauge!(
        "raft_node_state",
        "Current node state (0=Follower, 1=Candidate, 2=Leader)"
    );
    describe_gauge!("raft_commit_index", "Current commit index");
    describe_gauge!("raft_last_applied", "Last applied index");
    describe_gauge!("raft_log_size", "Number of entries in the log");
    describe_gauge!("raft_leader_id", "Current leader node ID");
    describe_gauge!(
        "raft_pending_client_requests",
        "Number of pending client requests"
    );

    // Network Metrics
    describe_counter!("raft_messages_sent_total", "Total messages sent");
    describe_counter!("raft_messages_received_total", "Total messages received");
    describe_histogram!("raft_message_send_duration_seconds", "Message send latency");
    describe_counter!("raft_connection_errors_total", "Connection errors");
    describe_gauge!(
        "raft_active_connections",
        "Number of active peer connections"
    );

    // Performance Metrics
    describe_histogram!(
        "raft_append_entries_duration_seconds",
        "Time to process AppendEntries"
    );
    describe_histogram!(
        "raft_client_request_duration_seconds",
        "Client request processing time"
    );
    describe_histogram!(
        "raft_heartbeat_interval_seconds",
        "Actual heartbeat intervals"
    );
    describe_gauge!("raft_replication_lag", "Per-follower replication lag");

    // State Machine Metrics
    describe_counter!("kvstore_operations_total", "KV operations");
    describe_gauge!("kvstore_size", "Number of keys in the store");
    describe_histogram!("kvstore_operation_duration_seconds", "Operation latency");
}

pub fn record_term(term: u64) {
    gauge!("raft_current_term").set(term as f64);
}

pub fn record_node_state(state: NodeState) {
    let value = match state {
        NodeState::Follower => 0.0,
        NodeState::Candidate => 1.0,
        NodeState::Leader => 2.0,
    };
    gauge!("raft_node_state").set(value);
}

pub fn record_commit_index(index: u64) {
    gauge!("raft_commit_index").set(index as f64);
}

pub fn record_last_applied(index: u64) {
    gauge!("raft_last_applied").set(index as f64);
}

pub fn record_log_size(size: usize) {
    gauge!("raft_log_size").set(size as f64);
}

pub fn record_leader_id(leader_id: Option<u64>) {
    if let Some(id) = leader_id {
        gauge!("raft_leader_id").set(id as f64);
    } else {
        gauge!("raft_leader_id").set(-1.0);
    }
}

pub fn record_pending_requests(count: usize) {
    gauge!("raft_pending_client_requests").set(count as f64);
}

pub fn record_message_sent(message_type: &str, peer_id: u8) {
    counter!("raft_messages_sent_total",
        "message_type" => message_type.to_string(),
        "peer_id" => peer_id.to_string()
    )
    .increment(1);
}

pub fn record_message_received(message_type: &str) {
    counter!("raft_messages_received_total",
        "message_type" => message_type.to_string()
    )
    .increment(1);
}

pub fn record_connection_error(peer_id: u64) {
    counter!("raft_connection_errors_total",
        "peer_id" => peer_id.to_string()
    )
    .increment(1);
}

pub fn record_active_connections(count: usize) {
    gauge!("raft_active_connections").set(count as f64);
}

pub fn record_replication_lag(follower_id: u64, lag: u64) {
    gauge!("raft_replication_lag",
        "follower_id" => follower_id.to_string()
    )
    .set(lag as f64);
}

pub fn record_kvstore_operation(op_type: &str, duration_secs: f64) {
    counter!("kvstore_operations_total",
        "operation" => op_type.to_string()
    )
    .increment(1);

    histogram!("kvstore_operation_duration_seconds",
        "operation" => op_type.to_string()
    )
    .record(duration_secs);
}

pub fn record_kvstore_size(size: usize) {
    gauge!("kvstore_size").set(size as f64);
}

pub struct Timer {
    start: Instant,
    metric_name: &'static str,
    labels: Vec<(&'static str, String)>,
}

impl Timer {
    #[must_use]
    pub fn new(metric_name: &'static str) -> Self {
        Self {
            start: Instant::now(),
            metric_name,
            labels: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_label(mut self, key: &'static str, value: String) -> Self {
        self.labels.push((key, value));
        self
    }

    pub fn observe(self) {
        let duration = self.start.elapsed().as_secs_f64();
        let histogram = histogram!(self.metric_name, &self.labels);
        histogram.record(duration);
    }
}
