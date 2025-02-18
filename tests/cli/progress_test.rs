use blast_cli::progress::ProgressManager;

#[test]
fn test_progress_manager() {
    let mut manager = ProgressManager::new();
    
    // Create a progress bar
    let bar = manager.create_bar("test", 100, "Testing");
    assert!(manager.get_bar("test").is_some());
    
    // Update progress
    bar.inc(50);
    assert_eq!(bar.position(), 50);
    
    // Remove bar
    manager.remove_bar("test");
    assert!(manager.get_bar("test").is_none());
}

#[test]
fn test_multiple_bars() {
    let mut manager = ProgressManager::new();
    
    // Create multiple bars
    manager.create_bar("bar1", 100, "Task 1");
    manager.create_bar("bar2", 200, "Task 2");
    
    assert_eq!(manager.bars.len(), 2);
    
    // Clear all bars
    manager.clear_all();
    assert!(manager.bars.is_empty());
}

#[test]
fn test_resolution_progress() {
    let mut manager = ProgressManager::new();
    
    // Test resolution progress
    manager.start_resolution();
    assert!(manager.resolution_spinner.is_some());
    
    manager.finish_resolution();
    assert!(manager.resolution_spinner.is_none());
}

#[test]
fn test_installation_progress() {
    let mut manager = ProgressManager::new();
    
    // Test installation progress
    let total_packages = 5;
    manager.start_installation(total_packages);
    assert!(manager.installation_progress.is_some());
    
    for _ in 0..total_packages {
        manager.increment();
    }
    
    manager.finish_installation();
    assert!(manager.installation_progress.is_none());
}
