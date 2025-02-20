use regex::Regex;

/// Represents a Python import statement
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportStatement {
    /// The module being imported
    pub module: String,
    /// Specific names being imported (for 'from' imports)
    pub names: Vec<String>,
    /// Whether this is a 'from' import
    pub is_from: bool,
    /// The full module path for 'from' imports
    pub from_path: Option<String>,
}

impl ImportStatement {
    pub fn new(module: String) -> Self {
        Self {
            module,
            names: Vec::new(),
            is_from: false,
            from_path: None,
        }
    }

    pub fn with_names(module: String, names: Vec<String>, from_path: String) -> Self {
        Self {
            module,
            names,
            is_from: true,
            from_path: Some(from_path),
        }
    }

    /// Get the root package name that needs to be installed
    pub fn get_package_name(&self) -> String {
        if self.is_from {
            // For 'from' imports, use the first part of the path
            self.from_path.as_ref()
                .and_then(|p| p.split('.').next())
                .unwrap_or(&self.module)
                .to_string()
        } else {
            // For regular imports, use the first part of the module name
            self.module.split('.').next()
                .unwrap_or(&self.module)
                .to_string()
        }
    }

    /// Parse Python imports from a line of code
    pub fn parse_from_line(line: &str) -> Vec<Self> {
        let mut imports = Vec::new();
        let line = line.trim();

        // Skip comments and empty lines
        if line.starts_with('#') || line.is_empty() {
            return imports;
        }

        // Handle multiline imports with parentheses
        if line.contains('(') && !line.contains(')') {
            // This is a multiline import - it should be handled by the caller
            return imports;
        }

        // Match 'from ... import ...' statements
        if line.starts_with("from ") {
            let from_re = Regex::new(r"^from\s+([.\w]+)\s+import\s+(.+)$").unwrap();
            if let Some(caps) = from_re.captures(line) {
                let from_path = caps.get(1).unwrap().as_str().to_string();
                let imports_str = caps.get(2).unwrap().as_str();

                // Handle multiple imports
                let names: Vec<String> = imports_str
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();

                if !names.is_empty() {
                    imports.push(ImportStatement::with_names(
                        names[0].clone(),
                        names,
                        from_path,
                    ));
                }
            }
        }
        // Match 'import ...' statements
        else if line.starts_with("import ") {
            let import_re = Regex::new(r"^import\s+(.+)$").unwrap();
            if let Some(caps) = import_re.captures(line) {
                let modules = caps.get(1).unwrap().as_str();
                
                // Handle multiple imports and aliases
                for module in modules.split(',') {
                    let module = module.trim();
                    if module.is_empty() {
                        continue;
                    }

                    // Handle 'as' aliases
                    let module_name = module.split_whitespace()
                        .next()
                        .unwrap_or(module)
                        .to_string();

                    imports.push(ImportStatement::new(module_name));
                }
            }
        }

        imports
    }
} 