use blast_core::environment::package::{
    OperationScheduler, 
    SchedulerConfig, 
    PackageOperation, 
    OperationPriority,
    OperationStatus
};
use chrono::Utc;
use tokio::time::sleep;
use std::time::Duration;

#[tokio::test]
async fn test_operation_scheduling() {
    let config = SchedulerConfig::default();
    let scheduler = OperationScheduler::new(config);

    // Queue multiple operations
    let op1 = PackageOperation::Install {
        name: "package1".to_string(),
        version: None,
        dependencies: Vec::new(),
    };
    let op2 = PackageOperation::Install {
        name: "package2".to_string(),
        version: None,
        dependencies: Vec::new(),
    };

    let id1 = scheduler.queue_operation(op1, Some(OperationPriority::High), Vec::new()).await.unwrap();
    let id2 = scheduler.queue_operation(op2, Some(OperationPriority::Normal), Vec::new()).await.unwrap();

    // Verify queue state
    let stats = scheduler.get_queue_stats().await;
    assert_eq!(stats.queued_operations, 2);
    assert_eq!(
        *stats.operations_by_priority.get(&OperationPriority::High).unwrap(),
        1
    );
    assert_eq!(
        *stats.operations_by_priority.get(&OperationPriority::Normal).unwrap(),
        1
    );

    // Verify operation statuses
    let status1 = scheduler.get_operation_status(&id1).await.unwrap();
    let status2 = scheduler.get_operation_status(&id2).await.unwrap();

    match status1 {
        OperationStatus::Queued { position, .. } => assert_eq!(position, 1),
        _ => panic!("Unexpected status"),
    }
    match status2 {
        OperationStatus::Queued { position, .. } => assert_eq!(position, 2),
        _ => panic!("Unexpected status"),
    }
}

#[tokio::test]
async fn test_rate_limiting() {
    let mut config = SchedulerConfig::default();
    config.ops_per_minute = 2;
    let scheduler = OperationScheduler::new(config);

    // Queue operations rapidly
    for i in 0..3 {
        let op = PackageOperation::Install {
            name: format!("package{}", i),
            version: None,
            dependencies: Vec::new(),
        };
        scheduler.queue_operation(op, None, Vec::new()).await.unwrap();
    }

    // Verify rate limiting
    let stats = scheduler.get_queue_stats().await;
    assert!(stats.active_operations <= 2);

    // Wait for rate limit to reset
    sleep(Duration::from_secs(60)).await;

    // Queue another operation
    let op = PackageOperation::Install {
        name: "package4".to_string(),
        version: None,
        dependencies: Vec::new(),
    };
    scheduler.queue_operation(op, None, Vec::new()).await.unwrap();

    // Verify operation was accepted
    let stats = scheduler.get_queue_stats().await;
    assert!(stats.queued_operations > 0);
}

#[tokio::test]
async fn test_priority_ordering() {
    let scheduler = OperationScheduler::new(SchedulerConfig::default());

    // Queue operations with different priorities
    let ops = vec![
        (PackageOperation::Install {
            name: "low".to_string(),
            version: None,
            dependencies: Vec::new(),
        }, OperationPriority::Low),
        (PackageOperation::Install {
            name: "critical".to_string(),
            version: None,
            dependencies: Vec::new(),
        }, OperationPriority::Critical),
        (PackageOperation::Install {
            name: "normal".to_string(),
            version: None,
            dependencies: Vec::new(),
        }, OperationPriority::Normal),
    ];

    for (op, priority) in ops {
        scheduler.queue_operation(op, Some(priority), Vec::new()).await.unwrap();
    }

    // Verify ordering
    let queue = scheduler.queue.read().await;
    let ordered: Vec<_> = queue.iter().map(|op| op.priority).collect();
    assert_eq!(ordered, vec![OperationPriority::Critical, OperationPriority::Normal, OperationPriority::Low]);
}

#[tokio::test]
async fn test_dependency_handling() {
    let scheduler = OperationScheduler::new(SchedulerConfig::default());

    // Queue dependent operations
    let op1 = PackageOperation::Install {
        name: "base".to_string(),
        version: None,
        dependencies: Vec::new(),
    };
    let id1 = scheduler.queue_operation(op1, None, Vec::new()).await.unwrap();

    let op2 = PackageOperation::Install {
        name: "dependent".to_string(),
        version: None,
        dependencies: Vec::new(),
    };
    scheduler.queue_operation(op2, None, vec![id1.clone()]).await.unwrap();

    // Verify dependent operation doesn't start until dependency completes
    let stats = scheduler.get_queue_stats().await;
    assert_eq!(stats.active_operations, 1);

    // Complete first operation
    scheduler.update_operation_status(&id1, OperationStatus::Completed {
        started_at: Utc::now(),
        completed_at: Utc::now(),
    }).await;

    // Verify dependent operation starts
    let stats = scheduler.get_queue_stats().await;
    assert_eq!(stats.active_operations, 1);
} 