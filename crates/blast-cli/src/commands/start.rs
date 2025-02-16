use std::path::PathBuf;
use blast_core::{
    config::BlastConfig,
    error::BlastResult,
    python::PythonVersion,
    security::{SecurityPolicy, IsolationLevel, ResourceLimits},
};
use blast_daemon::{
    DaemonConfig,
    Daemon,
};
use tracing::{debug, info};

/// Execute the start command
pub async fn execute(
    python: Option<String>,
    name: Option<String>,
    path: Option<PathBuf>,
    env_vars: Vec<String>,
    config: &BlastConfig,
) -> BlastResult<()> {
    debug!("Starting blast environment");
    debug!("Python version: {:?}", python);
    debug!("Name: {:?}", name);
    debug!("Path: {:?}", path);
    debug!("Environment variables: {:?}", env_vars);

    // Parse Python version
    let python_version = match python {
        Some(ver) => PythonVersion::parse(&ver)?,
        None => config.python_version.clone(),
    };

    // Use provided path or current directory
    let env_path = path.unwrap_or_else(|| config.env_path());

    // Use provided name or directory name
    let env_name = name.unwrap_or_else(|| {
        env_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string()
    });

    // Create daemon configuration
    let daemon_config = DaemonConfig {
        max_pending_updates: 100,
    };

    // Create security policy
    let security_policy = SecurityPolicy {
        isolation_level: IsolationLevel::Process,
        python_version: python_version.clone(),
        resource_limits: ResourceLimits::default(),
    };

    // Create and start daemon
    let daemon = Daemon::new(daemon_config).await?;
    
    // Create environment with isolation
    let _env = daemon.create_environment(&security_policy).await?;

    // Set up shell prompt
    std::env::set_var("BLAST_ENV_NAME", &env_name);
    std::env::set_var("BLAST_ENV_PATH", env_path.display().to_string());
    std::env::set_var("BLAST_SOCKET_PATH", format!("/tmp/blast_{}.sock", env_name));
    
    // Set custom environment variables
    for var in env_vars {
        if let Some((key, value)) = var.split_once('=') {
            std::env::set_var(key, value);
        }
    }

    // Modify shell prompt
    if let Ok(shell) = std::env::var("SHELL") {
        if shell.contains("bash") {
            std::env::set_var("PS1", "(blast) $PS1");
        } else if shell.contains("zsh") {
            std::env::set_var("PROMPT", "(blast) $PROMPT");
        }
    }

    info!("Started blast environment:");
    info!("  Name: {}", env_name);
    info!("  Python: {}", python_version);
    info!("  Path: {}", env_path.display());
    info!("  Isolation: Process-level");
    info!("  Monitoring: Active");

    // Keep daemon running
    tokio::signal::ctrl_c().await?;
    daemon.shutdown().await?;

    Ok(())
} 