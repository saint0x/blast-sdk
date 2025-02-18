use std::time::{Duration, Instant};
use uuid::Uuid;
use blast_daemon::metrics::{MetricsManager, PackageMetrics, EnvironmentMetrics};

#[tokio::test]
async fn test_metrics_manager() {
    let manager = MetricsManager::new(100);

    // Record package metrics
    let op_id = Uuid::new_v4();
    manager.record_package_install(
        op_id,
        "test-package".to_string(),
        "1.0.0".to_string(),
        Duration::from_millis(500),
        Duration::from_millis(200),
        5,
        3,
        1024 * 1024,
    ).await;

    // Update environment metrics
    manager.update_environment_metrics(
        "test-env".to_string(),
        10,
        1024 * 1024 * 100,
        1024 * 1024 * 50,
        Duration::from_millis(300),
    ).await;

    // Check averages
    let (avg_pip, avg_sync) = manager.get_average_install_times().await;
    assert!(avg_pip > Duration::from_millis(0));
    assert!(avg_sync > Duration::from_millis(0));

    // Check cache hit rate
    let hit_rate = manager.get_cache_hit_rate().await;
    assert!(hit_rate > 0.0);
}

#[tokio::test]
async fn test_package_metrics_retrieval() {
    let manager = MetricsManager::new(100);
    let op_id = Uuid::new_v4();

    // Record metrics
    manager.record_package_install(
        op_id,
        "numpy".to_string(),
        "1.21.0".to_string(),
        Duration::from_millis(1000),
        Duration::from_millis(500),
        10,
        7,
        2048 * 1024,
    ).await;

    // Retrieve and verify metrics
    let metrics = manager.get_package_metrics(&op_id).await.unwrap();
    assert_eq!(metrics.package_name, "numpy");
    assert_eq!(metrics.package_version, "1.21.0");
    assert_eq!(metrics.pip_install_duration, Duration::from_millis(1000));
    assert_eq!(metrics.sync_duration, Duration::from_millis(500));
    assert_eq!(metrics.dependency_count, 10);
    assert!(metrics.cache_hit_rate > 0.0);
    assert_eq!(metrics.memory_usage, 2048 * 1024);
}

#[tokio::test]
async fn test_environment_metrics_retrieval() {
    let manager = MetricsManager::new(100);
    let env_name = "test-environment";

    // Update metrics
    manager.update_environment_metrics(
        env_name.to_string(),
        20,
        1024 * 1024 * 200,
        1024 * 1024 * 100,
        Duration::from_millis(800),
    ).await;

    // Retrieve and verify metrics
    let metrics = manager.get_environment_metrics(env_name).await.unwrap();
    assert_eq!(metrics.total_packages, 20);
    assert_eq!(metrics.env_size, 1024 * 1024 * 200);
    assert_eq!(metrics.cache_size, 1024 * 1024 * 100);
    assert_eq!(metrics.avg_sync_duration, Duration::from_millis(800));
}

#[tokio::test]
async fn test_metrics_window() {
    let manager = MetricsManager::new(3); // Small window for testing

    // Record multiple package installations
    for i in 0..5 {
        manager.record_package_install(
            Uuid::new_v4(),
            format!("package-{}", i),
            "1.0.0".to_string(),
            Duration::from_millis(100 * (i + 1)),
            Duration::from_millis(50 * (i + 1)),
            i + 1,
            i,
            1024 * 1024,
        ).await;
    }

    // Check that averages only consider the window
    let (avg_pip, avg_sync) = manager.get_average_install_times().await;
    
    // Should only consider last 3 operations
    assert!(avg_pip <= Duration::from_millis(400));
    assert!(avg_sync <= Duration::from_millis(200));
}

#[tokio::test]
async fn test_cache_hit_rate_calculation() {
    let manager = MetricsManager::new(100);

    // Record operations with different cache hit rates
    manager.record_package_install(
        Uuid::new_v4(),
        "pkg1".to_string(),
        "1.0.0".to_string(),
        Duration::from_millis(100),
        Duration::from_millis(50),
        10,
        5, // 50% hit rate
        1024 * 1024,
    ).await;

    manager.record_package_install(
        Uuid::new_v4(),
        "pkg2".to_string(),
        "1.0.0".to_string(),
        Duration::from_millis(100),
        Duration::from_millis(50),
        10,
        8, // 80% hit rate
        1024 * 1024,
    ).await;

    // Average hit rate should be 65%
    let hit_rate = manager.get_cache_hit_rate().await;
    assert!(hit_rate > 0.60 && hit_rate < 0.70);
}
