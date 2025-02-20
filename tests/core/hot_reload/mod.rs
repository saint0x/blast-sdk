use blast_core::{
    python::PythonVersion,
    python::PythonEnvironment,
    hot_reload::{
        HotReloadConfig,
        HotReloadManager,
        ImportStatement,
    },
};
use tempfile::TempDir;

#[tokio::test]
async fn test_parse_simple_import() {
    let imports = ImportStatement::parse_from_line("import numpy");
    assert_eq!(imports.len(), 1);
    assert_eq!(imports[0].module, "numpy");
    assert!(!imports[0].is_from);
}

#[tokio::test]
async fn test_parse_multiple_imports() {
    let imports = ImportStatement::parse_from_line("import numpy, pandas, tensorflow");
    assert_eq!(imports.len(), 3);
    assert_eq!(imports[0].module, "numpy");
    assert_eq!(imports[1].module, "pandas");
    assert_eq!(imports[2].module, "tensorflow");
}

#[tokio::test]
async fn test_parse_from_import() {
    let imports = ImportStatement::parse_from_line("from numpy import array, zeros");
    assert_eq!(imports.len(), 1);
    assert_eq!(imports[0].module, "array");
    assert!(imports[0].is_from);
    assert_eq!(imports[0].names, vec!["array", "zeros"]);
    assert_eq!(imports[0].from_path, Some("numpy".to_string()));
}

#[tokio::test]
async fn test_parse_import_with_alias() {
    let imports = ImportStatement::parse_from_line("import numpy as np");
    assert_eq!(imports.len(), 1);
    assert_eq!(imports[0].module, "numpy");
}

#[tokio::test]
async fn test_parse_nested_import() {
    let imports = ImportStatement::parse_from_line("from tensorflow.keras import layers");
    assert_eq!(imports.len(), 1);
    assert_eq!(imports[0].get_package_name(), "tensorflow");
}

#[tokio::test]
async fn test_hot_reload() {
    let temp_dir = TempDir::new().unwrap();
    let version = PythonVersion::new(3, 9, Some(0));
    
    let env = PythonEnvironment::new(
        "test-env".to_string(),
        temp_dir.path().to_path_buf(),
        version,
    ).await.unwrap();

    let config = HotReloadConfig::default();
    let manager = HotReloadManager::new(env, config);

    // Test update processing
    manager.process_updates().await.unwrap();

    // Test getting updates
    let updates = manager.get_pending_updates().await.unwrap();
    assert!(updates.is_empty());
}

#[tokio::test]
async fn test_python_version() {
    let version = PythonVersion::new(3, 9, Some(0));
    assert_eq!(version.major(), 3);
    assert_eq!(version.minor(), 9);
    assert_eq!(version.patch(), Some(0));
} 