//! Telemetry and metrics collection abstraction
//!
//! Provides pluggable metrics collection for integration with monitoring systems
//! like Prometheus, DataDog, or custom dashboards.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Build event types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildEvent {
    /// Build started
    BuildStarted,
    /// Build completed successfully
    BuildCompleted,
    /// Build failed
    BuildFailed,
    /// Optimization started
    OptimizationStarted,
    /// Optimization completed
    OptimizationCompleted,
}

/// Metric data point
#[derive(Debug, Clone)]
pub struct MetricData {
    /// Metric name
    pub name: String,
    /// Metric value
    pub value: f64,
    /// Tags for categorization
    pub tags: HashMap<String, String>,
    /// Timestamp (if provided)
    pub timestamp: Option<std::time::SystemTime>,
}

impl MetricData {
    /// Create a new metric
    pub fn new(name: impl Into<String>, value: f64) -> Self {
        Self {
            name: name.into(),
            value,
            tags: HashMap::new(),
            timestamp: Some(std::time::SystemTime::now()),
        }
    }

    /// Add a tag
    pub fn with_tag(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.tags.insert(key.into(), value.into());
        self
    }
}

/// Trait for pluggable metrics collection
pub trait MetricsCollector: Send + Sync {
    /// Collector name
    fn name(&self) -> &str;

    /// Record a build event
    fn record_event(&self, event: BuildEvent, metadata: HashMap<String, String>);

    /// Record a metric value
    fn record_metric(&self, metric: MetricData);

    /// Record build duration
    fn record_duration(&self, stage: &str, duration: Duration) {
        let metric = MetricData::new(
            format!("{}_duration_ms", stage),
            duration.as_millis() as f64,
        )
        .with_tag("stage", stage);
        self.record_metric(metric);
    }

    /// Record size metric
    fn record_size(&self, label: &str, size_bytes: u64) {
        let metric = MetricData::new(format!("{}_size_bytes", label), size_bytes as f64)
            .with_tag("label", label);
        self.record_metric(metric);
    }

    /// Flush any buffered metrics
    fn flush(&self) {
        // Default: do nothing
    }
}

/// No-op collector (default)
pub struct NoOpCollector;

impl MetricsCollector for NoOpCollector {
    fn name(&self) -> &str {
        "noop"
    }

    fn record_event(&self, _event: BuildEvent, _metadata: HashMap<String, String>) {
        // Do nothing
    }

    fn record_metric(&self, _metric: MetricData) {
        // Do nothing
    }
}

/// Stdout collector for debugging
pub struct StdoutCollector;

impl MetricsCollector for StdoutCollector {
    fn name(&self) -> &str {
        "stdout"
    }

    fn record_event(&self, event: BuildEvent, metadata: HashMap<String, String>) {
        println!("[METRIC] Event: {:?}", event);
        if !metadata.is_empty() {
            println!("  Metadata: {:?}", metadata);
        }
    }

    fn record_metric(&self, metric: MetricData) {
        print!("[METRIC] {}: {}", metric.name, metric.value);
        if !metric.tags.is_empty() {
            print!(" (tags: {:?})", metric.tags);
        }
        println!();
    }
}

/// Event with metadata
type EventRecord = (BuildEvent, HashMap<String, String>);

/// In-memory collector for testing
#[derive(Default)]
pub struct MemoryCollector {
    events: Arc<Mutex<Vec<EventRecord>>>,
    metrics: Arc<Mutex<Vec<MetricData>>>,
}

impl MemoryCollector {
    /// Create a new memory collector
    pub fn new() -> Self {
        Self::default()
    }

    /// Get all recorded events
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex has been poisoned by a panic in another thread
    /// while holding the lock. This is extremely rare in practice.
    pub fn events(&self) -> Vec<EventRecord> {
        self.events
            .lock()
            .expect("Memory collector lock poisoned")
            .clone()
    }

    /// Get all recorded metrics
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex has been poisoned by a panic in another thread
    /// while holding the lock. This is extremely rare in practice.
    pub fn metrics(&self) -> Vec<MetricData> {
        self.metrics
            .lock()
            .expect("Memory collector lock poisoned")
            .clone()
    }

    /// Clear all recorded data
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex has been poisoned by a panic in another thread
    /// while holding the lock. This is extremely rare in practice.
    pub fn clear(&self) {
        self.events
            .lock()
            .expect("Memory collector lock poisoned")
            .clear();
        self.metrics
            .lock()
            .expect("Memory collector lock poisoned")
            .clear();
    }
}

impl MetricsCollector for MemoryCollector {
    fn name(&self) -> &str {
        "memory"
    }

    fn record_event(&self, event: BuildEvent, metadata: HashMap<String, String>) {
        self.events
            .lock()
            .expect("Memory collector lock poisoned")
            .push((event, metadata));
    }

