use std::collections::HashMap;
use blast_core::{
    package::{Package, PackageId, Version, VersionConstraint},
    python::PythonVersion,
    state::{EnvironmentState, StateVerification},
};
use blast_daemon::transaction::{
    TransactionManager,
    TransactionOperation,
    TransactionContext,
    TransactionStatus,
    TransactionMetrics,
};

mod transaction_lifecycle {
    use super::*;

    #[tokio::test]
    async fn test_transaction_creation() {
        let initial_state = EnvironmentState::new(
            "test".to_string(),
            PythonVersion::parse("3.8").unwrap(),
            HashMap::new(),
            HashMap::new(),
        );

        let manager = TransactionManager::new(initial_state);
        
        // Begin transaction
        let ctx = manager.begin_transaction().await.unwrap();
        
        assert!(matches!(ctx.status, TransactionStatus::Pending));
        assert!(ctx.operations.is_empty());
        assert!(ctx.state_before.is_empty());
    }

    #[tokio::test]
    async fn test_transaction_commit() {
        let initial_state = EnvironmentState::new(
            "test".to_string(),
            PythonVersion::parse("3.8").unwrap(),
            HashMap::new(),
            HashMap::new(),
        );

        let manager = TransactionManager::new(initial_state);
        
        // Begin transaction
        let mut ctx = manager.begin_transaction().await.unwrap();
        
        // Add operations
        let package = Package::new(
            PackageId::new(
                "test-package",
                Version::parse("1.0.0").unwrap(),
            ),
            HashMap::new(),
            VersionConstraint::any(),
        );
        
        ctx.add_operation(TransactionOperation::Install(package.clone())).unwrap();
        
        // Commit transaction
        manager.commit_transaction(ctx.id).await.unwrap();
        
        // Verify state
        let state = manager.get_current_state().await.unwrap();
        assert!(state.packages.contains_key("test-package"));
        
        // Verify transaction status
        let txn = manager.get_transaction(ctx.id).await.unwrap().unwrap();
        assert!(matches!(txn.status, TransactionStatus::Committed));
    }

    #[tokio::test]
    async fn test_transaction_rollback() {
        let initial_state = EnvironmentState::new(
            "test".to_string(),
            PythonVersion::parse("3.8").unwrap(),
            HashMap::new(),
            HashMap::new(),
        );

        let manager = TransactionManager::new(initial_state);
        
        // Begin transaction
        let mut ctx = manager.begin_transaction().await.unwrap();
        
        // Add operations
        let package = Package::new(
            PackageId::new(
                "test-package",
                Version::parse("1.0.0").unwrap(),
            ),
            HashMap::new(),
            VersionConstraint::any(),
        );
        
        ctx.add_operation(TransactionOperation::Install(package.clone())).unwrap();
        
        // Rollback transaction
        manager.rollback_transaction(ctx.id).await.unwrap();
        
        // Verify state
        let state = manager.get_current_state().await.unwrap();
        assert!(!state.packages.contains_key("test-package"));
        
        // Verify transaction status
        let txn = manager.get_transaction(ctx.id).await.unwrap().unwrap();
        assert!(matches!(txn.status, TransactionStatus::RolledBack));
    }
}

mod transaction_operations {
    use super::*;

    #[tokio::test]
    async fn test_package_install() {
        let initial_state = EnvironmentState::new(
            "test".to_string(),
            PythonVersion::parse("3.8").unwrap(),
            HashMap::new(),
            HashMap::new(),
        );

        let manager = TransactionManager::new(initial_state);
        let mut ctx = manager.begin_transaction().await.unwrap();
        
        // Install package
        let package = Package::new(
            PackageId::new(
                "test-package",
                Version::parse("1.0.0").unwrap(),
            ),
            HashMap::new(),
            VersionConstraint::any(),
        );
        
        ctx.add_operation(TransactionOperation::Install(package.clone())).unwrap();
        manager.commit_transaction(ctx.id).await.unwrap();
        
        let state = manager.get_current_state().await.unwrap();
        assert_eq!(state.packages[&package.name().to_string()], *package.version());
    }

    #[tokio::test]
    async fn test_package_uninstall() {
        let mut initial_state = EnvironmentState::new(
            "test".to_string(),
            PythonVersion::parse("3.8").unwrap(),
            HashMap::new(),
            HashMap::new(),
        );

        // Add package to initial state
        let package = Package::new(
            PackageId::new(
                "test-package",
                Version::parse("1.0.0").unwrap(),
            ),
            HashMap::new(),
            VersionConstraint::any(),
        );
        initial_state.packages.insert(package.name().to_string(), package.version().clone());

        let manager = TransactionManager::new(initial_state);
        let mut ctx = manager.begin_transaction().await.unwrap();
        
        // Uninstall package
        ctx.add_operation(TransactionOperation::Uninstall(package.clone())).unwrap();
        manager.commit_transaction(ctx.id).await.unwrap();
        
        let state = manager.get_current_state().await.unwrap();
        assert!(!state.packages.contains_key(package.name()));
    }

