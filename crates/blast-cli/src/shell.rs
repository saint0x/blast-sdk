use std::env;
use std::path::PathBuf;
use blast_core::{
    error::BlastResult,
    shell_scripts::ActivationScripts,
};
use serde::{Serialize, Deserialize};
use std::io::Write;

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
        // Try SHELL env var first
        if let Ok(shell_path) = env::var("SHELL") {
            let shell_name = shell_path.split('/').last().unwrap_or("");
            match shell_name {
                "bash" => return Shell::Bash,
                "zsh" => return Shell::Zsh,
                "fish" => return Shell::Fish,
                "pwsh" | "powershell" => return Shell::PowerShell,
                _ => {}
            }
        }

        // Try BASH_VERSION env var
        if env::var("BASH_VERSION").is_ok() {
            return Shell::Bash;
        }

        // Try ZSH_VERSION env var
        if env::var("ZSH_VERSION").is_ok() {
            return Shell::Zsh;
        }

        // On macOS, check common shell paths
        #[cfg(target_os = "macos")]
        {
            if std::path::Path::new("/bin/zsh").exists() {
                return Shell::Zsh;
            }
            if std::path::Path::new("/bin/bash").exists() {
                return Shell::Bash;
            }
        }

        // On Unix systems, try checking parent process name
        #[cfg(unix)]
        if let Ok(ppid) = std::fs::read_to_string("/proc/self/ppid") {
            if let Ok(cmdline) = std::fs::read_to_string(format!("/proc/{}/cmdline", ppid.trim())) {
                let cmd = cmdline.split('\0').next().unwrap_or("");
                match cmd {
                    cmd if cmd.contains("bash") => return Shell::Bash,
                    cmd if cmd.contains("zsh") => return Shell::Zsh,
                    cmd if cmd.contains("fish") => return Shell::Fish,
                    cmd if cmd.contains("pwsh") || cmd.contains("powershell") => return Shell::PowerShell,
                    _ => {}
                }
            }
        }

        // Default to zsh on macOS (since it's the default shell), bash otherwise
        #[cfg(target_os = "macos")]
        return Shell::Zsh;
        #[cfg(not(target_os = "macos"))]
        return Shell::Bash;
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
        // Temporarily disable colored output for activation script
        std::env::set_var("NO_COLOR", "1");
        let scripts = ActivationScripts::generate(&self.env_path, &self.env_name);
        std::env::remove_var("NO_COLOR");
        
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

    pub fn generate_shell_function(&self) -> String {
        match self.shell {
            Shell::Bash | Shell::Zsh => r#"
# Blast environment manager integration
blast() {
    if [ "$1" = "start" ]; then
        # Create a unique temporary file based on PID
        local temp_file="/tmp/blast_${$}_$(date +%s)"
        
        # Run blast and capture output
        if command blast "$@" > "$temp_file" 2>&1; then
            # Source the file if it contains activation script
            if grep -q "BLAST_ENV_NAME" "$temp_file" && grep -q "deactivate()" "$temp_file"; then
                . "$temp_file"
                rm -f "$temp_file"
                return 0
            else
                cat "$temp_file"
                rm -f "$temp_file"
                return 1
            fi
        else
            cat "$temp_file"
            rm -f "$temp_file"
            return 1
        fi
    else
        command blast "$@"
    fi
}
"#.trim().to_string(),
            Shell::Fish => r#"
function blast
    # Prevent recursion
    if set -q BLAST_RECURSION_CHECK
        echo "Error: Detected recursive blast activation" >&2
        return 1
    end
    set -x BLAST_RECURSION_CHECK 1

    # Safety mechanism: limit subshell depth
    set -q SHLVL; or set SHLVL 1
    if test $SHLVL -gt 20
        echo "Error: Shell nesting level too deep ($SHLVL)" >&2
        set -e BLAST_RECURSION_CHECK
        return 1
    end

    if test "$argv[1]" = "start"
        # Get blast binary path
        set -l blast_bin (command -v blast)
        if test $status -ne 0
            echo "Error: blast binary not found in PATH" >&2
            set -e BLAST_RECURSION_CHECK
            return 1
        end

        # Create temp files with cleanup
        set -l temp_file (mktemp)
        set -l log_file (mktemp)
        
        function cleanup --on-event fish_exit
            rm -f "$temp_file" "$log_file"
            set -e BLAST_RECURSION_CHECK
        end

        # Safety mechanism: timeout
        if command -v timeout >/dev/null 2>&1
            if not timeout 30s $blast_bin $argv > $temp_file 2> $log_file
                if test $status -eq 124
                    echo "Error: Command timed out after 30 seconds" >&2
                end
                cat $log_file >&2
                cleanup
                return 1
            end
        else
            if not $blast_bin $argv > $temp_file 2> $log_file
                cat $log_file >&2
                cleanup
                return 1
            end
        end

        # Verify activation script
        if grep -q "BLAST_ENV_NAME" $temp_file; and grep -q "deactivate" $temp_file
            # Safety mechanism: check script size
            set -l script_size (wc -l < $temp_file)
            if test $script_size -gt 1000
                echo "Error: Activation script too large ($script_size lines)" >&2
                cleanup
                return 1
            end

            source $temp_file
            set -l exit_code $status
            
            if test -s "$log_file"
                cat "$log_file" >&2
            end
            
            cleanup
            return $exit_code
        else
            cat "$log_file" >&2
            cat "$temp_file" >&2
            cleanup
            return 1
        end
    else
        command blast $argv
        set -l exit_code $status
        set -e BLAST_RECURSION_CHECK
        return $exit_code
    end
end
"#.trim().to_string(),
            Shell::PowerShell => r#"
function blast {
    param([Parameter(ValueFromRemainingArguments=$true)]$args)
    
    # Prevent recursion
    if ($env:BLAST_RECURSION_CHECK) {
        Write-Error "Error: Detected recursive blast activation"
        return 1
    }
    $env:BLAST_RECURSION_CHECK = 1

    # Safety mechanism: check call stack depth
    if ((Get-PSCallStack).Count -gt 20) {
        Write-Error "Error: PowerShell call stack too deep"
        $env:BLAST_RECURSION_CHECK = $null
        return 1
    }

    try {
        if ($args[0] -eq "start") {
            # Get blast binary path
            $blast_bin = Get-Command blast -ErrorAction SilentlyContinue
            if (-not $blast_bin) {
                Write-Error "Error: blast binary not found in PATH"
                return 1
            }

            # Create temp files
            $temp_file = [System.IO.Path]::GetTempFileName()
            $log_file = [System.IO.Path]::GetTempFileName()

            try {
                # Safety mechanism: timeout
                $timeoutSeconds = 30
                $processStartTime = Get-Date
                
                $process = Start-Process -FilePath $blast_bin.Path -ArgumentList $args `
                    -RedirectStandardOutput $temp_file -RedirectStandardError $log_file `
                    -NoNewWindow -PassThru

                # Wait with timeout
                if (-not $process.WaitForExit($timeoutSeconds * 1000)) {
                    $process.Kill()
                    Write-Error "Error: Command timed out after $timeoutSeconds seconds"
                    return 1
                }

                if ($process.ExitCode -eq 0) {
                    # Verify activation script
                    $content = Get-Content $temp_file -Raw
                    if ($content -match "BLAST_ENV_NAME" -and $content -match "deactivate") {
                        # Safety mechanism: check script size
                        $scriptLines = @(Get-Content $temp_file).Count
                        if ($scriptLines -gt 1000) {
                            Write-Error "Error: Activation script too large ($scriptLines lines)"
                            return 1
                        }

                        # Source the script
                        . $temp_file
                        
                        # Show any warnings/info
                        if (Test-Path $log_file) {
                            Get-Content $log_file | Write-Warning
                        }
                        
                        return $LASTEXITCODE
                    } else {
                        Get-Content $log_file | Write-Error
                        Get-Content $temp_file | Write-Error
                        return 1
                    }
                } else {
                    Get-Content $log_file | Write-Error
                    return $process.ExitCode
                }
            }
            finally {
                if (Test-Path $temp_file) { Remove-Item -Force $temp_file }
                if (Test-Path $log_file) { Remove-Item -Force $log_file }
                $env:BLAST_RECURSION_CHECK = $null
            }
        } else {
            & $blast_bin.Path $args
            return $LASTEXITCODE
        }
    }
    catch {
        Write-Error $_.Exception.Message
        return 1
    }
    finally {
        $env:BLAST_RECURSION_CHECK = $null
    }
}
"#.trim().to_string(),
            Shell::Unknown => "# Unsupported shell detected".to_string(),
        }
    }

    pub fn install_shell_function(&self) -> BlastResult<()> {
        let function_code = self.generate_shell_function();
        let home_dir = dirs::home_dir().ok_or_else(|| {
            blast_core::error::BlastError::Environment("Home directory not found".to_string())
        })?;

        let rc_file = match self.shell {
            Shell::Bash => home_dir.join(".bashrc"),
            Shell::Zsh => home_dir.join(".zshrc"),
            Shell::Fish => home_dir.join(".config/fish/config.fish"),
            Shell::PowerShell => home_dir.join(if cfg!(windows) {
                "Documents/WindowsPowerShell/Microsoft.PowerShell_profile.ps1"
            } else {
                ".config/powershell/Microsoft.PowerShell_profile.ps1"
            }),
            Shell::Unknown => return Ok(()),
        };

        // Create parent directories if they don't exist
        if let Some(parent) = rc_file.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Read current content
        let mut current_content = std::fs::read_to_string(&rc_file).unwrap_or_default();

        // Remove existing blast function if present
        if let Some(start) = current_content.find("# Blast environment manager") {
            if let Some(end) = current_content[start..].find("\n\n") {
                current_content.replace_range(start..start + end + 2, "");
            }
        }

        // Write the new function
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(&rc_file)?;

        write!(file, "{}\n\n# Blast environment manager integration\n{}\n", current_content.trim(), function_code)?;

        Ok(())
    }
} 