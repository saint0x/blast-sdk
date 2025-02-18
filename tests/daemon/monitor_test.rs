use std::path::PathBuf;
use tempfile::tempdir;
use blast_daemon::monitor::{
    PythonResourceMonitor,
    PythonResourceLimits,
    EnvironmentUsage,
    EnvDiskUsage,
    CacheUsage,
};

mod resource_monitor {
    use super::*;

    #[test]
    fn test_monitor_creation() {
        let env_dir = tempdir().unwrap();
        let cache_dir = tempdir().unwrap();
        
        let monitor = PythonResourceMonitor::new(
            env_dir.path().to_path_buf(),
            cache_dir.path().to_path_buf(),
            PythonResourceLimits::default(),
        );

        assert!(monitor.get_limits().max_env_size > 0);
        assert!(monitor.get_limits().max_cache_size > 0);
    }

    #[test]
    fn test_resource_usage() {
        let env_dir = tempdir().unwrap();
        let cache_dir = tempdir().unwrap();
        
        let mut monitor = PythonResourceMonitor::new(
            env_dir.path().to_path_buf(),
            cache_dir.path().to_path_buf(),
            PythonResourceLimits::default(),
        );
        
        // Test initial usage
        let usage = monitor.get_current_usage();
        assert_eq!(usage.env_disk_usage.total_size, 0);
        assert_eq!(usage.cache_usage.total_size, 0);
        assert_eq!(usage.cache_usage.package_count, 0);
        
        // Create some test files
        std::fs::create_dir_all(env_dir.path().join("lib/python3.8/site-packages")).unwrap();
        std::fs::write(
            env_dir.path().join("lib/python3.8/site-packages/test.py"),
            "print('test')",
        ).unwrap();
        
        // Test updated usage
        let usage = monitor.get_current_usage();
        assert!(usage.env_disk_usage.total_size > 0);
    }

    #[test]
    fn test_limit_checking() {
        let env_dir = tempdir().unwrap();
        let cache_dir = tempdir().unwrap();
        
        let mut monitor = PythonResourceMonitor::new(
            env_dir.path().to_path_buf(),
            cache_dir.path().to_path_buf(),
            PythonResourceLimits::default(),
        );
        
        // Test initial limits
        assert!(monitor.check_limits());
        
        // Test with strict limits
        let new_limits = PythonResourceLimits {
            max_env_size: 1024, // 1KB
            max_cache_size: 1024, // 1KB
        };
        monitor.update_limits(new_limits);
        
        // Create large test files
        std::fs::create_dir_all(env_dir.path().join("lib/python3.8/site-packages")).unwrap();
        std::fs::write(
            env_dir.path().join("lib/python3.8/site-packages/large.py"),
            vec![b'a'; 2048], // 2KB
        ).unwrap();
        
        // Test limit violation
        assert!(!monitor.check_limits());
    }
}

mod usage_tracking {
    use super::*;

    #[test]
    fn test_env_disk_usage() {
        let env_dir = tempdir().unwrap();
        let cache_dir = tempdir().unwrap();
        
        let mut monitor = PythonResourceMonitor::new(
            env_dir.path().to_path_buf(),
            cache_dir.path().to_path_buf(),
            PythonResourceLimits::default(),
        );
        
        // Create test environment structure
        std::fs::create_dir_all(env_dir.path().join("lib/python3.8/site-packages")).unwrap();
        std::fs::create_dir_all(env_dir.path().join("bin")).unwrap();
        
        // Add some files
        std::fs::write(
            env_dir.path().join("lib/python3.8/site-packages/pkg1.py"),
            "print('pkg1')",
        ).unwrap();
        std::fs::write(
            env_dir.path().join("lib/python3.8/site-packages/pkg2.py"),
            "print('pkg2')",
        ).unwrap();
        
        let usage = monitor.get_current_usage();
        assert!(usage.env_disk_usage.total_size > 0);
        assert!(usage.env_disk_usage.packages_size > 0);
    }

    #[test]
    fn test_cache_usage() {
        let env_dir = tempdir().unwrap();
        let cache_dir = tempdir().unwrap();
        
        let mut monitor = PythonResourceMonitor::new(
            env_dir.path().to_path_buf(),
            cache_dir.path().to_path_buf(),
            PythonResourceLimits::default(),
        );
        
        // Create test cache structure
        std::fs::create_dir_all(cache_dir.path().join("packages")).unwrap();
        
        // Add some cached packages
        std::fs::write(
            cache_dir.path().join("packages/pkg1-1.0.0.whl"),
            vec![b'a'; 1024], // 1KB
        ).unwrap();
        std::fs::write(
            cache_dir.path().join("packages/pkg2-2.0.0.whl"),
            vec![b'b'; 2048], // 2KB
        ).unwrap();
        
        let usage = monitor.get_current_usage();
        assert!(usage.cache_usage.total_size >= 3072); // At least 3KB
        assert_eq!(usage.cache_usage.package_count, 2);
    }
}

mod error_handling {
    use super::*;

    #[test]
    fn test_invalid_paths() {
        let env_dir = tempdir().unwrap();
        let cache_dir = tempdir().unwrap();
        
        let mut monitor = PythonResourceMonitor::new(
            env_dir.path().join("nonexistent"),
            cache_dir.path().join("nonexistent"),
            PythonResourceLimits::default(),
        );
        
        // Should handle nonexistent directories gracefully
        let usage = monitor.get_current_usage();
        assert_eq!(usage.env_disk_usage.total_size, 0);
        assert_eq!(usage.cache_usage.total_size, 0);
    }

    #[test]
    fn test_permission_handling() {
        let env_dir = tempdir().unwrap();
        let cache_dir = tempdir().unwrap();
        
        let mut monitor = PythonResourceMonitor::new(
            env_dir.path().to_path_buf(),
            cache_dir.path().to_path_buf(),
            PythonResourceLimits::default(),
        );
        
        // Create test directories
        std::fs::create_dir_all(env_dir.path().join("lib/python3.8/site-packages")).unwrap();
        std::fs::create_dir_all(cache_dir.path().join("packages")).unwrap();
        
        // Add some files
        std::fs::write(
            env_dir.path().join("lib/python3.8/site-packages/test.py"),
            "print('test')",
        ).unwrap();
        
        // Get initial usage
        let initial_usage = monitor.get_current_usage();
        
        // Change permissions to make directories unreadable
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(env_dir.path(), std::fs::Permissions::from_mode(0o000)).unwrap();
            std::fs::set_permissions(cache_dir.path(), std::fs::Permissions::from_mode(0o000)).unwrap();
        }
        
        // Should handle permission errors gracefully
        let usage = monitor.get_current_usage();
        assert_eq!(usage.env_disk_usage.total_size, initial_usage.env_disk_usage.total_size);
        assert_eq!(usage.cache_usage.total_size, initial_usage.cache_usage.total_size);
        
        // Restore permissions
        #[cfg(unix)]
        {
            std::fs::set_permissions(env_dir.path(), std::fs::Permissions::from_mode(0o755)).unwrap();
            std::fs::set_permissions(cache_dir.path(), std::fs::Permissions::from_mode(0o755)).unwrap();
        }
    }
}
