use blast_core::{error::BlastResult, config::BlastConfig};
use blast_daemon::state::StateManagement;
use tracing::{info, debug};
use std::path::Path;
use tokio::fs;
use crate::commands;

/// Execute the list command
pub async fn execute(config: &BlastConfig) -> BlastResult<()> {
    debug!("Scanning for blast environments");

    // Get current environment name if any
    let env_name = std::env::var("BLAST_ENV_NAME").ok();
    let env_path = std::env::var("BLAST_ENV_PATH").ok();
    
    debug!("Current shell environment: name={:?}, path={:?}", env_name, env_path);
    
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

    // List all available environments
    let environments_dir = config.project_root.join("environments");
    debug!("Scanning environments directory: {}", environments_dir.display());
    
    println!("Available environments:");
    if environments_dir.exists() {
        let mut entries = fs::read_dir(&environments_dir).await?;
        let mut env_count = 0;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_dir() {
                debug!("Checking directory: {}", path.display());
                if is_blast_environment(&path).await? {
                    env_count += 1;
                    let name = path.file_name().unwrap().to_string_lossy();
                    let is_active = env_name.as_ref().map_or(false, |active| active == &*name);
                    
                    let status = if is_active { "*" } else { " " };
                    println!("{} {}", status, name);
                } else {
                    debug!("Directory is not a valid blast environment: {}", path.display());
                }
            }
        }
        debug!("Found {} valid environment(s)", env_count);
    }

    Ok(())
}

/// Check if a directory is a valid blast environment
async fn is_blast_environment(path: &Path) -> BlastResult<bool> {
    let bin_dir = path.join("bin");
    let lib_dir = path.join("lib");
    let lib_python_dir = lib_dir.join("python3").join("site-packages");
    
    let is_valid = bin_dir.exists() && lib_dir.exists() && lib_python_dir.exists();
    debug!("Environment validation for {}: bin={}, lib={}, site-packages={}", 
        path.display(), bin_dir.exists(), lib_dir.exists(), lib_python_dir.exists());
    
    Ok(is_valid)
} 