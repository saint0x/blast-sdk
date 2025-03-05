use std::path::PathBuf;
use blast_core::{
    config::BlastConfig,
    error::BlastResult,
    python::{PythonVersion, PythonEnvironment},
    environment::Environment,
};
use blast_daemon::state::StateManagement;
use tracing::{info, debug};

/// Execute the start command
pub async fn execute(
    python: Option<String>,
    name: Option<String>,
    path: Option<PathBuf>,
    _env_vars: Vec<String>,
    config: &BlastConfig,
) -> BlastResult<()> {
    debug!("Initializing blast environment");
    
    // Check if we're already in a blast environment
    if let Ok(current_env) = std::env::var("BLAST_ENV_NAME") {
        return Err(blast_core::error::BlastError::Environment(
            format!("Already in blast environment: {}. Run 'blast kill' first to deactivate", current_env)
        ));
    }
    
    // Resolve environment path
    let env_path = path.unwrap_or_else(|| config.env_path());
    
    // Resolve environment name
    let env_name = name.unwrap_or_else(|| {
        env_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string()
    });

    // Parse Python version
    let python_version = python
        .map(|v| PythonVersion::parse(&v))
        .transpose()?
        .unwrap_or_else(|| config.python_version.clone());

    debug!("Creating environment {} with Python {}", env_name, python_version);

    // Get daemon with proper configuration
    let daemon = crate::commands::get_daemon(config, Some(&env_name)).await?;
    
    // Start daemon in background first to ensure state management is available
    daemon.start_background().await?;
    
    // Brief pause to ensure daemon starts
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    
    // Verify daemon is actually running
    if let Err(e) = daemon.verify_access().await {
        return Err(blast_core::error::BlastError::Environment(
            format!("Daemon failed to start: {}", e)
        ));
    }

    // Create environment if it doesn't exist
    let env = PythonEnvironment::new(
        env_name.clone(),
        env_path.clone(),
        python_version.clone(),
    ).await?;

    debug!("Created Python environment at {}", env.path().display());
    debug!("Setting up state management");

    // First check if there's an active environment
    let state_manager = daemon.state_manager();
    {
        let state_manager = state_manager.read().await;
        let current_state = state_manager.get_current_state().await?;
        
        if current_state.active_env_name.is_some() {
            return Err(blast_core::error::BlastError::Environment(
                format!("Another environment is already active in the daemon. Run 'blast kill' first.")
            ));
        }
    }
    
    // Drop the read lock before acquiring write lock
    let state_manager = daemon.state_manager();
    let mut state_manager = state_manager.write().await;
    state_manager.set_active_environment(env_name.clone(), env_path.clone(), python_version.clone()).await?;
    // Save state to disk
    state_manager.save().await?;
    
    // Drop the write lock before acquiring read lock for verification
    drop(state_manager);
    
    // Verify state was updated
    let state_manager = daemon.state_manager();
    let state_manager = state_manager.read().await;
    let state = state_manager.get_current_state().await?;
    if state.active_env_name.as_deref() != Some(&env_name) {
        return Err(blast_core::error::BlastError::Environment(
            format!("Failed to update environment state for {}", env_name)
        ));
    }

    debug!("Environment state updated successfully");

    // Output shell activation commands directly
    println!("export BLAST_ENV_NAME=\"{}\"", env_name);
    println!("export BLAST_ENV_PATH=\"{}\"", env_path.display());
    println!("export BLAST_PYTHON_VERSION=\"{}\"", python_version);
    println!("export PS1=\"(blast) $PS1\"");
    println!("export PATH=\"{}/bin:$PATH\"", env_path.display());
    println!("hash -r 2>/dev/null || true");

    info!("Environment {} successfully activated", env_name);
    Ok(())
} 