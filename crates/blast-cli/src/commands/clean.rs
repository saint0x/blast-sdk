use blast_core::{
    config::BlastConfig,
    error::BlastResult,
    python::{PythonEnvironment, PythonVersion},
    environment::Environment,
};
use blast_daemon::{Daemon, DaemonConfig, state::StateManagement};
use tracing::{debug, info};

/// Execute the clean command
pub async fn execute(config: &BlastConfig) -> BlastResult<()> {
    debug!("Cleaning environment");

    // Check if we're in a blast environment
    let env_name = match std::env::var("BLAST_ENV_NAME") {
        Ok(name) => name,
        Err(_) => {
            info!("Not in a blast environment");
            return Ok(());
        }
    };

    // Create daemon configuration
    let daemon_config = DaemonConfig {
        max_pending_updates: 100,
        max_snapshot_age_days: 7,
        env_path: config.project_root.join("environments/default"),
        cache_path: config.project_root.join("cache"),
    };

    // Connect to existing daemon
    let daemon = Daemon::new(daemon_config).await?;

    // Get current environment state
    let state_manager = daemon.state_manager();
    let state_guard = state_manager.read().await;
    let state_manager: &dyn StateManagement = &*state_guard;
    let current_state = state_manager.get_current_state().await?;

    if current_state.active_env_name.is_some() {
        info!("Cleaning environment: {}", env_name);
        
        // Create a Python environment instance
        let environment = PythonEnvironment::new(
            env_name.clone(),
            config.project_root.join("environments").join(&env_name),
            current_state.active_python_version.unwrap_or_else(|| PythonVersion::parse("3.8.0").unwrap()),
        ).await?;

        // Initialize the environment
        Environment::init(&environment).await?;
        
        info!("Environment cleaned and reinitialized");
    } else {
        info!("No active environment found");
    }

    Ok(())
} 