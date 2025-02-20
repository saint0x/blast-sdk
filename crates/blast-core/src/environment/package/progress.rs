use std::collections::HashMap;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Installation step in the package operation process
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum InstallationStep {
    /// Downloading package
    Downloading,
    /// Resolving dependencies
    ResolvingDependencies,
    /// Validating dependency graph
    ValidatingGraph,
    /// Installing package
    Installing,
    /// Configuring package
    ConfiguringPackage,
    /// Updating state
    UpdatingState,
    /// Operation complete
    Complete,
    /// Operation failed
    Failed,
}

/// Progress information for an installation operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallationProgress {
    /// Unique operation identifier
    pub operation_id: String,
    /// Package name
    pub package_name: String,
    /// Current installation step
    pub step: InstallationStep,
    /// Progress percentage (0.0 to 1.0)
    pub progress: f32,
    /// Current action description
    pub current_action: String,
    /// Operation start time
    pub started_at: DateTime<Utc>,
    /// Estimated completion time
    pub estimated_completion: Option<DateTime<Utc>>,
    /// Error message if failed
    pub error: Option<String>,
    /// Child operation progress (for dependency installations)
    pub children: HashMap<String, InstallationProgress>,
}

/// Progress tracker for package operations
#[derive(Debug)]
pub struct ProgressTracker {
    /// Active operations
    operations: Arc<RwLock<HashMap<String, InstallationProgress>>>,
    /// Progress update broadcaster
    progress_tx: broadcast::Sender<InstallationProgress>,
}

impl InstallationProgress {
    /// Create new installation progress
    pub fn new(package_name: String) -> Self {
        Self {
            operation_id: Uuid::new_v4().to_string(),
            package_name,
            step: InstallationStep::Downloading,
            progress: 0.0,
            current_action: "Initializing".to_string(),
            started_at: Utc::now(),
            estimated_completion: None,
            error: None,
            children: HashMap::new(),
        }
    }

    /// Update progress
    pub fn update_progress(&mut self, progress: f32, action: impl Into<String>) {
        self.progress = progress.clamp(0.0, 1.0);
        self.current_action = action.into();
    }

    /// Set current step
    pub fn set_step(&mut self, step: InstallationStep) {
        let step_clone = step.clone();
        self.step = step;
        match step_clone {
            InstallationStep::Complete => {
                self.progress = 1.0;
                self.current_action = "Complete".to_string();
            }
            InstallationStep::Failed => {
                self.current_action = "Failed".to_string();
            }
            _ => {}
        }
    }

    /// Mark as failed with error
    pub fn set_failed(&mut self, error: impl Into<String>) {
        self.step = InstallationStep::Failed;
        self.error = Some(error.into());
        self.current_action = "Failed".to_string();
    }

    /// Add child operation
    pub fn add_child(&mut self, child: InstallationProgress) {
        self.children.insert(child.operation_id.clone(), child);
    }

    /// Update child operation
    pub fn update_child(&mut self, child_id: &str, progress: InstallationProgress) {
        if let Some(child) = self.children.get_mut(child_id) {
            *child = progress;
        }
    }

    /// Calculate total progress including children
    pub fn total_progress(&self) -> f32 {
        if self.children.is_empty() {
            return self.progress;
        }

        let child_progress: f32 = self.children.values()
            .map(|child| child.total_progress())
            .sum::<f32>() / self.children.len() as f32;

        (self.progress + child_progress) / 2.0
    }
}

impl ProgressTracker {
    /// Create new progress tracker
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(100);
        Self {
            operations: Arc::new(RwLock::new(HashMap::new())),
            progress_tx: tx,
        }
    }

    /// Start tracking new operation
    pub async fn start_operation(&self, package_name: String) -> String {
        let progress = InstallationProgress::new(package_name);
        let operation_id = progress.operation_id.clone();
        
        let mut operations = self.operations.write().await;
        operations.insert(operation_id.clone(), progress.clone());
        let _ = self.progress_tx.send(progress);
        
        operation_id
    }

    /// Update operation progress
    pub async fn update_operation(
        &self,
        operation_id: &str,
        step: InstallationStep,
        progress: f32,
        action: impl Into<String>,
    ) -> Option<()> {
        let mut operations = self.operations.write().await;
        let operation = operations.get_mut(operation_id)?;
        
        operation.set_step(step);
        operation.update_progress(progress, action);
        let _ = self.progress_tx.send(operation.clone());
        
        Some(())
    }

    /// Mark operation as failed
    pub async fn fail_operation(&self, operation_id: &str, error: impl Into<String>) -> Option<()> {
        let mut operations = self.operations.write().await;
        let operation = operations.get_mut(operation_id)?;
        
        operation.set_failed(error);
        let _ = self.progress_tx.send(operation.clone());
        
        Some(())
    }

    /// Complete operation
    pub async fn complete_operation(&self, operation_id: &str) -> Option<()> {
        let mut operations = self.operations.write().await;
        let operation = operations.get_mut(operation_id)?;
        
        operation.set_step(InstallationStep::Complete);
        let _ = self.progress_tx.send(operation.clone());
        
        Some(())
    }

    /// Subscribe to progress updates
    pub fn subscribe(&self) -> broadcast::Receiver<InstallationProgress> {
        self.progress_tx.subscribe()
    }

    /// Get current progress
    pub async fn get_progress(&self, operation_id: &str) -> Option<InstallationProgress> {
        let operations = self.operations.read().await;
        operations.get(operation_id).cloned()
    }

    /// Get all active operations
    pub async fn get_active_operations(&self) -> Vec<InstallationProgress> {
        let operations = self.operations.read().await;
        operations.values().cloned().collect()
    }
}

