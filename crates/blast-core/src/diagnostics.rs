use std::fmt;
use std::path::PathBuf;
use serde::{Serialize, Deserialize};

/// Diagnostic severity levels
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiagnosticLevel {
    /// Critical errors that prevent operation
    Error,
    /// Important issues that don't prevent operation
    Warning,
    /// Informational messages about operations
    Info,
    /// Detailed debug information
    Debug,
    /// Very detailed trace information
    Trace,
}

/// Categories of diagnostics for better organization
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiagnosticCategory {
    /// Version-related issues
    Version,
    /// Dependency resolution problems
    Dependency,
    /// Transaction and rollback issues
    Transaction,
    /// Environment state issues
    Environment,
    /// Package management issues
    Package,
    /// Performance-related issues
    Performance,
    /// Security-related issues
    Security,
    /// Configuration problems
    Configuration,
    /// Custom category with string identifier
    Custom(String),
}

impl fmt::Display for DiagnosticCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Version => write!(f, "Version"),
            Self::Dependency => write!(f, "Dependency"),
            Self::Transaction => write!(f, "Transaction"),
            Self::Environment => write!(f, "Environment"),
            Self::Package => write!(f, "Package"),
            Self::Performance => write!(f, "Performance"),
            Self::Security => write!(f, "Security"),
            Self::Configuration => write!(f, "Configuration"),
            Self::Custom(s) => write!(f, "{}", s),
        }
    }
}

/// Code context for diagnostics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeContext {
    /// File where the issue occurred
    pub file: PathBuf,
    /// Line number in the file
    pub line: Option<u32>,
    /// Column number in the file
    pub column: Option<u32>,
    /// Relevant code snippet
    pub snippet: Option<String>,
    /// Function or method name
    pub function: Option<String>,
}

/// Suggestion for resolving diagnostic issues
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticSuggestion {
    /// Description of the suggestion
    pub description: String,
    /// Code or command to fix the issue
    pub fix: Option<String>,
    /// Additional context about the fix
    pub context: Option<String>,
    /// Estimated time to implement fix
    pub estimated_time: Option<String>,
    /// Whether the fix can be applied automatically
    pub auto_fixable: bool,
}

/// Diagnostic information for operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    /// Severity level of the diagnostic
    pub level: DiagnosticLevel,
    /// Main error message or description
    pub message: String,
    /// Category of the diagnostic
    pub category: DiagnosticCategory,
    /// Detailed explanation of the issue
    pub details: Option<String>,
    /// Suggestions for resolving the issue
    pub suggestions: Vec<DiagnosticSuggestion>,
    /// Code context if available
    pub code_context: Option<CodeContext>,
    /// Operation context (e.g. package name, version)
    pub operation_context: Option<String>,
    /// Timestamp when the diagnostic was created
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Unique identifier for the diagnostic
    pub id: uuid::Uuid,
}

impl Diagnostic {
    /// Create a new diagnostic
    pub fn new(
        level: DiagnosticLevel,
        message: String,
        category: DiagnosticCategory,
    ) -> Self {
        Self {
            level,
            message,
            category,
            details: None,
            suggestions: Vec::new(),
            code_context: None,
            operation_context: None,
            timestamp: chrono::Utc::now(),
            id: uuid::Uuid::new_v4(),
        }
    }

    /// Add detailed explanation
    pub fn with_details(mut self, details: String) -> Self {
        self.details = Some(details);
        self
    }

    /// Add a suggestion
    pub fn with_suggestion(mut self, suggestion: DiagnosticSuggestion) -> Self {
        self.suggestions.push(suggestion);
        self
    }

    /// Add code context
    pub fn with_code_context(mut self, context: CodeContext) -> Self {
        self.code_context = Some(context);
        self
    }

    /// Add operation context
    pub fn with_operation_context(mut self, context: String) -> Self {
        self.operation_context = Some(context);
        self
    }
}

/// Collection of diagnostics with query capabilities
#[derive(Debug, Default)]
pub struct DiagnosticCollection {
    diagnostics: Vec<Diagnostic>,
}

impl DiagnosticCollection {
    /// Create a new empty collection
    pub fn new() -> Self {
        Self {
            diagnostics: Vec::new(),
        }
    }

    /// Add a diagnostic to the collection
    pub fn add(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }

    /// Get all diagnostics
    pub fn all(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Get diagnostics by level
    pub fn by_level(&self, level: DiagnosticLevel) -> Vec<&Diagnostic> {
        self.diagnostics
            .iter()
            .filter(|d| d.level == level)
            .collect()
    }

    /// Get diagnostics by category
    pub fn by_category(&self, category: &DiagnosticCategory) -> Vec<&Diagnostic> {
        self.diagnostics
            .iter()
            .filter(|d| d.category == *category)
            .collect()
    }

    /// Get diagnostics within a time range
    pub fn in_time_range(
        &self,
        start: chrono::DateTime<chrono::Utc>,
        end: chrono::DateTime<chrono::Utc>,
    ) -> Vec<&Diagnostic> {
        self.diagnostics
            .iter()
            .filter(|d| d.timestamp >= start && d.timestamp <= end)
            .collect()
    }

    /// Clear all diagnostics
    pub fn clear(&mut self) {
        self.diagnostics.clear();
    }
} 