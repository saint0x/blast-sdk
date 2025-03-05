use std::io::Write;
use termcolor::{StandardStream, ColorChoice, ColorSpec, Color, WriteColor};

impl Scheduler {
    async fn update_operation_status(&self, operation_id: &str, status: OperationStatus) {
        let mut statuses = self.operation_statuses.write().await;
        
        // Update status based on type
        match &status {
            OperationStatus::Queued { position, estimated_start, .. } => {
                tracing::info!(
                    "Operation {} queued at position {}, estimated start: {}",
                    operation_id, position, estimated_start
                );
            }
            OperationStatus::Running { started_at, progress, estimated_completion, .. } => {
                tracing::info!(
                    "Operation {} running (started at: {}, progress: {:.1}%, estimated completion: {})",
                    operation_id, started_at, progress * 100.0, estimated_completion
                );
            }
            OperationStatus::Completed { started_at, completed_at, .. } => {
                // Get package name from operation
                if let Some(operation) = self.active_operations.read().await.get(operation_id) {
                    match &operation.operation_type {
                        OperationType::Install { package, .. } => {
                            // Print success message in blue
                            let mut stdout = StandardStream::stdout(ColorChoice::Auto);
                            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Blue))).ok();
                            writeln!(&mut stdout, "✓ Package: {} installed!", package.name()).ok();
                            stdout.reset().ok();
                        }
                        _ => {}
                    }
                }
                tracing::info!(
                    "Operation {} completed (started: {}, completed: {})",
                    operation_id, started_at, completed_at
                );
            }
            OperationStatus::Failed { started_at, failed_at, error, .. } => {
                tracing::error!(
                    "Operation {} failed at {} with error: {} (started at: {})",
                    operation_id, failed_at, error, started_at
                );
            }
            OperationStatus::TimedOut { started_at, timeout_at, .. } => {
                tracing::warn!(
                    "Operation {} timed out at {} (started at: {})",
                    operation_id, timeout_at, started_at
                );
            }
        }
        
        // Store status
        statuses.insert(operation_id.to_string(), status.clone());
        
        // Broadcast status update
        // ... existing code ...
    }
} 