    #[tokio::test]
    async fn test_package_update() {
        let mut initial_state = EnvironmentState::new(
            "test".to_string(),
            PythonVersion::parse("3.8").unwrap(),
            HashMap::new(),
            HashMap::new(),
        );

        // Add old version to initial state
        let old_package = Package::new(
            PackageId::new(
                "test-package",
                Version::parse("1.0.0").unwrap(),
            ),
            HashMap::new(),
            VersionConstraint::any(),
        );
        initial_state.packages.insert(old_package.name().to_string(), old_package.version().clone());

        let manager = TransactionManager::new(initial_state);
        let mut ctx = manager.begin_transaction().await.unwrap();
        
        // Update to new version
        let new_package = Package::new(
            PackageId::new(
                "test-package",
                Version::parse("2.0.0").unwrap(),
            ),
            HashMap::new(),
            VersionConstraint::any(),
        );
        
        ctx.add_operation(TransactionOperation::Update {
            from: old_package.clone(),
            to: new_package.clone(),
        }).unwrap();
        
        manager.commit_transaction(ctx.id).await.unwrap();
        
        let state = manager.get_current_state().await.unwrap();
        assert_eq!(state.packages[&new_package.name().to_string()], *new_package.version());
    }
}

mod transaction_metrics {
    use super::*;

    #[tokio::test]
    async fn test_metrics_collection() {
        let initial_state = EnvironmentState::new(
            "test".to_string(),
            PythonVersion::parse("3.8").unwrap(),
            HashMap::new(),
            HashMap::new(),
        );

        let manager = TransactionManager::new(initial_state);
        let mut ctx = manager.begin_transaction().await.unwrap();
        
        // Add operation
        let package = Package::new(
            PackageId::new(
                "test-package",
                Version::parse("1.0.0").unwrap(),
            ),
            HashMap::new(),
            VersionConstraint::any(),
        );
        
        ctx.add_operation(TransactionOperation::Install(package.clone())).unwrap();
        
        // Update metrics
        let metrics = TransactionMetrics {
            duration: Some(std::time::Duration::from_secs(1)),
            memory_usage: 1024 * 1024, // 1MB
            cpu_usage: 50.0,
            network_operations: 2,
            cache_hits: 1,
            dependencies_checked: 5,
        };
        ctx.update_metrics(metrics.clone());
        
        // Commit and verify metrics
        manager.commit_transaction(ctx.id).await.unwrap();
        
        let txn = manager.get_transaction(ctx.id).await.unwrap().unwrap();
        assert_eq!(txn.metrics.memory_usage, metrics.memory_usage);
        assert_eq!(txn.metrics.cpu_usage, metrics.cpu_usage);
        assert_eq!(txn.metrics.network_operations, metrics.network_operations);
        assert_eq!(txn.metrics.cache_hits, metrics.cache_hits);
        assert_eq!(txn.metrics.dependencies_checked, metrics.dependencies_checked);
    }
}

mod error_handling {
    use super::*;

    #[tokio::test]
    async fn test_invalid_transaction_id() {
        let initial_state = EnvironmentState::new(
            "test".to_string(),
            PythonVersion::parse("3.8").unwrap(),
            HashMap::new(),
            HashMap::new(),
        );

        let manager = TransactionManager::new(initial_state);
        
        // Try to commit non-existent transaction
        let result = manager.commit_transaction(uuid::Uuid::new_v4()).await;
        assert!(result.is_err());
        
        // Try to rollback non-existent transaction
        let result = manager.rollback_transaction(uuid::Uuid::new_v4()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_double_commit() {
        let initial_state = EnvironmentState::new(
            "test".to_string(),
            PythonVersion::parse("3.8").unwrap(),
            HashMap::new(),
            HashMap::new(),
        );

        let manager = TransactionManager::new(initial_state);
        let ctx = manager.begin_transaction().await.unwrap();
        
        // First commit should succeed
        assert!(manager.commit_transaction(ctx.id).await.is_ok());
        
        // Second commit should fail
        assert!(manager.commit_transaction(ctx.id).await.is_err());
    }

    #[tokio::test]
    async fn test_commit_after_rollback() {
        let initial_state = EnvironmentState::new(
            "test".to_string(),
            PythonVersion::parse("3.8").unwrap(),
            HashMap::new(),
            HashMap::new(),
        );

        let manager = TransactionManager::new(initial_state);
        let ctx = manager.begin_transaction().await.unwrap();
        
        // Rollback should succeed
        assert!(manager.rollback_transaction(ctx.id).await.is_ok());
        
        // Commit after rollback should fail
        assert!(manager.commit_transaction(ctx.id).await.is_err());
    }
}
