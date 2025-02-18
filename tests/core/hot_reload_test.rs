use std::collections::HashSet;
use std::path::PathBuf;
use tempfile::tempdir;
use blast_core::{
    hot_reload::{HotReloadManager, HotReloadError},
    python::{PythonEnvironment, PythonVersion},
    package::{Package, PackageMetadata},
    version::VersionConstraint,
};

#[tokio::test]
async fn test_hot_reload_basic() {
    let temp_dir = tempdir().unwrap();
    let env = PythonEnvironment::new(
        temp_dir.path().to_path_buf(),
        PythonVersion::default(),
    );
    
    let manager = HotReloadManager::new().await.unwrap();
    manager.register_environment(env).await.unwrap();
    
    // Test monitoring
    manager.start_monitoring().await.unwrap();
}

#[tokio::test]
async fn test_file_analysis() {
    let manager = HotReloadManager::new().await.unwrap();
    let temp_dir = tempdir().unwrap();
    
    // Create a test Python file
    let test_file = temp_dir.path().join("test.py");
    std::fs::write(&test_file, b"import numpy\nimport pandas\n").unwrap();
    
    // Analyze imports
    let imports = manager.analyze_file(&test_file).await.unwrap();
    assert!(imports.contains("numpy"));
    assert!(imports.contains("pandas"));
}

#[tokio::test]
async fn test_package_installation_tracking() {
    let temp_dir = tempdir().unwrap();
    let env = PythonEnvironment::new(
        temp_dir.path().to_path_buf(),
        PythonVersion::default(),
    );
    
    let manager = HotReloadManager::new().await.unwrap();
    manager.register_environment(env.clone()).await.unwrap();
    
    // Create test package
    let package = Package::new(
        "test-package".to_string(),
        "1.0.0".to_string(),
        PackageMetadata::new(
            "test-package".to_string(),
            "1.0.0".to_string(),
            std::collections::HashMap::new(),
            VersionConstraint::any(),
        ),
        VersionConstraint::any(),
    ).unwrap();
    
    // Track package installation
    manager.handle_package_installed("test-env", package).await.unwrap();
}

#[tokio::test]
async fn test_multiple_environments() {
    let manager = HotReloadManager::new().await.unwrap();
    
    // Create multiple environments
    for i in 0..3 {
        let temp_dir = tempdir().unwrap();
        let env = PythonEnvironment::new(
            temp_dir.path().to_path_buf(),
            PythonVersion::default(),
        );
        manager.register_environment(env).await.unwrap();
    }
    
    // Start monitoring all environments
    manager.start_monitoring().await.unwrap();
}

#[tokio::test]
async fn test_import_notifications() {
    let manager = HotReloadManager::new().await.unwrap();
    let temp_dir = tempdir().unwrap();
    let env = PythonEnvironment::new(
        temp_dir.path().to_path_buf(),
        PythonVersion::default(),
    );
    
    manager.register_environment(env).await.unwrap();
    
    // Create test files with imports
    let test_files = vec![
        ("test1.py", "import numpy\n"),
        ("test2.py", "from pandas import DataFrame\n"),
        ("test3.py", "import tensorflow as tf\n"),
    ];
    
    for (name, content) in test_files {
        let file_path = temp_dir.path().join(name);
        std::fs::write(&file_path, content).unwrap();
        let imports = manager.analyze_file(&file_path).await.unwrap();
        assert!(!imports.is_empty());
    }
}

#[tokio::test]
async fn test_error_handling() {
    let manager = HotReloadManager::new().await.unwrap();
    
    // Test invalid file path
    let result = manager.analyze_file(PathBuf::from("/nonexistent/path.py").as_path()).await;
    assert!(result.is_err());
    
    // Test invalid environment registration
    let env = PythonEnvironment::new(
        PathBuf::from("/nonexistent"),
        PythonVersion::default(),
    );
    let result = manager.register_environment(env).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_python_file_changes() {
    let manager = HotReloadManager::new().await.unwrap();
    let temp_dir = tempdir().unwrap();
    let env = PythonEnvironment::new(
        temp_dir.path().to_path_buf(),
        PythonVersion::default(),
    );
    
    manager.register_environment(env).await.unwrap();
    
    // Create and modify Python files
    let test_file = temp_dir.path().join("test.py");
    
    // Initial content
    std::fs::write(&test_file, b"import numpy\n").unwrap();
    let imports = manager.analyze_file(&test_file).await.unwrap();
    assert!(imports.contains("numpy"));
    
    // Modified content
    std::fs::write(&test_file, b"import pandas\n").unwrap();
    let imports = manager.analyze_file(&test_file).await.unwrap();
    assert!(imports.contains("pandas"));
} 