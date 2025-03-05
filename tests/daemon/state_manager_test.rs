use std::collections::HashMap;
use std::sync::Arc;
use tempfile::tempdir;
use blast_core::{
    package::Package,
    python::PythonVersion,
};
use blast_image::chrono;
use blast_daemon::{
    state::StateManager,
    metrics::MetricsManager,
};

mod state_operations {
    use super::*;

    #[tokio::test]
    async fn test_state_initialization() {
        let dir = tempdir().unwrap();
        let metrics = Arc::new(MetricsManager::new(100));
        let manager = StateManager::new(dir.path().to_path_buf(), metrics);

        let state = manager.get_current_state().await.unwrap();
        assert_eq!(state.packages.len(), 0);
        assert_eq!(state.name, "default");
        assert_eq!(state.python_version, PythonVersion::parse("3.8.0").unwrap());
    }

    #[tokio::test]
    async fn test_state_update() {
        let dir = tempdir().unwrap();
        let metrics = Arc::new(MetricsManager::new(100));
        let manager = StateManager::new(dir.path().to_path_buf(), metrics);

        // Create test package
        let package = Package::new(
            PackageId::new(
                "test-package",
                Version::parse("1.0.0").unwrap(),
            ),
            HashMap::new(),
            VersionConstraint::parse(">=3.7").unwrap(),
        );

        // Update state with package
        manager.update_state(&[package.clone()]).await.unwrap();

        // Verify state update
        let state = manager.get_current_state().await.unwrap();
        assert_eq!(state.packages.len(), 1);
        assert!(state.packages.contains_key(package.name()));
        assert_eq!(state.packages[package.name()], *package.version());
    }
}

mod snapshots {
    use super::*;

    #[tokio::test]
    async fn test_snapshot_creation() {
        let dir = tempdir().unwrap();
        let metrics = Arc::new(MetricsManager::new(100));
        let manager = StateManager::new(dir.path().to_path_buf(), metrics);

        // Create snapshot
        let description = "Test snapshot".to_string();
        let snapshot_id = manager.create_snapshot(description.clone()).await.unwrap();

        // Verify snapshot
        let snapshot = manager.get_snapshot(snapshot_id).await.unwrap().unwrap();
        assert_eq!(snapshot.description, description);
        assert_eq!(snapshot.state.packages.len(), 0);
    }

    #[tokio::test]
    async fn test_snapshot_restore() {
        let dir = tempdir().unwrap();
        let metrics = Arc::new(MetricsManager::new(100));
        let manager = StateManager::new(dir.path().to_path_buf(), metrics);

        // Create initial snapshot
        let snapshot_id = manager.create_snapshot("Initial state".to_string()).await.unwrap();

        // Add package
        let package = Package::new(
            PackageId::new(
                "test-package",
                Version::parse("1.0.0").unwrap(),
            ),
            HashMap::new(),
            VersionConstraint::parse(">=3.7").unwrap(),
        );
        manager.update_state(&[package]).await.unwrap();

        // Verify package added
        let state = manager.get_current_state().await.unwrap();
        assert_eq!(state.packages.len(), 1);

        // Restore snapshot
        manager.restore_snapshot(snapshot_id).await.unwrap();

        // Verify state restored
        let restored_state = manager.get_current_state().await.unwrap();
        assert_eq!(restored_state.packages.len(), 0);
    }

    #[tokio::test]
    async fn test_snapshot_cleanup() {
        let dir = tempdir().unwrap();
        let metrics = Arc::new(MetricsManager::new(100));
        let manager = StateManager::new(dir.path().to_path_buf(), metrics);

        // Create multiple snapshots
        for i in 0..5 {
            manager.create_snapshot(format!("Snapshot {}", i)).await.unwrap();
        }

        // Clean up snapshots older than 1 hour
        let cleaned = manager.cleanup_snapshots(Duration::hours(1)).await.unwrap();
        assert_eq!(cleaned, 0); // All snapshots are recent

        // Clean up snapshots older than 0 seconds (all snapshots)
        let cleaned = manager.cleanup_snapshots(Duration::seconds(0)).await.unwrap();
        assert_eq!(cleaned, 5);
    }
}

mod verification {
    use super::*;

    #[tokio::test]
    async fn test_state_verification() {
        let dir = tempdir().unwrap();
        let metrics = Arc::new(MetricsManager::new(100));
        let manager = StateManager::new(dir.path().to_path_buf(), metrics);

        // Verify initial state
        let verification = manager.verify_state().await.unwrap();
        assert!(!verification.is_empty()); // Should have warnings about missing directories

        // Create environment structure
        std::fs::create_dir_all(dir.path().join("bin")).unwrap();
        std::fs::create_dir_all(dir.path().join("lib/python3/site-packages")).unwrap();
        std::fs::File::create(dir.path().join("bin/python")).unwrap();

        // Add package
        let package = Package::new(
            PackageId::new(
                "test-package",
                Version::parse("1.0.0").unwrap(),
            ),
            HashMap::new(),
            VersionConstraint::parse(">=3.7").unwrap(),
        );
        manager.update_state(&[package]).await.unwrap();

        // Verify state with package
        let verification = manager.verify_state().await.unwrap();
        assert!(!verification.is_empty()); // Should have warning about missing package installation
    }

    #[tokio::test]
    async fn test_invalid_state() {
        let dir = tempdir().unwrap();
        let metrics = Arc::new(MetricsManager::new(100));
        let manager = StateManager::new(dir.path().to_path_buf(), metrics);

        // Add package without creating environment structure
        let package = Package::new(
            PackageId::new(
                "test-package",
                Version::parse("1.0.0").unwrap(),
            ),
            HashMap::new(),
            VersionConstraint::parse(">=3.7").unwrap(),
        );
        manager.update_state(&[package]).await.unwrap();

        // Verify state
        let verification = manager.verify_state().await.unwrap();
        assert!(!verification.is_empty());
        assert!(verification.has_errors()); // Should have errors about missing environment
    }
}
