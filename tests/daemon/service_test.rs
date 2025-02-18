use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;
use blast_core::{
    package::{Package, PackageId, Version},
    python::PythonVersion,
};
use blast_daemon::{
    DaemonService,
    update::{UpdateType, UpdateRequest},
    monitor::{MonitorEvent, EnvironmentUsage},
};

mod update_service {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_update_service_creation() {
        let temp_dir = tempdir().unwrap();
        let env_path = temp_dir.path().join("env");
        let cache_path = temp_dir.path().join("cache");

        let (monitor_tx, _) = mpsc::channel(100);
        let mut service = DaemonService::new(monitor_tx);
        
        assert!(service.start().await.is_ok());
        assert!(service.stop().await.is_ok());
    }

    #[tokio::test]
    async fn test_package_update() {
        let temp_dir = tempdir().unwrap();
        let env_path = temp_dir.path().join("env");
        let cache_path = temp_dir.path().join("cache");

        let (monitor_tx, _) = mpsc::channel(100);
        let mut service = DaemonService::new(monitor_tx);
        
        assert!(service.start().await.is_ok());

        // Create test package
        let package = Package::new(
            PackageId::new(
                "test-package",
                Version::parse("1.0.0").unwrap(),
            ),
            Default::default(),
            Default::default(),
        );

        // Send update request
        let request = UpdateRequest {
            update_type: UpdateType::PackageUpdate {
                package: package.clone(),
                force: false,
                update_deps: true,
            },
            priority: 0,
        };

        assert!(service.send_update_request(request).await.is_ok());
        assert!(service.stop().await.is_ok());
    }

    #[tokio::test]
    async fn test_package_install() {
        let temp_dir = tempdir().unwrap();
        let env_path = temp_dir.path().join("env");
        let cache_path = temp_dir.path().join("cache");

        let (monitor_tx, _) = mpsc::channel(100);
        let mut service = DaemonService::new(monitor_tx);
        
        assert!(service.start().await.is_ok());

        // Create test package
        let package = Package::new(
            PackageId::new(
                "test-package",
                Version::parse("1.0.0").unwrap(),
            ),
            Default::default(),
            Default::default(),
        );

        // Send install request
        let request = UpdateRequest {
            update_type: UpdateType::PackageInstall(package.clone()),
            priority: 0,
        };

        assert!(service.send_update_request(request).await.is_ok());
        assert!(service.stop().await.is_ok());
    }
}

mod daemon_service {
    use super::*;

    #[tokio::test]
    async fn test_daemon_service_lifecycle() {
        let (monitor_tx, mut monitor_rx) = mpsc::channel(100);
        let mut service = DaemonService::new(monitor_tx);

        // Test start
        assert!(service.start().await.is_ok());

        // Test resource usage notification
        let usage = EnvironmentUsage {
            env_disk_usage: Default::default(),
            cache_usage: Default::default(),
            memory_usage: Default::default(),
            cpu_usage: Default::default(),
        };

        assert!(service.notify_resource_usage(usage).await.is_ok());

        // Test package change notification
        assert!(service.notify_package_change().await.is_ok());

        // Test stop
        assert!(service.stop().await.is_ok());
    }

    #[tokio::test]
    async fn test_monitor_events() {
        let (monitor_tx, mut monitor_rx) = mpsc::channel(100);
        let mut service = DaemonService::new(monitor_tx);

        assert!(service.start().await.is_ok());

        // Send resource usage event
        let usage = EnvironmentUsage {
            env_disk_usage: Default::default(),
            cache_usage: Default::default(),
            memory_usage: Default::default(),
            cpu_usage: Default::default(),
        };

        assert!(service.notify_resource_usage(usage.clone()).await.is_ok());

        // Verify event received
        if let Some(MonitorEvent::ResourceUsage(received_usage)) = monitor_rx.recv().await {
            assert_eq!(received_usage.env_disk_usage, usage.env_disk_usage);
            assert_eq!(received_usage.cache_usage, usage.cache_usage);
            assert_eq!(received_usage.memory_usage, usage.memory_usage);
            assert_eq!(received_usage.cpu_usage, usage.cpu_usage);
        } else {
            panic!("Expected ResourceUsage event");
        }

        assert!(service.stop().await.is_ok());
    }
}

mod error_handling {
    use super::*;

    #[tokio::test]
    async fn test_invalid_update_request() {
        let (monitor_tx, _) = mpsc::channel(100);
        let mut service = DaemonService::new(monitor_tx);

        assert!(service.start().await.is_ok());

        // Create invalid package (missing version)
        let package = Package::new(
            PackageId::new(
                "test-package",
                Version::parse("0.0.0").unwrap(), // Invalid version
            ),
            Default::default(),
            Default::default(),
        );

        // Send update request
        let request = UpdateRequest {
            update_type: UpdateType::PackageUpdate {
                package: package.clone(),
                force: false,
                update_deps: true,
            },
            priority: 0,
        };

        // Request should be accepted but update should fail
        assert!(service.send_update_request(request).await.is_ok());
        
        assert!(service.stop().await.is_ok());
    }

    #[tokio::test]
    async fn test_service_shutdown_handling() {
        let (monitor_tx, _) = mpsc::channel(100);
        let mut service = DaemonService::new(monitor_tx);

        assert!(service.start().await.is_ok());

        // Stop service
        assert!(service.stop().await.is_ok());

        // Verify service is stopped by checking that operations fail
        let usage = EnvironmentUsage {
            env_disk_usage: Default::default(),
            cache_usage: Default::default(),
            memory_usage: Default::default(),
            cpu_usage: Default::default(),
        };

        assert!(service.notify_resource_usage(usage).await.is_err());
        assert!(service.notify_package_change().await.is_err());
    }
}
