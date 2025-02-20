/// Environment metadata
#[derive(Debug, Clone)]
pub struct EnvironmentMetadata {
    /// Environment name
    pub name: Option<String>,
    /// Environment description
    pub description: Option<String>,
    /// Custom tags
    pub tags: Vec<String>,
    /// Additional metadata
    pub extra: serde_json::Value,
}

impl Default for EnvironmentMetadata {
    fn default() -> Self {
        Self {
            name: None,
            description: None,
            tags: Vec::new(),
            extra: serde_json::Value::Object(serde_json::Map::new()),
        }
    }
} 