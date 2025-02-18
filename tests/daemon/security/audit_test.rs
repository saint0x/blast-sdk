use std::collections::HashMap;
use tempfile::tempdir;
use blast_daemon::security::audit::{AuditLogger, AuditEvent, EventSeverity};
use chrono::Utc;
use std::time::{Duration, SystemTime};
use blast_core::security::{AuditLevel, EventType};

#[tokio::test]
async fn test_audit_logging() {
    let temp_dir = tempdir().unwrap();
    let log_path = temp_dir.path().join("audit.log");
    
    let mut logger = AuditLogger::new(&log_path);
    
    // Add filter
    logger.add_filter("security", EventSeverity::Warning);

    // Log test events
    logger.log_event(AuditEvent {
        event_type: "security".to_string(),
        severity: EventSeverity::Critical,
        details: "Test security event".to_string(),
        package: Some("test-pkg".to_string()),
        environment: Some("test-env".to_string()),
        metadata: HashMap::new(),
    }).await.unwrap();

    // Query logs
    let entries = logger.query_log(
        None,
        None,
        Some(EventSeverity::Warning),
        Some(vec!["security".to_string()]),
    ).await.unwrap();

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].event_type, "security");
    assert_eq!(entries[0].severity, EventSeverity::Critical);

    // Check pending records
    let pending = logger.get_pending().await.unwrap();
    assert_eq!(pending.len(), 1);

    // Clear pending
    logger.clear_pending().await.unwrap();
    assert!(logger.get_pending().await.unwrap().is_empty());
}

#[tokio::test]
async fn test_multiple_event_types() {
    let temp_dir = tempdir().unwrap();
    let log_path = temp_dir.path().join("audit.log");
    let mut logger = AuditLogger::new(&log_path);

    // Add filters for different event types
    logger.add_filter("security", EventSeverity::Warning);
    logger.add_filter("package", EventSeverity::Info);

    // Log events of different types
    let events = vec![
        ("security", EventSeverity::Critical, "Security breach detected"),
        ("package", EventSeverity::Info, "Package installed"),
        ("security", EventSeverity::Warning, "Suspicious activity"),
    ];

    for (event_type, severity, details) in events {
        logger.log_event(AuditEvent {
            event_type: event_type.to_string(),
            severity,
            details: details.to_string(),
            package: None,
            environment: None,
            metadata: HashMap::new(),
        }).await.unwrap();
    }

    // Query by event type
    let security_events = logger.query_log(
        None,
        None,
        None,
        Some(vec!["security".to_string()]),
    ).await.unwrap();
    assert_eq!(security_events.len(), 2);

    let package_events = logger.query_log(
        None,
        None,
        None,
        Some(vec!["package".to_string()]),
    ).await.unwrap();
    assert_eq!(package_events.len(), 1);
}

#[tokio::test]
async fn test_severity_filtering() {
    let temp_dir = tempdir().unwrap();
    let log_path = temp_dir.path().join("audit.log");
    let mut logger = AuditLogger::new(&log_path);

    // Log events with different severities
    let severities = vec![
        EventSeverity::Debug,
        EventSeverity::Info,
        EventSeverity::Warning,
        EventSeverity::Error,
        EventSeverity::Critical,
    ];

    for severity in &severities {
        logger.log_event(AuditEvent {
            event_type: "test".to_string(),
            severity: *severity,
            details: format!("{:?} event", severity),
            package: None,
            environment: None,
            metadata: HashMap::new(),
        }).await.unwrap();
    }

    // Query with different severity thresholds
    let critical_only = logger.query_log(
        None,
        None,
        Some(EventSeverity::Critical),
        None,
    ).await.unwrap();
    assert_eq!(critical_only.len(), 1);

    let warning_and_above = logger.query_log(
        None,
        None,
        Some(EventSeverity::Warning),
        None,
    ).await.unwrap();
    assert_eq!(warning_and_above.len(), 3); // Warning, Error, Critical
}

#[tokio::test]
async fn test_time_range_filtering() {
    let temp_dir = tempdir().unwrap();
    let log_path = temp_dir.path().join("audit.log");
    let mut logger = AuditLogger::new(&log_path);

    let start_time = Utc::now().timestamp() as u64;
    
    // Log some events
    logger.log_event(AuditEvent {
        event_type: "test".to_string(),
        severity: EventSeverity::Info,
        details: "Test event".to_string(),
        package: None,
        environment: None,
        metadata: HashMap::new(),
    }).await.unwrap();

    let mid_time = Utc::now().timestamp() as u64;

    logger.log_event(AuditEvent {
        event_type: "test".to_string(),
        severity: EventSeverity::Info,
        details: "Another test event".to_string(),
        package: None,
        environment: None,
        metadata: HashMap::new(),
    }).await.unwrap();

    let end_time = Utc::now().timestamp() as u64;

    // Query with time range
    let all_events = logger.query_log(
        Some(start_time),
        Some(end_time),
        None,
        None,
    ).await.unwrap();
    assert_eq!(all_events.len(), 2);

    let mid_events = logger.query_log(
        Some(mid_time),
        Some(end_time),
        None,
        None,
    ).await.unwrap();
    assert_eq!(mid_events.len(), 1);
}

