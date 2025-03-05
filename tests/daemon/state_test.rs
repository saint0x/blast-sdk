use std::collections::HashMap;
use std::path::PathBuf;
use blast_core::{
    package::{Package, PackageId},
    python::version::PythonVersion,
};
use blast_daemon::state::{StateManager, StateManagement, State, PackageState, PackageStatus};
use tempfile::tempdir;
use uuid::Uuid;
use chrono::Utc;

mod state_changes {
    use super::*;

    #[tokio::test]
    async fn test_state_creation() {
        let dir = tempdir().unwrap();
        let manager = StateManager::new(dir.path().to_path_buf());
        let state = manager.get_current_state().await;
        assert!(state.is_ok());
    }

    #[tokio::test]
    async fn test_package_state() {
        let dir = tempdir().unwrap();
        let manager = StateManager::new(dir.path().to_path_buf());
        
        let package = Package::new(
            PackageId::new(
                "test-package",
                "1.0.0".to_string(),
            ),
            HashMap::new(),
            Default::default(),
        );

        // Add package to state
        let mut state = State::default();
        state.package_cache.insert(package.id().name().to_string(), PackageState {
            name: package.id().name().to_string(),
            version: package.version().to_string(),
            installed_at: Utc::now(),
            updated_at: Utc::now(),
            status: PackageStatus::Installed,
        });
        manager.update_current_state(state).await.unwrap();
        
        let state = manager.get_current_state().await.unwrap();
        assert!(state.package_cache.contains_key(package.id().name()));
    }

    #[tokio::test]
    async fn test_environment_state() {
        let dir = tempdir().unwrap();
        let manager = StateManager::new(dir.path().to_path_buf());
        
        // Set active environment
        let env_name = "test-env".to_string();
        let env_path = PathBuf::from("/test/path");
        let python_version = PythonVersion::parse("3.8").unwrap();
        
        manager.set_active_environment(env_name.clone(), env_path, python_version).await.unwrap();
        
        let state = manager.get_current_state().await.unwrap();
        assert_eq!(state.active_env_name.unwrap(), env_name);
    }
}

mod state_validation {
    use super::*;

    #[tokio::test]
    async fn test_state_persistence() {
        let dir = tempdir().unwrap();
        let manager = StateManager::new(dir.path().to_path_buf());
        
        // Add test data
        let package = Package::new(
            PackageId::new(
                "test-package",
                "1.0.0".to_string(),
            ),
            HashMap::new(),
            Default::default(),
        );

        let mut state = State::default();
        state.package_cache.insert(package.id().name().to_string(), PackageState {
            name: package.id().name().to_string(),
            version: package.version().to_string(),
            installed_at: Utc::now(),
            updated_at: Utc::now(),
            status: PackageStatus::Installed,
        });
        manager.update_current_state(state).await.unwrap();

        // Save state
        manager.save().await.unwrap();

        // Create new manager and load state
        let new_manager = StateManager::new(dir.path().to_path_buf());
        new_manager.load().await.unwrap();
        let loaded_state = new_manager.get_current_state().await.unwrap();
        
        assert!(loaded_state.package_cache.contains_key(package.id().name()));
    }
}

mod state_checkpoints {
    use super::*;

    #[tokio::test]
    async fn test_checkpoint_creation_and_restore() {
        let dir = tempdir().unwrap();
        let manager = StateManager::new(dir.path().to_path_buf());
        
        // Add initial state
        let mut state = State::default();
        state.active_env_name = Some("test-env".to_string());
        manager.update_current_state(state).await.unwrap();

        // Create checkpoint
        let checkpoint_id = Uuid::new_v4();
        manager.create_checkpoint(checkpoint_id, "Test checkpoint".to_string(), None).await.unwrap();

        // Change state
        let mut new_state = State::default();
        new_state.active_env_name = Some("other-env".to_string());
        manager.update_current_state(new_state).await.unwrap();

        // Restore checkpoint
        manager.restore_checkpoint(&checkpoint_id.to_string()).await.unwrap();
        
        let restored_state = manager.get_current_state().await.unwrap();
        assert_eq!(restored_state.active_env_name.unwrap(), "test-env");
    }
}
