use blast_core::error::BlastError;
use blast_daemon::error::DaemonError;

mod error_creation {
    use super::*;

    #[test]
    fn test_service_error() {
        let err = DaemonError::service("Service failed");
        assert!(matches!(err, DaemonError::Service(_)));
        assert_eq!(err.to_string(), "Service error: Service failed");
    }

    #[test]
    fn test_ipc_error() {
        let err = DaemonError::ipc("IPC failed");
        assert!(matches!(err, DaemonError::Ipc(_)));
        assert_eq!(err.to_string(), "IPC error: IPC failed");
    }

    #[test]
    fn test_monitor_error() {
        let err = DaemonError::monitor("Monitor failed");
        assert!(matches!(err, DaemonError::Monitor(_)));
        assert_eq!(err.to_string(), "Monitor error: Monitor failed");
    }

    #[test]
    fn test_core_error() {
        let err = DaemonError::core("Core failed");
        assert!(matches!(err, DaemonError::Core(_)));
        assert_eq!(err.to_string(), "Core error: Core failed");
    }

    #[test]
    fn test_resolver_error() {
        let err = DaemonError::resolver("Resolver failed");
        assert!(matches!(err, DaemonError::Resolver(_)));
        assert_eq!(err.to_string(), "Resolver error: Resolver failed");
    }

    #[test]
    fn test_transaction_error() {
        let err = DaemonError::transaction("Transaction failed");
        assert!(matches!(err, DaemonError::Transaction(_)));
        assert_eq!(err.to_string(), "Transaction error: Transaction failed");
    }

    #[test]
    fn test_validation_error() {
        let err = DaemonError::validation("Validation failed");
        assert!(matches!(err, DaemonError::Validation(_)));
        assert_eq!(err.to_string(), "Validation error: Validation failed");
    }

    #[test]
    fn test_version_error() {
        let err = DaemonError::version("Version failed");
        assert!(matches!(err, DaemonError::Version(_)));
        assert_eq!(err.to_string(), "Version error: Version failed");
    }

    #[test]
    fn test_resource_limit_error() {
        let err = DaemonError::resource_limit("Resource limit exceeded");
        assert!(matches!(err, DaemonError::ResourceLimit(_)));
        assert_eq!(err.to_string(), "Resource limit exceeded: Resource limit exceeded");
    }

    #[test]
    fn test_state_error() {
        let err = DaemonError::state("Invalid state");
        assert!(matches!(err, DaemonError::State(_)));
        assert_eq!(err.to_string(), "State error: Invalid state");
    }

    #[test]
    fn test_snapshot_error() {
        let err = DaemonError::snapshot("Failed to create snapshot");
        assert!(matches!(err, DaemonError::Snapshot(_)));
        assert_eq!(err.to_string(), "Snapshot error: Failed to create snapshot");
    }

    #[test]
    fn test_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
        let err = DaemonError::from(io_err);
        assert!(matches!(err, DaemonError::Io(_)));
        assert!(err.to_string().contains("File not found"));
    }
}

mod error_conversion {
    use super::*;

    #[test]
    fn test_blast_to_daemon_conversion() {
        // Test conversion from BlastError::State
        let blast_err = BlastError::State("Invalid state".to_string());
        let daemon_err: DaemonError = blast_err.into();
        assert!(matches!(daemon_err, DaemonError::State(_)));

        // Test conversion from BlastError::Version
        let blast_err = BlastError::Version("Invalid version".to_string());
        let daemon_err: DaemonError = blast_err.into();
        assert!(matches!(daemon_err, DaemonError::Version(_)));

        // Test conversion from BlastError::Package
        let blast_err = BlastError::Package("Invalid package".to_string());
        let daemon_err: DaemonError = blast_err.into();
        assert!(matches!(daemon_err, DaemonError::Core(_)));
    }

    #[test]
    fn test_daemon_to_blast_conversion() {
        // Test conversion from DaemonError::State
        let daemon_err = DaemonError::State("Invalid state".to_string());
        let blast_err: BlastError = daemon_err.into();
        assert!(matches!(blast_err, BlastError::State(_)));

        // Test conversion from DaemonError::Version
        let daemon_err = DaemonError::Version("Invalid version".to_string());
        let blast_err: BlastError = daemon_err.into();
        assert!(matches!(blast_err, BlastError::Version(_)));

        // Test conversion from DaemonError::Core
        let daemon_err = DaemonError::Core("Core error".to_string());
        let blast_err: BlastError = daemon_err.into();
        assert!(matches!(blast_err, BlastError::State(_)));
    }
}

mod error_traits {
    use super::*;

    #[test]
    fn test_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}

        assert_send::<DaemonError>();
        assert_sync::<DaemonError>();
    }

    #[test]
    fn test_error_display() {
        let err = DaemonError::service("Test error");
        assert_eq!(err.to_string(), "Service error: Test error");

        let err = DaemonError::state("Invalid state");
        assert_eq!(err.to_string(), "State error: Invalid state");
    }

    #[test]
    fn test_error_debug() {
        let err = DaemonError::service("Test error");
        assert!(format!("{:?}", err).contains("Service"));
        assert!(format!("{:?}", err).contains("Test error"));
    }
}