    fn record_metric(&self, metric: MetricData) {
        self.metrics
            .lock()
            .expect("Memory collector lock poisoned")
            .push(metric);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metric_data_new_creates_with_timestamp() {
        let metric = MetricData::new("test_metric", 42.0);
        assert_eq!(metric.name, "test_metric");
        assert_eq!(metric.value, 42.0);
        assert!(metric.timestamp.is_some());
        assert!(metric.tags.is_empty());
    }

    #[test]
    fn test_metric_data_with_tag_adds_tags() {
        let metric = MetricData::new("test", 10.0)
            .with_tag("env", "production")
            .with_tag("region", "us-west");

        assert_eq!(metric.tags.len(), 2);
        assert_eq!(metric.tags.get("env"), Some(&"production".to_string()));
        assert_eq!(metric.tags.get("region"), Some(&"us-west".to_string()));
    }

    #[test]
    fn test_noop_collector_does_nothing() {
        let collector = NoOpCollector;
        assert_eq!(collector.name(), "noop");

        // Should not panic or error
        collector.record_event(BuildEvent::BuildStarted, HashMap::new());
        collector.record_metric(MetricData::new("test", 1.0));
        collector.record_duration("build", Duration::from_secs(1));
        collector.record_size("wasm", 1024);
        collector.flush();
    }

    #[test]
    fn test_memory_collector_records_events() {
        let collector = MemoryCollector::new();
        assert_eq!(collector.name(), "memory");

        let mut metadata = HashMap::new();
        metadata.insert("profile".to_string(), "release".to_string());

        collector.record_event(BuildEvent::BuildStarted, metadata.clone());
        collector.record_event(BuildEvent::BuildCompleted, HashMap::new());

        let events = collector.events();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].0, BuildEvent::BuildStarted);
        assert_eq!(events[0].1.get("profile"), Some(&"release".to_string()));
        assert_eq!(events[1].0, BuildEvent::BuildCompleted);
    }

    #[test]
    fn test_memory_collector_records_metrics() {
        let collector = MemoryCollector::new();

        collector.record_metric(MetricData::new("size", 1024.0));
        collector.record_metric(MetricData::new("duration", 500.0).with_tag("stage", "build"));

        let metrics = collector.metrics();
        assert_eq!(metrics.len(), 2);
        assert_eq!(metrics[0].name, "size");
        assert_eq!(metrics[0].value, 1024.0);
        assert_eq!(metrics[1].name, "duration");
        assert_eq!(metrics[1].value, 500.0);
        assert_eq!(metrics[1].tags.get("stage"), Some(&"build".to_string()));
    }

    #[test]
    fn test_memory_collector_clear_removes_all_data() {
        let collector = MemoryCollector::new();

        collector.record_event(BuildEvent::BuildStarted, HashMap::new());
        collector.record_metric(MetricData::new("test", 1.0));

        assert_eq!(collector.events().len(), 1);
        assert_eq!(collector.metrics().len(), 1);

        collector.clear();

        assert_eq!(collector.events().len(), 0);
        assert_eq!(collector.metrics().len(), 0);
    }

    #[test]
    fn test_record_duration_creates_correct_metric() {
        let collector = MemoryCollector::new();

        collector.record_duration("optimization", Duration::from_millis(1500));

        let metrics = collector.metrics();
        assert_eq!(metrics.len(), 1);
        assert_eq!(metrics[0].name, "optimization_duration_ms");
        assert_eq!(metrics[0].value, 1500.0);
        assert_eq!(
            metrics[0].tags.get("stage"),
            Some(&"optimization".to_string())
        );
    }

    #[test]
    fn test_record_size_creates_correct_metric() {
        let collector = MemoryCollector::new();

        collector.record_size("wasm_output", 524288);

        let metrics = collector.metrics();
        assert_eq!(metrics.len(), 1);
        assert_eq!(metrics[0].name, "wasm_output_size_bytes");
        assert_eq!(metrics[0].value, 524288.0);
        assert_eq!(
            metrics[0].tags.get("label"),
            Some(&"wasm_output".to_string())
        );
    }

    #[test]
    fn test_memory_collector_handles_concurrent_events() {
        use std::thread;

        let collector = Arc::new(MemoryCollector::new());
        let mut handles = vec![];

        for i in 0..10 {
            let collector_clone = Arc::clone(&collector);
            let handle = thread::spawn(move || {
                let mut metadata = HashMap::new();
                metadata.insert("thread".to_string(), i.to_string());
                collector_clone.record_event(BuildEvent::BuildStarted, metadata);
                collector_clone.record_metric(MetricData::new(format!("metric_{}", i), i as f64));
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(collector.events().len(), 10);
        assert_eq!(collector.metrics().len(), 10);
    }

    #[test]
    fn test_telemetry_with_invalid_data_handles_gracefully() {
        let collector = MemoryCollector::new();

        // Test with extreme values
        collector.record_metric(MetricData::new("large", f64::MAX));
        collector.record_metric(MetricData::new("zero", 0.0));
        collector.record_metric(MetricData::new("negative", -100.0));

        let metrics = collector.metrics();
        assert_eq!(metrics.len(), 3);
        assert_eq!(metrics[0].value, f64::MAX);
        assert_eq!(metrics[1].value, 0.0);
        assert_eq!(metrics[2].value, -100.0);
    }

    #[test]
    fn test_build_event_equality() {
        assert_eq!(BuildEvent::BuildStarted, BuildEvent::BuildStarted);
        assert_ne!(BuildEvent::BuildStarted, BuildEvent::BuildCompleted);
        assert_ne!(BuildEvent::BuildCompleted, BuildEvent::BuildFailed);
    }

    #[test]
    fn test_stdout_collector_does_not_panic() {
        let collector = StdoutCollector;
        assert_eq!(collector.name(), "stdout");

        // These should print but not panic
        collector.record_event(BuildEvent::OptimizationStarted, HashMap::new());
        collector.record_metric(MetricData::new("test", 42.0));
        collector.flush();
    }
}
