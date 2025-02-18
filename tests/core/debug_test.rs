use std::collections::HashMap;
use std::time::Duration;
use chrono::Utc;
use blast_core::debug::{
    DebugCollector, OperationRecord, OperationStatus,
    EnvironmentState, PackageInfo, DependencyGraph
};

#[test]
fn test_debug_collector() {
    let mut collector = DebugCollector::new();

    // Test operation recording
    let operation = OperationRecord {
        id: uuid::Uuid::new_v4(),
        operation_type: "install".to_string(),
        timestamp: Utc::now(),
        duration: Duration::from_secs(1),
        status: OperationStatus::Success,
        error: None,
        stack_trace: None,
    };
    collector.record_operation(operation);

    // Test environment state capture
    let state = EnvironmentState {
        packages: vec![
            PackageInfo {
                name: "requests".to_string(),
                version: "2.28.2".to_string(),
                installed_at: Utc::now(),
                dependencies: vec!["urllib3".to_string()],
                is_direct: true,
            }
        ],
        python_version: "3.9.0".to_string(),
        env_vars: HashMap::new(),
        venv_path: std::path::PathBuf::from("/tmp/venv"),
        last_modified: Utc::now(),
    };
    collector.capture_environment(state);

    // Generate and verify report
    let report = collector.generate_report();
    assert_eq!(report.operation_history.len(), 1);
    assert!(report.environment_state.is_some());
    assert!(report.dependency_graph.is_some());
}

#[test]
fn test_dependency_graph() {
    let mut graph = DependencyGraph::new();
    
    // Add some dependencies
    graph.add_dependency("requests", "urllib3");
    graph.add_dependency("requests", "certifi");
    graph.add_dependency("urllib3", "certifi");

    // Verify DOT output
    let dot = graph.to_dot();
    assert!(dot.contains("requests"));
    assert!(dot.contains("urllib3"));
    assert!(dot.contains("certifi"));
} 