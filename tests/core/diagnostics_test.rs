use std::path::PathBuf;
use chrono::{TimeZone, Utc};
use blast_core::diagnostics::{
    Diagnostic, DiagnosticLevel, DiagnosticCategory, DiagnosticCollection,
    DiagnosticSuggestion, CodeContext,
};

#[test]
fn test_diagnostic_creation() {
    let diagnostic = Diagnostic::new(
        DiagnosticLevel::Error,
        "Package installation failed".to_string(),
        DiagnosticCategory::Package,
    );

    assert_eq!(diagnostic.level, DiagnosticLevel::Error);
    assert_eq!(diagnostic.message, "Package installation failed");
    assert_eq!(diagnostic.category, DiagnosticCategory::Package);
    assert!(diagnostic.details.is_none());
    assert!(diagnostic.suggestions.is_empty());
}

#[test]
fn test_diagnostic_builder_methods() {
    let suggestion = DiagnosticSuggestion {
        description: "Try updating pip".to_string(),
        fix: Some("python -m pip install --upgrade pip".to_string()),
        context: Some("Outdated pip version".to_string()),
        estimated_time: Some("1 minute".to_string()),
        auto_fixable: true,
    };

    let code_context = CodeContext {
        file: PathBuf::from("requirements.txt"),
        line: Some(10),
        column: Some(5),
        snippet: Some("pandas==1.2.3".to_string()),
        function: None,
    };

    let diagnostic = Diagnostic::new(
        DiagnosticLevel::Warning,
        "Outdated dependency".to_string(),
        DiagnosticCategory::Dependency,
    )
    .with_details("Package version is outdated".to_string())
    .with_suggestion(suggestion)
    .with_code_context(code_context)
    .with_operation_context("pip install pandas".to_string());

    assert!(diagnostic.details.is_some());
    assert_eq!(diagnostic.suggestions.len(), 1);
    assert!(diagnostic.code_context.is_some());
    assert!(diagnostic.operation_context.is_some());
}

#[test]
fn test_diagnostic_collection() {
    let mut collection = DiagnosticCollection::new();
    
    // Add diagnostics with different levels and categories
    let error_diagnostic = Diagnostic::new(
        DiagnosticLevel::Error,
        "Critical error".to_string(),
        DiagnosticCategory::Security,
    );
    
    let warning_diagnostic = Diagnostic::new(
        DiagnosticLevel::Warning,
        "Performance warning".to_string(),
        DiagnosticCategory::Performance,
    );
    
    collection.add(error_diagnostic);
    collection.add(warning_diagnostic);
    
    // Test filtering by level
    let errors = collection.by_level(DiagnosticLevel::Error);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].message, "Critical error");
    
    // Test filtering by category
    let performance_issues = collection.by_category(&DiagnosticCategory::Performance);
    assert_eq!(performance_issues.len(), 1);
    assert_eq!(performance_issues[0].message, "Performance warning");
}

#[test]
fn test_time_range_filtering() {
    let mut collection = DiagnosticCollection::new();
    
    // Create diagnostics with specific timestamps
    let mut old_diagnostic = Diagnostic::new(
        DiagnosticLevel::Info,
        "Old message".to_string(),
        DiagnosticCategory::Version,
    );
    old_diagnostic.timestamp = Utc.timestamp_opt(1600000000, 0).unwrap();
    
    let mut new_diagnostic = Diagnostic::new(
        DiagnosticLevel::Info,
        "New message".to_string(),
        DiagnosticCategory::Version,
    );
    new_diagnostic.timestamp = Utc.timestamp_opt(1700000000, 0).unwrap();
    
    collection.add(old_diagnostic);
    collection.add(new_diagnostic);
    
    // Test time range filtering
    let range_start = Utc.timestamp_opt(1650000000, 0).unwrap();
    let range_end = Utc.timestamp_opt(1750000000, 0).unwrap();
    let filtered = collection.in_time_range(range_start, range_end);
    
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].message, "New message");
}

#[test]
fn test_diagnostic_category_display() {
    assert_eq!(DiagnosticCategory::Version.to_string(), "Version");
    assert_eq!(DiagnosticCategory::Package.to_string(), "Package");
    assert_eq!(DiagnosticCategory::Custom("Test".to_string()).to_string(), "Test");
}

#[test]
fn test_collection_clear() {
    let mut collection = DiagnosticCollection::new();
    
    collection.add(Diagnostic::new(
        DiagnosticLevel::Error,
        "Test error".to_string(),
        DiagnosticCategory::Configuration,
    ));
    
    assert_eq!(collection.all().len(), 1);
    collection.clear();
    assert_eq!(collection.all().len(), 0);
} 