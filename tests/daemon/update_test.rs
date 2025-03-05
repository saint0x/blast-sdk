use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;
use tempfile::tempdir;
use blast_core::{
    package::{Package, PackageId},
    version::Version,
    environment::resources::{ResourceUsage, DiskUsage},
};
use blast_daemon::{
    update::{UpdateManager, UpdateType, UpdateRequest},
    monitor::{MonitorEvent, EnvironmentUsage},
};

mod update_manager {
    use super::*;

    #[tokio::test]
    async fn test_update_manager_creation() {
        let dir = tempdir().unwrap();
        let env_path = dir.path().join("env");
        let cache_path = dir.path().join("cache");
        let (monitor_tx, monitor_rx) = mpsc::channel(100);

        let manager = UpdateManager::new(env_path, cache_path, monitor_rx);
        assert!(manager.metrics().get_operation_count().await == 0);
    }

    #[tokio::test]
    async fn test_file_change_handling() {
        let dir = tempdir().unwrap();
        let env_path = dir.path().join("env");
        let cache_path = dir.path().join("cache");
        let (monitor_tx, monitor_rx) = mpsc::channel(100);

        let mut manager = UpdateManager::new(env_path.clone(), cache_path, monitor_rx);

        // Create test environment structure
        std::fs::create_dir_all(&env_path).unwrap();
        std::fs::create_dir_all(env_path.join("lib/python3/site-packages")).unwrap();

        // Send file change event
        let test_file = env_path.join("lib/python3/site-packages/test.py");
        monitor_tx.send(MonitorEvent::FileChanged(test_file)).await.unwrap();

        // Run manager briefly to process events
        tokio::spawn(async move {
            manager.run().await.unwrap();
        });

        // Wait for processing
        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
    }

    #[tokio::test]
    async fn test_package_change_handling() {
        let dir = tempdir().unwrap();
        let env_path = dir.path().join("env");
        let cache_path = dir.path().join("cache");
        let (monitor_tx, monitor_rx) = mpsc::channel(100);

        let mut manager = UpdateManager::new(env_path.clone(), cache_path.clone(), monitor_rx);

        // Create environment structure
        std::fs::create_dir_all(&env_path).unwrap();
        std::fs::create_dir_all(env_path.join("lib/python3/site-packages")).unwrap();
        std::fs::create_dir_all(env_path.join("bin")).unwrap();
        std::fs::create_dir_all(env_path.join("include")).unwrap();

        // Send package change event
        monitor_tx.send(MonitorEvent::PackageChanged).await.unwrap();

        // Run manager briefly to process events
        tokio::spawn(async move {
            manager.run().await.unwrap();
        });

        // Wait for processing
        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
    }

    #[tokio::test]
    async fn test_resource_update_handling() {
        let dir = tempdir().unwrap();
        let env_path = dir.path().join("env");
        let cache_path = dir.path().join("cache");
        let (monitor_tx, monitor_rx) = mpsc::channel(100);

        let mut manager = UpdateManager::new(env_path, cache_path, monitor_rx);

        // Send resource update event
        let usage = EnvironmentUsage {
            env_disk_usage: DiskUsage {
                total_size: 1024 * 1024, // 1MB
                package_count: 1,
            },
            cache_usage: DiskUsage {
                total_size: 2048 * 1024, // 2MB
                package_count: 2,
            },
            memory_usage: ResourceUsage {
                current: 512 * 1024, // 512KB
                peak: 1024 * 1024,   // 1MB
            },
            cpu_usage: ResourceUsage {
                current: 5.0,
                peak: 10.0,
            },
        };

        monitor_tx.send(MonitorEvent::ResourceUpdate(usage)).await.unwrap();

        // Run manager briefly to process events
        tokio::spawn(async move {
            manager.run().await.unwrap();
        });

        // Wait for processing
        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
    }
}

mod update_requests {
    use super::*;

    #[test]
    fn test_update_request_creation() {
        let package = Package::new(
            PackageId::new(
                "test-package",
                Version::parse("1.0.0").unwrap(),
            ),
            Default::default(),
            Default::default(),
        );

        // Test update request
        let update_req = UpdateRequest::new_update(package.clone(), false, true);
        match update_req.update_type {
            UpdateType::PackageUpdate { package: p, force, update_deps } => {
                assert_eq!(p.name(), "test-package");
                assert_eq!(force, false);
                assert_eq!(update_deps, true);
            },
            _ => panic!("Wrong update type"),
        }

        // Test install request
        let install_req = UpdateRequest::new_install(package.clone());
        match install_req.update_type {
            UpdateType::PackageInstall(p) => {
                assert_eq!(p.name(), "test-package");
            },
            _ => panic!("Wrong update type"),
        }

        // Test remove request
        let remove_req = UpdateRequest::new_remove(package.clone());
        match remove_req.update_type {
            UpdateType::PackageRemove(p) => {
                assert_eq!(p.name(), "test-package");
            },
            _ => panic!("Wrong update type"),
        }

        // Test sync request
        let sync_req = UpdateRequest::new_sync();
        match sync_req.update_type {
            UpdateType::EnvironmentSync => {},
            _ => panic!("Wrong update type"),
        }
    }
}

mod error_handling {
    use super::*;

    #[tokio::test]
    async fn test_invalid_environment_state() {
        let dir = tempdir().unwrap();
        let env_path = dir.path().join("env");
        let cache_path = dir.path().join("cache");
        let (monitor_tx, monitor_rx) = mpsc::channel(100);

        let mut manager = UpdateManager::new(env_path, cache_path, monitor_rx);

        // Send package change event without creating environment structure
        monitor_tx.send(MonitorEvent::PackageChanged).await.unwrap();

        // Run manager briefly to process events
        let handle = tokio::spawn(async move {
            manager.run().await
        });

        // Wait for processing
        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

        // Manager should have errored due to missing environment structure
        handle.abort();
    }

    #[tokio::test]
    async fn test_resource_limit_handling() {
        let dir = tempdir().unwrap();
        let env_path = dir.path().join("env");
        let cache_path = dir.path().join("cache");
        let (monitor_tx, monitor_rx) = mpsc::channel(100);

        let mut manager = UpdateManager::new(env_path.clone(), cache_path.clone(), monitor_rx);

        // Create environment structure
        std::fs::create_dir_all(&env_path).unwrap();
        std::fs::create_dir_all(env_path.join("lib/python3/site-packages")).unwrap();
        std::fs::create_dir_all(env_path.join("bin")).unwrap();
        std::fs::create_dir_all(env_path.join("include")).unwrap();

        // Send resource update with high usage
        let usage = EnvironmentUsage {
            env_disk_usage: DiskUsage {
                total_size: 10 * 1024 * 1024 * 1024, // 10GB
                package_count: 1000,
            },
            cache_usage: DiskUsage {
                total_size: 20 * 1024 * 1024 * 1024, // 20GB
                package_count: 2000,
            },
            memory_usage: ResourceUsage {
                current: 8 * 1024 * 1024 * 1024, // 8GB
                peak: 16 * 1024 * 1024 * 1024,   // 16GB
            },
            cpu_usage: ResourceUsage {
                current: 95.0,
                peak: 100.0,
            },
        };

        monitor_tx.send(MonitorEvent::ResourceUpdate(usage)).await.unwrap();

        // Run manager briefly to process events
        let handle = tokio::spawn(async move {
            manager.run().await
        });

        // Wait for processing
        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

        handle.abort();
    }
}