impl Default for ProgressTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_progress_tracking() {
        let tracker = ProgressTracker::new();
        let mut receiver = tracker.subscribe();

        // Start operation
        let op_id = tracker.start_operation("test-package".to_string()).await;
        
        // Verify initial state
        let progress = tracker.get_progress(&op_id).await.unwrap();
        assert_eq!(progress.package_name, "test-package");
        assert_eq!(progress.step, InstallationStep::Downloading);
        assert_eq!(progress.progress, 0.0);
        
        // Update progress
        tracker.update_operation(
            &op_id,
            InstallationStep::ResolvingDependencies,
            0.5,
            "Resolving dependencies",
        ).await;
        
        // Verify update
        let progress = tracker.get_progress(&op_id).await.unwrap();
        assert_eq!(progress.step, InstallationStep::ResolvingDependencies);
        assert_eq!(progress.progress, 0.5);
        assert_eq!(progress.current_action, "Resolving dependencies");
        
        // Verify broadcast
        let update = receiver.try_recv().unwrap();
        assert_eq!(update.operation_id, op_id);
        assert_eq!(update.step, InstallationStep::ResolvingDependencies);
        
        // Complete operation
        tracker.complete_operation(&op_id).await;
        
        // Verify completion
        let progress = tracker.get_progress(&op_id).await.unwrap();
        assert_eq!(progress.step, InstallationStep::Complete);
        assert_eq!(progress.progress, 1.0);
    }

    #[tokio::test]
    async fn test_nested_progress() {
        let tracker = ProgressTracker::new();
        
        // Start parent operation
        let _parent_id = tracker.start_operation("parent-package".to_string()).await;
        let mut parent = InstallationProgress::new("parent-package".to_string());
        
        // Add child operations
        let child1 = InstallationProgress::new("child1".to_string());
        let child2 = InstallationProgress::new("child2".to_string());
        
        parent.add_child(child1);
        parent.add_child(child2);
        
        // Update parent progress
        parent.update_progress(0.5, "Installing");
        
        // Verify total progress
        assert_eq!(parent.total_progress(), 0.25); // (0.5 + 0.0 + 0.0) / 2
        
        // Update children progress
        let mut updated_child = InstallationProgress::new("child1".to_string());
        updated_child.update_progress(1.0, "Complete");
        let child_id = updated_child.operation_id.clone();
        parent.update_child(&child_id, updated_child);
        
        // Verify updated total progress
        assert_eq!(parent.total_progress(), 0.375); // (0.5 + (1.0 + 0.0) / 2) / 2
    }

    #[tokio::test]
    async fn test_error_handling() {
        let tracker = ProgressTracker::new();
        let mut receiver = tracker.subscribe();
        
        // Start operation
        let op_id = tracker.start_operation("test-package".to_string()).await;
        
        // Fail operation
        tracker.fail_operation(&op_id, "Test error").await;
        
        // Verify failure state
        let progress = tracker.get_progress(&op_id).await.unwrap();
        assert_eq!(progress.step, InstallationStep::Failed);
        assert_eq!(progress.error, Some("Test error".to_string()));
        
        // Verify broadcast
        let update = receiver.try_recv().unwrap();
        assert_eq!(update.step, InstallationStep::Failed);
        assert_eq!(update.error, Some("Test error".to_string()));
    }

    #[tokio::test]
    async fn test_multiple_operations() {
        let tracker = ProgressTracker::new();
        
        // Start multiple operations
        let op1_id = tracker.start_operation("package1".to_string()).await;
        let op2_id = tracker.start_operation("package2".to_string()).await;
        
        // Update operations
        tracker.update_operation(&op1_id, InstallationStep::Installing, 0.5, "Installing").await;
        tracker.update_operation(&op2_id, InstallationStep::ResolvingDependencies, 0.3, "Resolving").await;
        
        // Get active operations
        let active_ops = tracker.get_active_operations().await;
        assert_eq!(active_ops.len(), 2);
        
        // Verify individual states
        let op1 = tracker.get_progress(&op1_id).await.unwrap();
        let op2 = tracker.get_progress(&op2_id).await.unwrap();
        
        assert_eq!(op1.step, InstallationStep::Installing);
        assert_eq!(op2.step, InstallationStep::ResolvingDependencies);
    }
} 