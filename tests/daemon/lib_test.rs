use std::collections::HashMap;
use blast_core::{
    package::{Package, PackageId, Version, VersionConstraint},
    security::SecurityPolicy,
};
use blast_daemon::{
    Daemon,
    DaemonConfig,
    transaction::TransactionOperation,
};

#[tokio::test]
async fn test_daemon_lifecycle() {
    let config = DaemonConfig::default();
    let daemon = Daemon::new(config.clone()).await.unwrap();
    
    // Test configuration
    assert_eq!(daemon.config().max_pending_updates, config.max_pending_updates);
    
    // Test pending updates
    assert!(daemon.pending_updates() > 0);
    
    // Test transaction support
    let mut ctx = daemon.begin_transaction().await.unwrap();
    
    // Create test package
    let package = Package::new(
        PackageId::new(
            "test-package",
            Version::parse("1.0.0").unwrap(),
        ),
        HashMap::new(),
        VersionConstraint::parse(">=3.7").unwrap(),
    );
    
    // Add operation to transaction
    ctx.add_operation(TransactionOperation::Install(package.clone())).unwrap();
    
    // Commit transaction
    daemon.commit_transaction(ctx.id).await.unwrap();
    
    // List checkpoints
    let checkpoints = daemon.list_checkpoints().await.unwrap();
    assert!(!checkpoints.is_empty());
    
    // Test shutdown
    daemon.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_checkpoint_restore() {
    let config = DaemonConfig::default();
    let daemon = Daemon::new(config).await.unwrap();
    
    // Create initial state with a package
    let mut ctx = daemon.begin_transaction().await.unwrap();
    let package = Package::new(
        PackageId::new(
            "test-package",
            Version::parse("1.0.0").unwrap(),
        ),
        HashMap::new(),
        VersionConstraint::parse(">=3.7").unwrap(),
    );
    ctx.add_operation(TransactionOperation::Install(package.clone())).unwrap();
    daemon.commit_transaction(ctx.id).await.unwrap();
    
    // Get checkpoints
    let checkpoints = daemon.list_checkpoints().await.unwrap();
    assert!(!checkpoints.is_empty());
    
    // Modify state
    let mut ctx = daemon.begin_transaction().await.unwrap();
    let package_v2 = Package::new(
        PackageId::new(
            "test-package",
            Version::parse("2.0.0").unwrap(),
        ),
        HashMap::new(),
        VersionConstraint::parse(">=3.7").unwrap(),
    );
    ctx.add_operation(TransactionOperation::Update {
        from: package.clone(),
        to: package_v2.clone(),
    }).unwrap();
    daemon.commit_transaction(ctx.id).await.unwrap();
    
    // Restore from first checkpoint
    let first_checkpoint = &checkpoints[0];
    daemon.restore_checkpoint(first_checkpoint.id).await.unwrap();
    
    // Verify state was restored
    let final_checkpoints = daemon.list_checkpoints().await.unwrap();
    let final_state = &final_checkpoints.last().unwrap().state;
    assert_eq!(
        final_state.packages.get("test-package").unwrap().version(),
        package.version()
    );
}

#[tokio::test]
async fn test_environment_management() {
    let config = DaemonConfig::default();
    let daemon = Daemon::new(config).await.unwrap();
    
    // Test environment creation
    let policy = SecurityPolicy::default();
    let env = daemon.create_environment(&policy).await.unwrap();
    
    // Test environment listing
    let environments = daemon.list_environments().await.unwrap();
    assert!(!environments.is_empty());
    
    // Test active environment
    let active_env = daemon.get_active_environment().await.unwrap();
    assert!(active_env.is_some());
    
    // Test environment cleanup
    daemon.clean_environment(&env).await.unwrap();
    
    // Test environment destruction
    daemon.destroy_environment(&env).await.unwrap();
}

#[tokio::test]
async fn test_performance_metrics() {
    let config = DaemonConfig::default();
    let daemon = Daemon::new(config).await.unwrap();
    
    // Get initial performance snapshot
    let metrics = daemon.get_performance_metrics().await.unwrap();
    
    // Verify metric fields
    assert!(metrics.avg_pip_install_time.as_secs() >= 0);
    assert!(metrics.avg_sync_time.as_secs() >= 0);
    assert!(metrics.cache_hit_rate >= 0.0 && metrics.cache_hit_rate <= 1.0);
    assert!(metrics.timestamp <= std::time::Instant::now());
}

#[tokio::test]
async fn test_transaction_management() {
    let config = DaemonConfig::default();
    let daemon = Daemon::new(config).await.unwrap();
    
    // Begin transaction
    let ctx = daemon.begin_transaction().await.unwrap();
    
    // Get transaction
    let retrieved = daemon.get_transaction(ctx.id).await.unwrap();
    assert!(retrieved.is_some());
    
    // Rollback transaction
    daemon.rollback_transaction(ctx.id).await.unwrap();
    
    // Verify transaction is rolled back
    let final_state = daemon.get_transaction(ctx.id).await.unwrap().unwrap();
    assert!(matches!(final_state.status, blast_daemon::transaction::TransactionStatus::RolledBack));
}
