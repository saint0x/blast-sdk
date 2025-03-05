use blast_core::{
    config::BlastConfig,
    error::BlastResult,
};
use blast_daemon::{Daemon, DaemonConfig, state::StateManagement};
use tracing::{info, debug};
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
    force: bool,
    config: &BlastConfig,
) -> BlastResult<()> {
    debug!("Killing environment");

    // Create daemon configuration
    let daemon_config = DaemonConfig {
        max_pending_updates: 100,
        max_snapshot_age_days: 7,
        env_path: config.project_root.join("environments").join(&env_name),
        cache_path: config.project_root.join("cache"),
    };

    // Connect to daemon
    let daemon = Daemon::new(daemon_config).await?;

    // Get current state before we kill everything
    let state_manager = daemon.state_manager();
    let state_manager = state_manager.read().await;
    let state = state_manager.get_current_state().await?;

    // Store environment info for logging
    let env_path = state.active_env_path.clone();

    // Clear active environment state
    let state_manager = daemon.state_manager();
    let state_manager = state_manager.write().await;
    state_manager.clear_active_environment().await?;

    // Read PID file
    let pid_file = std::path::Path::new("/tmp/blast").join("daemon.pid");
    if pid_file.exists() {
        if let Ok(pid_str) = std::fs::read_to_string(&pid_file) {
            if let Ok(pid) = pid_str.trim().parse::<u32>() {
                // Kill the daemon process
                #[cfg(unix)]
                {
                    if force {
                        // Force kill immediately
                        let _ = kill(Pid::from_raw(pid as i32), Signal::SIGKILL);
                    } else {
                        // Try graceful shutdown first
                        let _ = kill(Pid::from_raw(pid as i32), Signal::SIGTERM);
                        // Give it a moment to clean up
                        std::thread::sleep(std::time::Duration::from_millis(100));
                        // Force kill if still running
                        let _ = kill(Pid::from_raw(pid as i32), Signal::SIGKILL);
                    }
                }
                
                #[cfg(windows)]
                {
                    let _ = Command::new("taskkill")
                        .args(&["/PID", &pid.to_string(), if force { "/F" } else { "" }])
                        .output();
                }
            }
        }
        // Clean up PID file
        let _ = std::fs::remove_file(&pid_file);
    }

    // Output deactivation script for shell to evaluate
    println!(r#"
# Reset old environment variables
if [ -n "$_OLD_BLAST_PATH" ] ; then
    PATH="$_OLD_BLAST_PATH"
    export PATH
    unset _OLD_BLAST_PATH
fi

if [ -n "$_OLD_BLAST_PYTHONPATH" ] ; then
    PYTHONPATH="$_OLD_BLAST_PYTHONPATH"
    export PYTHONPATH
    unset _OLD_BLAST_PYTHONPATH
fi

if [ -n "$_OLD_BLAST_PS1" ] ; then
    PS1="$_OLD_BLAST_PS1"
    export PS1
    unset _OLD_BLAST_PS1
fi

# Unset blast environment variables
unset BLAST_ENV_NAME
unset BLAST_ENV_PATH
unset BLAST_SOCKET_PATH

# Clean up functions
unset -f deactivate
unset -f pip

# Reset hash
hash -r 2>/dev/null"#);

    if let Some(path) = env_path {
        info!("Killed blast environment:");
        info!("  Name: {}", env_name);
        info!("  Path: {}", path.display());
    }

    Ok(())
} 