mod event_logging {
    use super::*;

    #[tokio::test]
    async fn test_basic_event_logging() {
        let logger = AuditLogger::new();
        
        let event = AuditEvent {
            timestamp: SystemTime::now(),
            level: AuditLevel::Info,
            event_type: EventType::ProcessStart,
            message: "Process started".into(),
            metadata: Default::default(),
        };
        
        logger.log_event(event.clone()).await.unwrap();
        
        let events = logger.get_events().await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].message, event.message);
    }

    #[tokio::test]
    async fn test_multiple_event_types() {
        let logger = AuditLogger::new();
        
        // Log different types of events
        let events = vec![
            AuditEvent {
                timestamp: SystemTime::now(),
                level: AuditLevel::Info,
                event_type: EventType::ProcessStart,
                message: "Process started".into(),
                metadata: Default::default(),
            },
            AuditEvent {
                timestamp: SystemTime::now(),
                level: AuditLevel::Warning,
                event_type: EventType::ResourceLimit,
                message: "Resource limit reached".into(),
                metadata: Default::default(),
            },
            AuditEvent {
                timestamp: SystemTime::now(),
                level: AuditLevel::Error,
                event_type: EventType::SecurityViolation,
                message: "Security violation detected".into(),
                metadata: Default::default(),
            },
        ];
        
        for event in events.clone() {
            logger.log_event(event).await.unwrap();
        }
        
        let logged_events = logger.get_events().await.unwrap();
        assert_eq!(logged_events.len(), 3);
        
        // Verify event types are preserved
        assert!(logged_events.iter().any(|e| e.event_type == EventType::ProcessStart));
        assert!(logged_events.iter().any(|e| e.event_type == EventType::ResourceLimit));
        assert!(logged_events.iter().any(|e| e.event_type == EventType::SecurityViolation));
    }
}

mod event_filtering {
    use super::*;

    #[tokio::test]
    async fn test_level_filtering() {
        let logger = AuditLogger::new();
        
        // Log events with different levels
        let events = vec![
            AuditEvent {
                timestamp: SystemTime::now(),
                level: AuditLevel::Debug,
                event_type: EventType::ProcessStart,
                message: "Debug event".into(),
                metadata: Default::default(),
            },
            AuditEvent {
                timestamp: SystemTime::now(),
                level: AuditLevel::Warning,
                event_type: EventType::ProcessStart,
                message: "Warning event".into(),
                metadata: Default::default(),
            },
        ];
        
        for event in events {
            logger.log_event(event).await.unwrap();
        }
        
        let warning_events = logger.get_events_by_level(AuditLevel::Warning).await.unwrap();
        assert_eq!(warning_events.len(), 1);
        assert_eq!(warning_events[0].level, AuditLevel::Warning);
    }

    #[tokio::test]
    async fn test_type_filtering() {
        let logger = AuditLogger::new();
        
        // Log events with different types
        let events = vec![
            AuditEvent {
                timestamp: SystemTime::now(),
                level: AuditLevel::Info,
                event_type: EventType::ProcessStart,
                message: "Process event".into(),
                metadata: Default::default(),
            },
            AuditEvent {
                timestamp: SystemTime::now(),
                level: AuditLevel::Info,
                event_type: EventType::ResourceLimit,
                message: "Resource event".into(),
                metadata: Default::default(),
            },
        ];
        
        for event in events {
            logger.log_event(event).await.unwrap();
        }
        
        let process_events = logger.get_events_by_type(EventType::ProcessStart).await.unwrap();
        assert_eq!(process_events.len(), 1);
        assert_eq!(process_events[0].event_type, EventType::ProcessStart);
    }
}

mod time_range_queries {
    use super::*;

    #[tokio::test]
    async fn test_time_range_filtering() {
        let logger = AuditLogger::new();
        
        let start_time = SystemTime::now();
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // Log some events
        logger.log_event(AuditEvent {
            timestamp: SystemTime::now(),
            level: AuditLevel::Info,
            event_type: EventType::ProcessStart,
            message: "Test event".into(),
            metadata: Default::default(),
        }).await.unwrap();
        
        tokio::time::sleep(Duration::from_millis(100)).await;
        let end_time = SystemTime::now();
        
        // Log more events outside the range
        logger.log_event(AuditEvent {
            timestamp: SystemTime::now(),
            level: AuditLevel::Info,
            event_type: EventType::ProcessStart,
            message: "Outside range".into(),
            metadata: Default::default(),
        }).await.unwrap();
        
        let range_events = logger.get_events_in_range(start_time..end_time).await.unwrap();
        assert_eq!(range_events.len(), 1);
        assert!(range_events[0].message.contains("Test event"));
    }
} 