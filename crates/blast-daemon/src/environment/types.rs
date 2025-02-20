use std::path::PathBuf;
use std::time::SystemTime;
use blast_core::environment::Environment as CoreEnvironment;

#[derive(Debug, Clone)]
pub struct DaemonEnvironment {
    pub name: String,
    pub python_version: String,
    pub path: PathBuf,
    pub last_accessed: SystemTime,
    pub active: bool,
}

impl From<Box<dyn CoreEnvironment>> for DaemonEnvironment {
    fn from(env: Box<dyn CoreEnvironment>) -> Self {
        Self {
            name: env.name().to_string(),
            python_version: env.python_version().to_string(),
            path: env.path().to_path_buf(),
            last_accessed: SystemTime::now(),
            active: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct EnvironmentImage {
    pub name: String,
    pub python_version: String,
    pub created: chrono::DateTime<chrono::Utc>,
} 