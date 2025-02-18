//! Output formatting utilities for CLI

use console::style;
use blast_core::{
    package::Package,
    version::Version,
};

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

    for (name, constraint) in package.metadata().dependencies.iter() {
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