use blast_core::{
    config::BlastConfig,
    error::BlastResult,
};
use tracing::info;
use std::process::Command;

/// Execute the kill command
pub async fn execute(
    env_name: String,
    _force: bool,
    config: &BlastConfig,
) -> BlastResult<()> {
    let env_path = config.project_root.join("environments").join(&env_name);
    
    // Read PID file
    let pid_file = std::path::Path::new("/tmp/blast").join("daemon.pid");
    if pid_file.exists() {
        if let Ok(pid_str) = std::fs::read_to_string(&pid_file) {
            if let Ok(pid) = pid_str.trim().parse::<u32>() {
                // Kill the daemon process
                #[cfg(unix)]
                {
                    use nix::sys::signal::{kill, Signal};
                    use nix::unistd::Pid;
                    // First try SIGTERM
                    let _ = kill(Pid::from_raw(pid as i32), Signal::SIGTERM);
                    // Give it a moment to clean up
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    // Force kill if still running
                    let _ = kill(Pid::from_raw(pid as i32), Signal::SIGKILL);
                }
                
                #[cfg(windows)]
                {
                    let _ = Command::new("taskkill")
                        .args(&["/PID", &pid.to_string(), "/F"])
                        .output();
                }
            }
        }
        // Clean up PID file
        let _ = std::fs::remove_file(&pid_file);
    }

    // Clean up environment variables
    std::env::remove_var("BLAST_ENV_NAME");
    std::env::remove_var("BLAST_ENV_PATH");
    std::env::remove_var("BLAST_SOCKET_PATH");
    std::env::remove_var("BLAST_DAEMON");
    std::env::remove_var("BLAST_EVAL");

    // Kill any running terminal processes
    if let Ok(ppid) = std::env::var("PPID") {
        if let Ok(ppid) = ppid.parse::<i32>() {
            #[cfg(unix)]
            {
                use nix::sys::signal::{kill, Signal};
                use nix::unistd::Pid;
                let _ = kill(Pid::from_raw(ppid), Signal::SIGTERM);
            }
        }
    }

    // Restore shell prompt
    if let Ok(shell) = std::env::var("SHELL") {
        if shell.contains("bash") || shell.contains("zsh") {
            println!("PS1=\"${{PS1#\\(blast\\) }}\"");
        } else if shell.contains("fish") {
            println!("functions -e fish_prompt; functions -c _old_fish_prompt fish_prompt");
        }
    }

    info!("Killed blast environment:");
    info!("  Name: {}", env_name);
    info!("  Path: {}", env_path.display());

    Ok(())
} 