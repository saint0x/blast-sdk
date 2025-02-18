use std::path::PathBuf;
use blast_core::error::{BlastResult, BlastError};
use crate::shell::Shell;
use tracing::{info, debug};

const SHELL_FUNCTION: &str = r#"
# Blast environment manager shell integration
blast() {
    eval "$(blast-cli start "$@")"
}
"#;

pub struct ShellSetup {
    shell: Shell,
    config_file: PathBuf,
}

impl ShellSetup {
    pub fn new() -> BlastResult<Self> {
        let shell = Shell::detect();
        let home_dir = dirs::home_dir().ok_or_else(|| {
            BlastError::Config("Could not determine home directory".to_string())
        })?;

        let config_file = match shell {
            Shell::Bash => home_dir.join(".bashrc"),
            Shell::Zsh => home_dir.join(".zshrc"),
            Shell::Fish => home_dir.join(".config").join("fish").join("config.fish"),
            Shell::PowerShell => home_dir.join("Documents")
                .join("WindowsPowerShell")
                .join("Microsoft.PowerShell_profile.ps1"),
            Shell::Unknown => return Err(BlastError::Config("Unsupported shell".to_string())),
        };

        Ok(Self {
            shell,
            config_file,
        })
    }

    pub fn ensure_shell_integration(&self) -> BlastResult<bool> {
        // Check if integration already exists
        if self.is_already_configured()? {
            debug!("Shell integration already configured");
            return Ok(false);
        }

        // Add shell integration
        self.add_shell_integration()?;
        info!("Added blast shell integration to {}", self.config_file.display());
        
        Ok(true)
    }

    fn is_already_configured(&self) -> BlastResult<bool> {
        if !self.config_file.exists() {
            return Ok(false);
        }

        let content = std::fs::read_to_string(&self.config_file)?;
        Ok(content.contains("blast() {") || content.contains("function blast"))
    }

    fn add_shell_integration(&self) -> BlastResult<()> {
        let mut content = String::new();
        
        if self.config_file.exists() {
            content = std::fs::read_to_string(&self.config_file)?;
            if !content.ends_with('\n') {
                content.push('\n');
            }
        }

        // Create parent directories if they don't exist
        if let Some(parent) = self.config_file.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Add our shell function
        content.push_str(SHELL_FUNCTION);
        std::fs::write(&self.config_file, content)?;

        Ok(())
    }

    pub fn get_shell_reload_command(&self) -> &'static str {
        match self.shell {
            Shell::Bash => "source ~/.bashrc",
            Shell::Zsh => "source ~/.zshrc",
            Shell::Fish => "source ~/.config/fish/config.fish",
            Shell::PowerShell => ". $PROFILE",
            Shell::Unknown => "",
        }
    }
}

/// Initialize blast for first use
pub fn initialize() -> BlastResult<()> {
    let setup = ShellSetup::new()?;
    
    if setup.ensure_shell_integration()? {
        info!("Blast shell integration has been configured.");
        info!("To activate the changes, please run:");
        info!("    {}", setup.get_shell_reload_command());
    }

    Ok(())
} 