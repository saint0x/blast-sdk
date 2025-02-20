use std::collections::{HashMap, BinaryHeap, VecDeque};
use std::cmp::{Ord, Ordering};
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast, mpsc};
use chrono::{DateTime, Utc, Duration as ChronoDuration};
use std::time::Duration;
use serde::{Deserialize, Serialize};
use crate::error::{BlastResult, BlastError};
use super::PackageOperation;

/// Operation priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum OperationPriority {
    /// Critical system updates
    Critical = 0,
    /// Security updates
    Security = 1,
    /// High priority operations
    High = 2,
    /// Normal operations
    Normal = 3,
    /// Low priority operations
    Low = 4,
    /// Background operations
    Background = 5,
}

impl Default for OperationPriority {
    fn default() -> Self {
        Self::Normal
    }
}

/// Operation type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OperationType {
    /// Package installation
    Install,
    /// Package uninstallation
    Uninstall,
    /// Package update
    Update,
    /// Dependency resolution
    DependencyResolution,
    /// State update
    StateUpdate,
    /// Graph validation
    GraphValidation,
}

/// Scheduler configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerConfig {
    /// Maximum concurrent operations
    pub max_concurrent_ops: usize,
    /// Operations per minute limit
    pub ops_per_minute: usize,
    /// Maximum queue size
    pub max_queue_size: usize,
    /// Operation timeouts in seconds
    pub operation_timeouts: HashMap<OperationType, u64>,
    /// Priority overrides for specific packages
    pub priority_overrides: HashMap<String, OperationPriority>,
    /// Whether to allow operation reordering
    pub allow_reordering: bool,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        let mut operation_timeouts = HashMap::new();
        operation_timeouts.insert(OperationType::Install, 300); // 5 minutes
        operation_timeouts.insert(OperationType::Uninstall, 180); // 3 minutes
        operation_timeouts.insert(OperationType::Update, 300); // 5 minutes
        operation_timeouts.insert(OperationType::DependencyResolution, 120); // 2 minutes
        operation_timeouts.insert(OperationType::StateUpdate, 60); // 1 minute
        operation_timeouts.insert(OperationType::GraphValidation, 60); // 1 minute

        Self {
            max_concurrent_ops: 3,
            ops_per_minute: 30,
            max_queue_size: 1000,
            operation_timeouts,
            priority_overrides: HashMap::new(),
            allow_reordering: true,
        }
    }
}

/// Prioritized operation
#[derive(Debug, Clone)]
struct PrioritizedOperation {
    /// Operation ID
    id: String,
    /// Operation priority
    priority: OperationPriority,
    /// Operation type
    op_type: OperationType,
    /// Package name
    package_name: String,
    /// Submission timestamp
    submitted_at: DateTime<Utc>,
    /// Operation
    #[allow(dead_code)]
    operation: PackageOperation,
    /// Dependencies (other operation IDs)
    dependencies: Vec<String>,
    /// Whether this is a system critical operation
    is_system_critical: bool,
    /// Timeout duration
    timeout: Duration,
}

impl PartialEq for PrioritizedOperation {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for PrioritizedOperation {}

impl PartialOrd for PrioritizedOperation {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PrioritizedOperation {
    fn cmp(&self, other: &Self) -> Ordering {
        // First compare system critical flag
        other.is_system_critical.cmp(&self.is_system_critical)
            // Then compare priority (lower value = higher priority)
            .then(self.priority.cmp(&other.priority))
            // Then compare submission time
            .then(self.submitted_at.cmp(&other.submitted_at))
    }
}

/// Operation status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OperationStatus {
    /// Operation is queued
    Queued {
        position: usize,
        estimated_start: DateTime<Utc>,
    },
    /// Operation is running
    Running {
        started_at: DateTime<Utc>,
        progress: f32,
        estimated_completion: DateTime<Utc>,
    },
    /// Operation completed successfully
    Completed {
        started_at: DateTime<Utc>,
        completed_at: DateTime<Utc>,
    },
    /// Operation failed
    Failed {
        started_at: DateTime<Utc>,
        failed_at: DateTime<Utc>,
        error: String,
    },
    /// Operation timed out
    TimedOut {
        started_at: DateTime<Utc>,
        timeout_at: DateTime<Utc>,
    },
}

/// Rate limiter implementation
#[derive(Debug)]
struct RateLimiter {
    /// Operations per minute limit
    ops_per_minute: usize,
    /// Operation timestamps
    operation_timestamps: VecDeque<DateTime<Utc>>,
}

impl RateLimiter {
    /// Create new rate limiter
    fn new(ops_per_minute: usize) -> Self {
        Self {
            ops_per_minute,
            operation_timestamps: VecDeque::with_capacity(ops_per_minute),
        }
    }

