use blast_core::error::BlastError;
use glob;

#[test]
fn test_error_creation() {
    let err = BlastError::python("Python error");
    assert!(matches!(err, BlastError::Python(_)));

    let err = BlastError::package("Package error");
    assert!(matches!(err, BlastError::Package(_)));

    let err = BlastError::environment("Environment error");
    assert!(matches!(err, BlastError::Environment(_)));
}

#[test]
fn test_error_conversion() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
    let err: BlastError = io_err.into();
    assert!(matches!(err, BlastError::Io(_)));

    let pattern_err = glob::Pattern::new("[").unwrap_err();
    let err: BlastError = pattern_err.into();
    assert!(matches!(err, BlastError::Pattern(_)));
}

#[test]
fn test_error_display() {
    let err = BlastError::python("test error");
    assert_eq!(err.to_string(), "Python error: test error");

    let err = BlastError::package("test error");
    assert_eq!(err.to_string(), "Package error: test error");
} 