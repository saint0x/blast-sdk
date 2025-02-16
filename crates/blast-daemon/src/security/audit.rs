use std::{
    collections::HashMap,
    path::PathBuf,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::Mutex;
use serde::{Serialize, Deserialize};
use blast_core::{
    error::{BlastError, BlastResult},
    security::{AuditRecord, AuditEvent, EventSeverity},
};

/// Audit logger for security events
pub struct AuditLogger {
    /// Log file path
    log_path: PathBuf,
    /// Pending records
    pending: Arc<Mutex<Vec<AuditRecord>>>,
    /// Event filters
    filters: HashMap<String, EventSeverity>,
}

/// Audit log entry
#[derive(Debug, Serialize, Deserialize)]
struct LogEntry {
    /// Timestamp in Unix seconds
    timestamp: u64,
    /// Event type
    event_type: String,
    /// Event severity
    severity: EventSeverity,
    /// Event details
    details: String,
    /// Related package if any
    package: Option<String>,
    /// Related environment if any
    environment: Option<String>,
    /// Additional metadata
    metadata: HashMap<String, String>,
}

impl AuditLogger {
    /// Create new audit logger
    pub fn new<P: Into<PathBuf>>(log_path: P) -> Self {
        Self {
            log_path: log_path.into(),
            pending: Arc::new(Mutex::new(Vec::new())),
            filters: HashMap::new(),
        }
    }

    /// Add event filter
    pub fn add_filter(&mut self, event_type: &str, min_severity: EventSeverity) {
        self.filters.insert(event_type.to_string(), min_severity);
    }

    /// Log audit event
    pub async fn log_event(&self, event: AuditEvent) -> BlastResult<()> {
        // Check if event should be filtered
        if let Some(min_severity) = self.filters.get(&event.event_type) {
            if event.severity < *min_severity {
                return Ok(());
            }
        }

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let entry = LogEntry {
            timestamp,
            event_type: event.event_type,
            severity: event.severity,
            details: event.details,
            package: event.package,
            environment: event.environment,
            metadata: event.metadata,
        };

        // Add to pending records
        self.pending.lock().await.push(AuditRecord {
            timestamp,
            event_type: entry.event_type.clone(),
            severity: entry.severity,
            details: entry.details.clone(),
        });

        // Write to log file
        let log_line = serde_json::to_string(&entry)?;
        tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)
            .await?
            .write_all(format!("{}\n", log_line).as_bytes())
            .await?;

        Ok(())
    }

    /// Get pending audit records
    pub async fn get_pending(&self) -> BlastResult<Vec<AuditRecord>> {
        Ok(self.pending.lock().await.clone())
    }

    /// Clear pending audit records
    pub async fn clear_pending(&self) -> BlastResult<()> {
        self.pending.lock().await.clear();
        Ok(())
    }

    /// Query audit log
    pub async fn query_log(&self, 
        start_time: Option<u64>,
        end_time: Option<u64>,
        min_severity: Option<EventSeverity>,
        event_types: Option<Vec<String>>,
    ) -> BlastResult<Vec<LogEntry>> {
        let content = tokio::fs::read_to_string(&self.log_path).await?;
        let mut entries: Vec<LogEntry> = content
            .lines()
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect();

        // Apply filters
        if let Some(start) = start_time {
            entries.retain(|e| e.timestamp >= start);
        }
        if let Some(end) = end_time {
            entries.retain(|e| e.timestamp <= end);
        }
        if let Some(severity) = min_severity {
            entries.retain(|e| e.severity >= severity);
        }
        if let Some(types) = event_types {
            entries.retain(|e| types.contains(&e.event_type));
        }

        Ok(entries)
    }

    /// Start background flush task
    pub async fn start_flush_task(&self, interval: std::time::Duration) -> BlastResult<()> {
        let pending = self.pending.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(interval);
            
            loop {
                interval.tick().await;
                pending.lock().await.clear();
            }
        });

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

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
} 