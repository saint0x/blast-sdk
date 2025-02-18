use std::path::PathBuf;
use blast_core::{
    config::BlastConfig,
    error::BlastResult,
    python::PythonVersion,
    security::{SecurityPolicy, IsolationLevel, ResourceLimits},
    Version,
};
use blast_daemon::{Daemon, DaemonConfig};
use tracing::{debug, info, warn};
use uuid::Uuid;
use crate::shell::EnvironmentActivator;
use std::collections::HashMap;
use tokio::process::Command;

/// Execute the start command
pub async fn execute(
    python: Option<String>,
    name: Option<String>,
    path: Option<PathBuf>,
    env_vars: Vec<String>,
    config: &BlastConfig,
) -> BlastResult<()> {
    // If BLAST_EVAL is set, we're in the activation phase
    if std::env::var("BLAST_EVAL").is_ok() {
        return handle_activation_phase(name, path, config).await;
    }

    // Otherwise, we're in the setup phase
    handle_setup_phase(python, name, path, env_vars, config).await
}

/// Handle the activation phase - outputs shell script for environment activation
async fn handle_activation_phase(
    name: Option<String>,
    path: Option<PathBuf>,
    config: &BlastConfig,
) -> BlastResult<()> {
    let env_path = path.unwrap_or_else(|| config.env_path());
    let env_name = name.unwrap_or_else(|| {
        env_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string()
    });

    let activator = EnvironmentActivator::new(env_path, env_name);
    
    // Ensure daemon is running
    if !activator.is_daemon_running() {
        // Initialize and start daemon
        let daemon_config = DaemonConfig {
            max_pending_updates: 100,
            max_snapshot_age_days: 7,
            env_path: activator.env_path().to_path_buf(),
            cache_path: config.project_root.join("cache"),
        };

        let daemon = Daemon::new(daemon_config).await?;
        start_background(&daemon).await?;
    }

    // Save shell state
    activator.save_state()?;

    // Output activation script
    println!("{}", activator.generate_activation_script());
    Ok(())
}

/// Handle the setup phase - creates the environment and starts the daemon
async fn handle_setup_phase(
    python: Option<String>,
    name: Option<String>,
    path: Option<PathBuf>,
    env_vars: Vec<String>,
    config: &BlastConfig,
) -> BlastResult<()> {
    debug!("Starting environment creation");
    
    // Check if we're already in a blast environment
    if let Ok(current_env) = std::env::var("BLAST_ENV_NAME") {
        warn!("Already in blast environment: {}", current_env);
        return Ok(());
    }
    
    // Resolve environment path
    let env_path = path.unwrap_or_else(|| config.env_path());
    if !env_path.exists() {
        debug!("Creating environment directory");
        std::fs::create_dir_all(&env_path)?;
    }

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

    debug!("Creating environment with Python {}", python_version);

    // Create daemon configuration
    let daemon_config = DaemonConfig {
        max_pending_updates: 100,
        max_snapshot_age_days: 7,
        env_path: env_path.clone(),
        cache_path: config.project_root.join("cache"),
    };

    // Initialize daemon
    let daemon = Daemon::new(daemon_config).await?;
    
    // Create security policy
    let security_policy = SecurityPolicy {
        isolation_level: IsolationLevel::Process,
        python_version: python_version.clone(),
        resource_limits: ResourceLimits::default(),
    };

    // Create environment
    let env = daemon.create_environment(&security_policy).await?;
    debug!("Created Python environment at {}", env.path().display());

    // Create environment state
    let env_vars: HashMap<String, Version> = env_vars
        .into_iter()
        .filter_map(|var| {
            let parts: Vec<&str> = var.split('=').collect();
            if parts.len() == 2 {
                Version::parse(parts[1]).ok().map(|version| (parts[0].to_string(), version))
            } else {
                None
            }
        })
        .collect();

    let env_state = blast_core::state::EnvironmentState::new(
        env_name.clone(),
        python_version.clone(),
        env_vars,
        Default::default(),
    );

    // Get state manager and ensure it's loaded
    let state_manager = daemon.state_manager();
    state_manager.write().await.load().await?;

    // Add environment to state manager
    state_manager.write().await.add_environment(env_name.clone(), env_state.clone()).await?;
    debug!("Added environment to state manager");

    // Set as active environment
    state_manager.write().await.set_active_environment(
        env_name.clone(),
        env.path().to_path_buf(),
        python_version.clone(),
    ).await?;
    debug!("Set as active environment");

    // Create initial checkpoint
    state_manager.write().await.create_checkpoint(
        Uuid::new_v4(),
        "Initial environment creation".to_string(),
        None,
    ).await?;
    debug!("Created initial checkpoint");

    // Create and save shell state
    let activator = EnvironmentActivator::new(
        env.path().to_path_buf(),
        env_name.clone(),
    );
    activator.save_state()?;

    info!("Environment ready: {} (Python {})", env_name, python_version);

    // Start daemon in background
    start_background(&daemon).await?;

    // Re-execute with BLAST_EVAL set to get shell activation
    let current_exe = std::env::current_exe()?;
    let mut cmd = Command::new(current_exe);
    cmd.arg("start")
        .env("BLAST_EVAL", "1")
        .env("BLAST_ENV_NAME", env_name)
        .env("BLAST_ENV_PATH", env.path());

    let output = cmd.output().await?;
    println!("{}", String::from_utf8_lossy(&output.stdout));

    Ok(())
}

async fn start_background(daemon: &Daemon) -> BlastResult<()> {
    // Start the daemon's background service
    daemon.start_background().await?;

    // If we're not already running as a daemon process
    if std::env::var("BLAST_DAEMON").is_err() {
        // Get current executable path
        let exe_path = std::env::current_exe()?;
        
        // Create command to run daemon in background
        let mut cmd = Command::new(exe_path);
        cmd.arg("daemon")
           .env("BLAST_DAEMON", "1")
           .stdin(std::process::Stdio::null())
           .stdout(std::process::Stdio::null())
           .stderr(std::process::Stdio::null());

        // Create a pid file to track the daemon process
        let pid_file = std::path::Path::new("/tmp/blast").join("daemon.pid");
        if !pid_file.parent().unwrap().exists() {
            std::fs::create_dir_all(pid_file.parent().unwrap())?;
        }

        // Spawn the process
        let child = cmd.spawn()?;
        
        // Write PID to file if available
        if let Some(pid) = child.id() {
            std::fs::write(&pid_file, pid.to_string())?;
        }

        // Don't wait for child, let it run in background
        tokio::spawn(async move {
            let _ = child.wait_with_output().await;
            // Clean up PID file when process exits
            let _ = std::fs::remove_file(&pid_file);
        });

        // Brief pause to ensure daemon is started
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    
    Ok(())
} 