    /// Check if operation can proceed
    fn can_proceed(&mut self) -> bool {
        let now = Utc::now();
        let minute_ago = now - ChronoDuration::minutes(1);
        
        // Remove timestamps older than 1 minute
        while self.operation_timestamps.front()
            .map_or(false, |&t| t < minute_ago) {
            self.operation_timestamps.pop_front();
        }
        
        // Check if under limit
        self.operation_timestamps.len() < self.ops_per_minute
    }

    /// Record operation
    fn record_operation(&mut self) {
        self.operation_timestamps.push_back(Utc::now());
    }
}

/// Operation scheduler implementation
pub struct OperationScheduler {
    /// Configuration
    config: SchedulerConfig,
    /// Operation queue
    queue: Arc<RwLock<BinaryHeap<PrioritizedOperation>>>,
    /// Active operations
    active_ops: Arc<RwLock<HashMap<String, PrioritizedOperation>>>,
    /// Operation statuses
    operation_statuses: Arc<RwLock<HashMap<String, OperationStatus>>>,
    /// Rate limiter
    rate_limiter: Arc<RwLock<RateLimiter>>,
    /// Status updates channel
    status_tx: broadcast::Sender<(String, OperationStatus)>,
    /// Operation processor channel
    processor_tx: mpsc::Sender<PrioritizedOperation>,
}

impl OperationScheduler {
    /// Create new operation scheduler
    pub fn new(config: SchedulerConfig) -> Self {
        let (status_tx, _) = broadcast::channel(100);
        let (processor_tx, _) = mpsc::channel(100);
        
        Self {
            config: config.clone(),
            queue: Arc::new(RwLock::new(BinaryHeap::new())),
            active_ops: Arc::new(RwLock::new(HashMap::new())),
            operation_statuses: Arc::new(RwLock::new(HashMap::new())),
            rate_limiter: Arc::new(RwLock::new(RateLimiter::new(config.ops_per_minute))),
            status_tx,
            processor_tx,
        }
    }

    /// Queue operation for processing
    pub async fn queue_operation(
        &self,
        operation: PackageOperation,
        priority: Option<OperationPriority>,
        dependencies: Vec<String>,
    ) -> BlastResult<String> {
        // Check queue size limit
        let queue_size = self.queue.read().await.len();
        if queue_size >= self.config.max_queue_size {
            return Err(BlastError::package("Operation queue is full".to_string()));
        }

        // Create prioritized operation
        let op = self.create_prioritized_operation(
            operation,
            priority.unwrap_or_default(),
            dependencies,
        ).await?;

        let op_id = op.id.clone();

        // Add to queue
        {
            let mut queue = self.queue.write().await;
            queue.push(op.clone());

            // Update status
            let position = queue.len();
            let estimated_start = self.estimate_start_time(&op, position).await;
            let status = OperationStatus::Queued {
                position,
                estimated_start,
            };
            self.update_operation_status(&op_id, status).await;
        }

        // Try to process queue
        self.process_queue().await;

        Ok(op_id)
    }

    /// Create prioritized operation
    async fn create_prioritized_operation(
        &self,
        operation: PackageOperation,
        priority: OperationPriority,
        dependencies: Vec<String>,
    ) -> BlastResult<PrioritizedOperation> {
        let id = uuid::Uuid::new_v4().to_string();
        let op_type = match &operation {
            PackageOperation::Install { .. } => OperationType::Install,
            PackageOperation::Uninstall { .. } => OperationType::Uninstall,
            PackageOperation::Update { .. } => OperationType::Update,
        };

        let package_name = match &operation {
            PackageOperation::Install { name, .. } => name.clone(),
            PackageOperation::Uninstall { name } => name.clone(),
            PackageOperation::Update { name, .. } => name.clone(),
        };

        // Check for priority override
        let final_priority = self.config.priority_overrides
            .get(&package_name)
            .copied()
            .unwrap_or(priority);

        // Get timeout for operation type
        let timeout = Duration::from_secs(
            self.config.operation_timeouts
                .get(&op_type)
                .copied()
                .unwrap_or(300)
        );

        Ok(PrioritizedOperation {
            id,
            priority: final_priority,
            op_type,
            package_name,
            submitted_at: Utc::now(),
            operation,
            dependencies,
            is_system_critical: final_priority == OperationPriority::Critical,
            timeout,
        })
    }

