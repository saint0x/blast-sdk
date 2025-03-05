use blast_core::{error::BlastResult, config::BlastConfig};
use blast_daemon::state::StateManagement;
use tracing::{info, debug};
use std::path::Path;
use crate::commands;

/// Execute the check command
pub async fn execute(config: &BlastConfig) -> BlastResult<()> {
    debug!("Checking blast environment state");

    // Get current environment name if any
    let env_name = std::env::var("BLAST_ENV_NAME").ok();
    let env_path = std::env::var("BLAST_ENV_PATH").ok();
    let python_version = std::env::var("BLAST_PYTHON_VERSION").ok();
    
    debug!("Shell environment variables: name={:?}, path={:?}, python={:?}", 
        env_name, env_path, python_version);
    
    // Get daemon with proper configuration
    let daemon = commands::get_daemon(config, env_name.as_deref()).await?;

    // Get state manager and load current state
    let state_manager = daemon.state_manager();
    let state_manager = state_manager.read().await;
    debug!("Loading state from disk");
    state_manager.load().await?;
    let state = state_manager.get_current_state().await?;
    debug!("Daemon state: name={:?}, path={:?}", 
        state.active_env_name, state.active_env_path);

    // Verify shell environment matches daemon state
    let shell_active = env_name.is_some() && env_path.is_some() && python_version.is_some();
    let daemon_active = state.active_env_name.is_some() && state.active_env_path.is_some();

    // Check if we're in a blast environment
    match (shell_active, daemon_active) {
        (true, true) => {
            // Verify shell environment matches daemon state
            if env_name.as_deref() == state.active_env_name.as_deref() 
                && env_path.as_deref().map(Path::new) == state.active_env_path.as_deref() {
                debug!("Shell and daemon states match");
                let env_name = env_name.as_ref().unwrap();
                let env_path = env_path.as_ref().unwrap();
                let python_version = python_version.as_ref().unwrap();
                info!("Environment '{}' is active and synchronized", env_name);
                println!("Active blast environment:");
                println!("  Name: {}", env_name);
                println!("  Path: {}", env_path);
                println!("  Python version: {}", python_version);
            } else {
                debug!("State mismatch detected");
                let env_name = env_name.as_ref().unwrap();
                let env_path = env_path.as_ref().unwrap();
                info!("Environment state mismatch between shell and daemon");
                println!("Warning: Shell environment does not match daemon state");
                println!("Shell environment:");
                println!("  Name: {}", env_name);
                println!("  Path: {}", env_path);
                println!("Daemon state:");
                if let Some(name) = &state.active_env_name {
                    println!("  Name: {}", name);
                }
                if let Some(path) = &state.active_env_path {
                    println!("  Path: {}", path.display());
                }
            }
        },
        (true, false) => {
            debug!("Shell active but daemon inactive");
            let env_name = env_name.as_ref().unwrap();
            let env_path = env_path.as_ref().unwrap();
            info!("Environment '{}' is active in shell but not in daemon", env_name);
            println!("Warning: Shell environment is active but daemon shows no active environment");
            println!("Shell environment:");
            println!("  Name: {}", env_name);
            println!("  Path: {}", env_path);
        },
        (false, true) => {
            debug!("Daemon active but shell inactive");
            info!("Environment '{}' is active in daemon but not in shell", 
                state.active_env_name.as_deref().unwrap_or("unknown"));
            println!("Warning: Daemon shows active environment but shell environment is not set");
            println!("Daemon state:");
            if let Some(name) = &state.active_env_name {
                println!("  Name: {}", name);
            }
            if let Some(path) = &state.active_env_path {
                println!("  Path: {}", path.display());
            }
        },
        (false, false) => {
            debug!("No active environment found");
            info!("No blast environment is currently active");
            println!("No active blast environment");
        }
    }

    // Check if daemon is running
    match daemon.verify_access().await {
        Ok(_) => debug!("Daemon is running and responsive"),
        Err(e) => info!("Warning: Daemon is not running: {}", e),
    }

    Ok(())
} 