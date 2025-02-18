use std::env;
use std::path::PathBuf;
use blast_core::{
    error::BlastResult,
    shell_scripts::ActivationScripts,
};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellState {
    original_path: Option<String>,
    original_pythonpath: Option<String>,
    original_prompt: Option<String>,
    active_env_name: Option<String>,
    active_env_path: Option<PathBuf>,
    socket_path: Option<String>,
}

impl ShellState {
    pub fn new() -> Self {
        Self {
            original_path: env::var("PATH").ok(),
            original_pythonpath: env::var("PYTHONPATH").ok(),
            original_prompt: env::var("PS1").or_else(|_| env::var("PROMPT")).ok(),
            active_env_name: None,
            active_env_path: None,
            socket_path: None,
        }
    }

    pub fn with_environment(mut self, env_name: String, env_path: PathBuf) -> Self {
        self.active_env_name = Some(env_name.clone());
        self.active_env_path = Some(env_path);
        self.socket_path = Some(format!("/tmp/blast_{}.sock", env_name));
        self
    }

    pub fn save(&self, state_path: &PathBuf) -> BlastResult<()> {
        let state_str = serde_json::to_string(self)?;
        std::fs::write(state_path, state_str)?;
        Ok(())
    }

    pub fn load(state_path: &PathBuf) -> BlastResult<Self> {
        let state_str = std::fs::read_to_string(state_path)?;
        let state = serde_json::from_str(&state_str)?;
        Ok(state)
    }
}

#[derive(Debug, Clone)]
pub enum Shell {
    Bash,
    Zsh,
    Fish,
    PowerShell,
    Unknown,
}

impl Shell {
    pub fn detect() -> Self {
        let shell_path = env::var("SHELL").unwrap_or_default();
        match shell_path.split('/').last().unwrap_or("") {
            "bash" => Shell::Bash,
            "zsh" => Shell::Zsh,
            "fish" => Shell::Fish,
            "pwsh" | "powershell" => Shell::PowerShell,
            _ => Shell::Unknown,
        }
    }

    pub fn get_activation_command(&self, env_path: &PathBuf) -> String {
        let bin_dir = env_path.join("bin");
        match self {
            Shell::Bash | Shell::Zsh => {
                format!("source {}/activate", bin_dir.display())
            }
            Shell::Fish => {
                format!("source {}/activate.fish", bin_dir.display())
            }
            Shell::PowerShell => {
                format!(". {}/activate.ps1", bin_dir.display())
            }
            Shell::Unknown => {
                format!("echo 'Unsupported shell detected. Please use bash, zsh, fish, or powershell.'")
            }
        }
    }

    pub fn get_deactivation_command(&self) -> &'static str {
        match self {
            Shell::Bash | Shell::Zsh | Shell::Fish => "deactivate",
            Shell::PowerShell => "deactivate",
            Shell::Unknown => "echo 'Unsupported shell detected.'",
        }
    }
}

pub struct EnvironmentActivator {
    shell: Shell,
    env_path: PathBuf,
    env_name: String,
    state: ShellState,
}

impl EnvironmentActivator {
    pub fn new(env_path: PathBuf, env_name: String) -> Self {
        let shell = Shell::detect();
        let state = ShellState::new().with_environment(env_name.clone(), env_path.clone());
        
        Self {
            shell,
            env_path,
            env_name,
            state,
        }
    }

    pub fn generate_activation_script(&self) -> String {
        let scripts = ActivationScripts::generate(&self.env_path, &self.env_name);
        match self.shell {
            Shell::Bash | Shell::Zsh => scripts.bash,
            Shell::Fish => scripts.fish,
            Shell::PowerShell => scripts.powershell,
            Shell::Unknown => "echo 'Unsupported shell detected. Please use bash, zsh, fish, or powershell.'".to_string(),
        }
    }

    pub fn generate_deactivation_script(&self) -> String {
        self.shell.get_deactivation_command().to_string()
    }

    pub fn save_state(&self) -> BlastResult<()> {
        let state_dir = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("blast");
        
        std::fs::create_dir_all(&state_dir)?;
        
        let state_path = state_dir.join(format!("{}.json", self.env_name));
        self.state.save(&state_path)?;
        
        // Create daemon socket directory if it doesn't exist
        if let Some(socket_path) = self.get_socket_path() {
            if let Some(parent) = PathBuf::from(socket_path).parent() {
                std::fs::create_dir_all(parent)?;
            }
        }
        
        Ok(())
    }

    pub fn get_socket_path(&self) -> Option<&str> {
        self.state.socket_path.as_deref()
    }

    pub fn is_daemon_running(&self) -> bool {
        if let Some(socket_path) = self.get_socket_path() {
            std::path::Path::new(socket_path).exists()
        } else {
            false
        }
    }

    pub fn ensure_daemon_running(&self) -> bool {
        self.is_daemon_running()
    }

    pub fn env_path(&self) -> &PathBuf {
        &self.env_path
    }

    pub fn env_name(&self) -> &str {
        &self.env_name
    }

    pub fn shell(&self) -> &Shell {
        &self.shell
    }
} 