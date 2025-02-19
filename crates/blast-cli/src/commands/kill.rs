use blast_core::{
    config::BlastConfig,
    error::BlastResult,
};
use tracing::info;
use crate::shell::EnvironmentActivator;

#[cfg(unix)]
use nix::sys::signal::{kill, Signal};
#[cfg(unix)]
use nix::unistd::Pid;

#[cfg(windows)]
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

    // Create activator to get deactivation script
    let activator = EnvironmentActivator::new(env_path.clone(), env_name.clone());
    
    // Output deactivation script
    print!("{}", activator.generate_deactivation_script());

    info!("Killed blast environment:");
    info!("  Name: {}", env_name);
    info!("  Path: {}", env_path.display());

    Ok(())
} 