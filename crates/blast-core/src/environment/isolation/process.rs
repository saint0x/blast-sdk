use std::collections::HashMap;
use std::path::PathBuf;
use crate::error::BlastResult;
use tokio::process::Command;
use tokio::sync::RwLock;
use std::sync::Arc;

/// Configuration for a process
#[derive(Debug, Clone)]
pub struct ProcessConfig {
    /// Command to run
    pub command: String,
    /// Command arguments
    pub args: Vec<String>,
    /// Working directory
    pub working_dir: PathBuf,
    /// Environment variables
    pub env: HashMap<String, String>,
}

impl Default for ProcessConfig {
    fn default() -> Self {
        Self {
            command: String::new(),
            args: Vec::new(),
            working_dir: PathBuf::from("."),
            env: HashMap::new(),
        }
    }
}

/// Handle to a running process
#[derive(Debug, Clone)]
pub struct ProcessHandle {
    /// Process ID
    pid: u32,
    /// Process state
    state: Arc<RwLock<ProcessState>>,
}

impl ProcessHandle {
    /// Get process ID
    pub fn pid(&self) -> u32 {
        self.pid
    }
}

/// Process state
#[derive(Debug, Clone)]
pub struct ProcessState {
    /// Whether process is running
    pub running: bool,
    /// Exit code if process has terminated
    pub exit_code: Option<i32>,
    /// Process start time
    pub start_time: std::time::SystemTime,
}

impl Default for ProcessState {
    fn default() -> Self {
        Self {
            running: false,
            exit_code: None,
            start_time: std::time::SystemTime::now(),
        }
    }
}

/// Process isolation trait
#[async_trait::async_trait]
pub trait ProcessIsolation: Send + Sync {
    /// Spawn a new process
    async fn spawn(&self, config: ProcessConfig) -> BlastResult<ProcessHandle>;
    
    /// Kill a process
    async fn kill(&self, handle: &ProcessHandle) -> BlastResult<()>;
    
    /// Get process state
    async fn get_state(&self, handle: &ProcessHandle) -> BlastResult<ProcessState>;
}

/// Process manager
#[derive(Debug)]
pub struct ProcessManager {
    /// Active processes
    processes: Arc<RwLock<HashMap<u32, ProcessHandle>>>,
}

impl ProcessManager {
    /// Create new process manager
    pub fn new() -> Self {
        Self {
            processes: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Spawn a new process
    pub async fn spawn(&self, config: ProcessConfig) -> BlastResult<ProcessHandle> {
        let mut command = Command::new(&config.command);
        command.args(&config.args);
        command.current_dir(&config.working_dir);
        command.envs(&config.env);
        
        let child = command.spawn()?;
        let pid = child.id().unwrap() as u32;
        
        let handle = ProcessHandle {
            pid,
            state: Arc::new(RwLock::new(ProcessState {
                running: true,
                exit_code: None,
                start_time: std::time::SystemTime::now(),
            })),
        };
        
        self.processes.write().await.insert(pid, handle.clone());
        Ok(handle)
    }
    
    /// Kill a process
    pub async fn kill(&self, handle: &ProcessHandle) -> BlastResult<()> {
        if let Some(state) = self.processes.write().await.get_mut(&handle.pid) {
            let mut state_lock = state.state.write().await;
            state_lock.running = false;
            state_lock.exit_code = Some(-1);
        }
        Ok(())
    }
    
    /// Get process state
    pub async fn get_state(&self, handle: &ProcessHandle) -> BlastResult<ProcessState> {
        if let Some(state) = self.processes.read().await.get(&handle.pid) {
            Ok(state.state.read().await.clone())
        } else {
            Ok(ProcessState::default())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::test;
    
    #[test]
    async fn test_process_spawn() {
        let config = ProcessConfig {
            command: "echo".into(),
            args: vec!["test".into()],
            working_dir: PathBuf::from("."),
            env: Default::default(),
        };
        
        let manager = ProcessManager::new();
        let handle = manager.spawn(config).await.unwrap();
        assert!(handle.pid() > 0);
    }
} 