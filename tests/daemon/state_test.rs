use std::collections::HashMap;
use blast_core::{
    package::{Package, PackageId, Version},
    environment::EnvironmentState,
    python::PythonVersion,
};
use blast_daemon::state::{StateManager, StateChange, StateValidation};

mod state_changes {
    use super::*;

    #[tokio::test]
    async fn test_state_creation() {
        let manager = StateManager::new();
        let state = manager.get_current_state().await;
        assert!(state.is_ok());
    }

    #[tokio::test]
    async fn test_package_state() {
        let mut manager = StateManager::new();
        
        let package = Package::new(
            PackageId::new(
                "test-package",
                Version::parse("1.0.0").unwrap(),
            ),
            HashMap::new(),
            Default::default(),
        );

        // Add package
        manager.add_package(package.clone()).await.unwrap();
        
        let state = manager.get_current_state().await.unwrap();
        assert!(state.has_package(&package.id));
    }

    #[tokio::test]
    async fn test_environment_state() {
        let mut manager = StateManager::new();
        
        // Create test environment
        let env = EnvironmentState {
            name: "test-env".to_string(),
            python_version: PythonVersion::parse("3.8").unwrap(),
            packages: HashMap::new(),
            variables: HashMap::new(),
            created_at: chrono::Utc::now(),
        };

        manager.add_environment(env.clone()).await.unwrap();
        
        let state = manager.get_current_state().await.unwrap();
        assert!(state.has_environment(&env.name));
    }
}

mod state_validation {
    use super::*;

    #[tokio::test]
    async fn test_state_validation() {
        let mut manager = StateManager::new();
        
        // Add test data
        let package = Package::new(
            PackageId::new(
                "test-package",
                Version::parse("1.0.0").unwrap(),
            ),
            HashMap::new(),
            Default::default(),
        );
        manager.add_package(package).await.unwrap();

        // Validate state
        let validation = manager.validate_state().await.unwrap();
        assert!(validation.is_valid);
        assert!(validation.issues.is_empty());
    }

    #[tokio::test]
    async fn test_invalid_state() {
        let mut manager = StateManager::new();
        
        // Create invalid state by adding conflicting packages
        let pkg1 = Package::new(
            PackageId::new(
                "test-package",
                Version::parse("1.0.0").unwrap(),
            ),
            HashMap::new(),
            Default::default(),
        );
        
        let pkg2 = Package::new(
            PackageId::new(
                "test-package",
                Version::parse("2.0.0").unwrap(),
            ),
            HashMap::new(),
            Default::default(),
        );

        manager.add_package(pkg1).await.unwrap();
        manager.add_package(pkg2).await.unwrap();

        let validation = manager.validate_state().await.unwrap();
        assert!(!validation.is_valid);
        assert!(!validation.issues.is_empty());
    }
}

mod state_persistence {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_state_persistence() {
        let dir = tempdir().unwrap();
        let mut manager = StateManager::with_storage(dir.path());
        
        // Add test data
        let package = Package::new(
            PackageId::new(
                "test-package",
                Version::parse("1.0.0").unwrap(),
            ),
            HashMap::new(),
            Default::default(),
        );
        manager.add_package(package.clone()).await.unwrap();

        // Save state
        manager.save_state().await.unwrap();

        // Create new manager and load state
        let new_manager = StateManager::with_storage(dir.path());
        let loaded_state = new_manager.get_current_state().await.unwrap();
        
        assert!(loaded_state.has_package(&package.id));
    }
}

mod state_transactions {
    use super::*;

    #[tokio::test]
    async fn test_state_transaction() {
        let mut manager = StateManager::new();
        
        // Start transaction
        let tx = manager.begin_transaction().await.unwrap();
        
        // Add package in transaction
        let package = Package::new(
            PackageId::new(
                "test-package",
                Version::parse("1.0.0").unwrap(),
            ),
            HashMap::new(),
            Default::default(),
        );
        
        tx.add_package(package.clone()).await.unwrap();
        
        // Commit transaction
        tx.commit().await.unwrap();
        
        // Verify state
        let state = manager.get_current_state().await.unwrap();
        assert!(state.has_package(&package.id));
    }

    #[tokio::test]
    async fn test_transaction_rollback() {
        let mut manager = StateManager::new();
        
        // Start transaction
        let tx = manager.begin_transaction().await.unwrap();
        
        // Add package in transaction
        let package = Package::new(
            PackageId::new(
                "test-package",
                Version::parse("1.0.0").unwrap(),
            ),
            HashMap::new(),
            Default::default(),
        );
        
        tx.add_package(package.clone()).await.unwrap();
        
        // Rollback transaction
        tx.rollback().await.unwrap();
        
        // Verify state doesn't have package
        let state = manager.get_current_state().await.unwrap();
        assert!(!state.has_package(&package.id));
    }
}
