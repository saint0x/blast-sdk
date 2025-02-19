use std::path::PathBuf;
use blast_core::error::{BlastResult, BlastError};
use crate::shell::Shell;
use tracing::{info, debug};

const BASH_ZSH_FUNCTION: &str = r#"
# Blast environment manager shell integration
blast() {
    if [ "$1" = "start" ]; then
        # Get the actual blast binary path
        local blast_bin
        if ! blast_bin=$(command -v blast); then
            echo "Error: blast binary not found in PATH" >&2
            return 1
        fi

        local temp_file
        temp_file=$(mktemp)
        if "$blast_bin" "$@" > "$temp_file"; then
            . "$temp_file"
            rm -f "$temp_file"
        else
            rm -f "$temp_file"
            return 1
        fi
    else
        command blast "$@"
    fi
}
"#;

const FISH_FUNCTION: &str = r#"
# Blast environment manager shell integration
function blast
    if test "$argv[1]" = "start"
        # Get the actual blast binary path
        set -l blast_bin (command -s blast)
        if test -z "$blast_bin"
            echo "Error: blast binary not found in PATH" >&2
            return 1
        end

        set -l temp_file (mktemp)
        if "$blast_bin" $argv > $temp_file
            source $temp_file
            rm -f $temp_file
        else
            rm -f $temp_file
            return 1
        end
    else
        command blast $argv
    end
end
"#;

const POWERSHELL_FUNCTION: &str = r#"
# Blast environment manager shell integration
function blast {
    if ($args[0] -eq "start") {
        # Get the actual blast binary path
        $blast_bin = Get-Command blast -ErrorAction SilentlyContinue
        if (-not $blast_bin) {
            Write-Error "Error: blast binary not found in PATH"
            return $false
        }

        $temp_file = [System.IO.Path]::GetTempFileName()
        if (& $blast_bin.Path @args > $temp_file) {
            . $temp_file
            Remove-Item $temp_file
        } else {
            Remove-Item $temp_file
            return $false
        }
    } else {
        & (Get-Command blast).Path @args
    }
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
        info!("To complete setup, please either:");
        info!("    1. Restart your shell, or");
        info!("    2. Run: {}", self.get_shell_reload_command());
        
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

        // Add shell-specific function
        let shell_function = match self.shell {
            Shell::Bash | Shell::Zsh => BASH_ZSH_FUNCTION,
            Shell::Fish => FISH_FUNCTION,
            Shell::PowerShell => POWERSHELL_FUNCTION,
            Shell::Unknown => return Err(BlastError::Config("Unsupported shell".to_string())),
        };

        content.push_str(shell_function);
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
        info!("To complete setup, please either:");
        info!("    1. Restart your shell, or");
        info!("    2. Run: {}", setup.get_shell_reload_command());
    }

    Ok(())
} 