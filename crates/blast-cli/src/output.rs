//! Output formatting utilities for CLI

use console::style;
use blast_core::package::{Package, Version};

/// Format a package for display
pub fn format_package(package: &Package) -> String {
    format!(
        "{} {}",
        style(package.name()).green(),
        style(package.version()).yellow()
    )
}

/// Format a version for display
pub fn format_version(version: &Version) -> String {
    style(version.to_string()).yellow().to_string()
}

/// Format a dependency tree
pub fn format_dependency_tree(package: &Package, depth: usize) -> String {
    let mut output = String::new();
    let indent = "  ".repeat(depth);
    
    output.push_str(&format!(
        "{}{}",
        indent,
        format_package(package)
    ));

    for (name, constraint) in package.dependencies() {
        output.push_str(&format!(
            "\n{}└── {} {}",
            indent,
            style(name).blue(),
            style(constraint).dim()
        ));
    }

    output
}

/// Format an error message
pub fn format_error(msg: &str) -> String {
    style(format!("Error: {}", msg)).red().to_string()
}

/// Format a success message
pub fn format_success(msg: &str) -> String {
    style(format!("Success: {}", msg)).green().to_string()
}

/// Format a warning message
pub fn format_warning(msg: &str) -> String {
    style(format!("Warning: {}", msg)).yellow().to_string()
}

/// Format an info message
pub fn format_info(msg: &str) -> String {
    style(msg).blue().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use blast_core::package::{PackageId, VersionConstraint};
    use std::collections::HashMap;

    #[test]
    fn test_format_package() {
        let package = Package::new(
            PackageId::new("test-package", Version::parse("1.0.0").unwrap()),
            HashMap::new(),
            VersionConstraint::parse(">=3.7").unwrap(),
        );
        let formatted = format_package(&package);
        assert!(formatted.contains("test-package"));
        assert!(formatted.contains("1.0.0"));
    }

    #[test]
    fn test_format_dependency_tree() {
        let mut deps = HashMap::new();
        deps.insert(
            "dep1".to_string(),
            VersionConstraint::parse(">=1.0.0").unwrap(),
        );
        deps.insert(
            "dep2".to_string(),
            VersionConstraint::parse(">=2.0.0").unwrap(),
        );

        let package = Package::new(
            PackageId::new("root", Version::parse("1.0.0").unwrap()),
            deps,
            VersionConstraint::parse(">=3.7").unwrap(),
        );

        let tree = format_dependency_tree(&package, 0);
        assert!(tree.contains("root"));
        assert!(tree.contains("dep1"));
        assert!(tree.contains("dep2"));
    }

    #[test]
    fn test_format_messages() {
        let error = format_error("test error");
        assert!(error.contains("Error: test error"));

        let success = format_success("test success");
        assert!(success.contains("Success: test success"));

        let warning = format_warning("test warning");
        assert!(warning.contains("Warning: test warning"));

        let info = format_info("test info");
        assert!(info.contains("test info"));
    }
} 