    /// Process operation queue
    async fn process_queue(&self) {
        let mut interval = tokio::time::interval(Duration::from_millis(100));
        
        loop {
            interval.tick().await;
            
            // Get next operation that can be processed
            let mut queue = self.queue.write().await;
            let active_ops = self.active_ops.write().await;
            
            if let Some(op) = self.get_next_processable_operation(&queue, &active_ops).await {
                // Check rate limit
                let mut rate_limiter = self.rate_limiter.write().await;
                if !rate_limiter.can_proceed() {
                    continue;
                }
                rate_limiter.record_operation();
                drop(rate_limiter);
                
                // Remove from queue and add to active operations
                queue.pop();
                drop(queue);
                
                let op_id = op.id.clone();
                tracing::info!("Processing operation {} for package {}", op_id, op.package_name);
                
                // Clone op before moving it
                let op_clone = op.clone();
                
                // Send to processor
                if let Err(e) = self.processor_tx.send(op).await {
                    tracing::error!("Failed to send operation to processor: {}", e);
                    continue;
                }
                
                // Update status
                self.update_operation_status(&op_clone.id, OperationStatus::Running {
                    started_at: Utc::now(),
                    progress: 0.0,
                    estimated_completion: Utc::now() + ChronoDuration::seconds(op_clone.timeout.as_secs() as i64),
                }).await;
                
                // Add to active operations with write lock
                let mut active_ops = self.active_ops.write().await;
                active_ops.insert(op_clone.id.clone(), op_clone);
            }
        }
    }

    /// Get next operation that can be processed
    async fn get_next_processable_operation(
        &self,
        queue: &BinaryHeap<PrioritizedOperation>,
        active_ops: &HashMap<String, PrioritizedOperation>,
    ) -> Option<PrioritizedOperation> {
        let statuses = self.operation_statuses.read().await;
        for op in queue.iter() {
            // Check if all dependencies are completed
            let deps_completed = op.dependencies.iter().all(|dep_id| {
                !active_ops.contains_key(dep_id) &&
                matches!(
                    statuses.get(dep_id),
                    Some(OperationStatus::Completed { .. })
                )
            });

            if deps_completed {
                return Some(op.clone());
            }
        }
        None
    }

    /// Estimate operation start time
    async fn estimate_start_time(&self, op: &PrioritizedOperation, position: usize) -> DateTime<Utc> {
        let now = Utc::now();
        
        // Get average duration for operation type
        let avg_duration = ChronoDuration::seconds(
            self.config.operation_timeouts
                .get(&op.op_type)
                .copied()
                .unwrap_or(300) as i64
        );

        // Calculate delay based on position and concurrent operations
        let operations_ahead = position.saturating_sub(1);
        let batches = operations_ahead / self.config.max_concurrent_ops;
        let delay = avg_duration * batches as i32;

        now + delay
    }

    /// Update operation status
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
        if let Err(e) = self.status_tx.send((operation_id.to_string(), status)) {
            tracing::error!("Failed to broadcast operation status update: {}", e);
        }
    }

    /// Get operation status
    pub async fn get_operation_status(&self, operation_id: &str) -> Option<OperationStatus> {
        self.operation_statuses.read().await.get(operation_id).cloned()
    }

    /// Subscribe to status updates
    pub fn subscribe_status_updates(&self) -> broadcast::Receiver<(String, OperationStatus)> {
        self.status_tx.subscribe()
    }

    /// Cancel operation
    pub async fn cancel_operation(&self, operation_id: &str) -> BlastResult<()> {
        // Remove from queue if queued
        {
            let mut queue = self.queue.write().await;
            queue.retain(|op| op.id != operation_id);
        }

        // Remove from active operations if running
        {
            let mut active_ops = self.active_ops.write().await;
            active_ops.remove(operation_id);
        }

        // Update status
        let status = OperationStatus::Failed {
            started_at: Utc::now(),
            failed_at: Utc::now(),
            error: "Operation cancelled".to_string(),
        };
        self.update_operation_status(operation_id, status).await;

        Ok(())
    }

    /// Get queue statistics
    pub async fn get_queue_stats(&self) -> QueueStatistics {
        let queue = self.queue.read().await;
        let active_ops = self.active_ops.read().await;

        QueueStatistics {
            queued_operations: queue.len(),
            active_operations: active_ops.len(),
            operations_by_priority: self.count_operations_by_priority(&queue).await,
            operations_by_type: self.count_operations_by_type(&queue).await,
        }
    }

    /// Count operations by priority
    async fn count_operations_by_priority(
        &self,
        queue: &BinaryHeap<PrioritizedOperation>,
    ) -> HashMap<OperationPriority, usize> {
        let mut counts = HashMap::new();
        for op in queue.iter() {
            *counts.entry(op.priority).or_insert(0) += 1;
        }
        counts
    }

    /// Count operations by type
    async fn count_operations_by_type(
        &self,
        queue: &BinaryHeap<PrioritizedOperation>,
    ) -> HashMap<OperationType, usize> {
        let mut counts = HashMap::new();
        for op in queue.iter() {
            *counts.entry(op.op_type).or_insert(0) += 1;
        }
        counts
    }
}

/// Queue statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueStatistics {
    /// Number of queued operations
    pub queued_operations: usize,
    /// Number of active operations
    pub active_operations: usize,
    /// Operations by priority
    pub operations_by_priority: HashMap<OperationPriority, usize>,
    /// Operations by type
    pub operations_by_type: HashMap<OperationType, usize>,
} 