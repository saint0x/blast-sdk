use std::path::PathBuf;
use std::io::Write;
use blast_core::{
    config::BlastConfig,
    error::BlastResult,
    python::PythonVersion,
    security::{SecurityPolicy, IsolationLevel, ResourceLimits},
};
use blast_daemon::{Daemon, DaemonConfig};
use uuid::Uuid;
use crate::shell::EnvironmentActivator;
use crate::environment::Environment;
use tokio::process::Command;

/// Execute the start command
pub async fn execute(
    python: Option<String>,
    name: Option<String>,
    path: Option<PathBuf>,
    env_vars: Vec<String>,
    config: &BlastConfig,
) -> BlastResult<()> {
    // Check if we're in the final script output phase
    if std::env::var("BLAST_SCRIPT_OUTPUT").is_ok() {
        // Disable all output except the script
        std::env::set_var("NO_COLOR", "1");
        std::env::set_var("CLICOLOR", "0");
        std::env::set_var("CLICOLOR_FORCE", "0");
        std::env::set_var("RUST_LOG", "off");

        let env_path = path.unwrap_or_else(|| config.env_path());
        let activate_script = env_path.join("bin").join("activate");
        
        if !activate_script.exists() {
            return Err(blast_core::error::BlastError::Environment(
                "Activation script not found".to_string()
            ));
        }

        // Read and output the script directly
        let script_content = std::fs::read_to_string(&activate_script)?;
        print!("{}", script_content); // Use print! instead of write_all for cleaner output
        std::io::stdout().flush()?;
        
        return Ok(());
    }

    // If we're not in script output mode, we're in the initial setup phase
    handle_setup_phase(python, name, path, env_vars, config).await
}

/// Handle the setup phase - creates the environment and starts the daemon
async fn handle_setup_phase(
    python: Option<String>,
    name: Option<String>,
    path: Option<PathBuf>,
    env_vars: Vec<String>,
    config: &BlastConfig,
) -> BlastResult<()> {
    eprintln!("Creating new Blast environment");
    
    // Check if we're already in a blast environment
    if let Ok(current_env) = std::env::var("BLAST_ENV_NAME") {
        eprintln!("Warning: Already in blast environment: {}", current_env);
        eprintln!("Action Required: Run 'blast kill' first to deactivate");
        return Ok(());
    }

    // Clone values early to avoid ownership issues
    let name_for_cmd = name.clone();
    let path_for_cmd = path.clone();
    
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

    eprintln!("Environment Configuration");
    eprintln!("Name: {}", env_name);
    eprintln!("Python Version: {}", python_version);
    eprintln!("Location: {}", env_path.display());

    // Create the environment structure
    eprintln!("Creating Environment");
    eprintln!("Setting up directory structure");
    let environment = Environment::new(
        env_path.clone(),
        env_name.clone(),
        python_version.clone(),
    );
    environment.create()?;
    eprintln!("Done");

    // Create daemon configuration
    let daemon_config = DaemonConfig {
        max_pending_updates: 100,
        max_snapshot_age_days: 7,
        env_path: env_path.clone(),
        cache_path: config.project_root.join("cache"),
    };

    eprintln!("Initializing Services");
    eprintln!("Starting daemon");
    let daemon = Daemon::new(daemon_config).await?;
    eprintln!("Done");
    
    // Create security policy
    let security_policy = SecurityPolicy {
        isolation_level: IsolationLevel::Process,
        python_version: python_version.clone(),
        resource_limits: ResourceLimits::default(),
    };
    
    eprintln!("Creating Python environment");
    let env = daemon.create_environment(&security_policy).await?;
    eprintln!("Done");

    eprintln!("Setting Up State");
    eprintln!("Initializing state manager");
    let state_manager = daemon.state_manager();
    state_manager.write().await.load().await?;
    eprintln!("Done");

    // Create environment state
    let env_state = blast_core::state::EnvironmentState::new(
        env_name.clone(),
        python_version.clone(),
        env_vars.into_iter()
            .filter_map(|var| {
                let parts: Vec<&str> = var.split('=').collect();
                if parts.len() == 2 {
                    blast_core::Version::parse("0.1.0")
                        .ok()
                        .map(|version| (parts[0].to_string(), version))
                } else {
                    None
                }
            })
            .collect(),
        Default::default(),
    );

    eprintln!("Saving environment state");
    state_manager.write().await.add_environment(env_name.clone(), env_state.clone()).await?;
    eprintln!("Done");

    eprintln!("Creating initial checkpoint");
    state_manager.write().await.create_checkpoint(
        Uuid::new_v4(),
        "Initial environment creation".to_string(),
        None,
    ).await?;
    eprintln!("Done");

    // Create and save shell state
    let activator = EnvironmentActivator::new(
        env.path().to_path_buf(),
        env_name.clone(),
    );

    eprintln!("Saving shell state");
    activator.save_state()?;
    eprintln!("Done");

    eprintln!("Starting Background Services");
    eprintln!("Starting daemon process");
    start_background(&daemon).await?;
    eprintln!("Done");

    eprintln!("Activating Environment");

    // Get the activation script
    let current_exe = std::env::current_exe()?;
    let mut script_cmd = Command::new(current_exe);
    script_cmd.arg("start")
        .env("BLAST_SCRIPT_OUTPUT", "1")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null());

    if let Some(n) = name_for_cmd {
        script_cmd.arg("--name").arg(n);
    }
    if let Some(p) = path_for_cmd {
        script_cmd.arg("--path").arg(p);
    }

    let script_output = script_cmd.output().await?;
    if !script_output.status.success() {
        eprintln!("Failed to generate activation script");
        return Err(blast_core::error::BlastError::Environment(
            "Failed to generate activation script".to_string()
        ));
    }

    // Write script to stdout for shell function to source
    std::io::stdout().write_all(&script_output.stdout)?;
    std::io::stdout().flush()?;

    eprintln!("Environment ready!");
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
        let mut cmd = Command::new(&exe_path);
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
            
            // Verify the daemon started
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            if !pid_file.exists() {
                return Err(blast_core::error::BlastError::Environment(
                    "Failed to start daemon process".to_string()
                ));
            }
            
            // Log daemon start if not in script output mode
            if std::env::var("BLAST_SCRIPT_OUTPUT").is_err() {
                eprintln!("Started daemon process with PID {}", pid);
            }
        }

        // Don't wait for child, let it run in background
        tokio::spawn(async move {
            let _ = child.wait_with_output().await;
            // Clean up PID file when process exits
            let _ = std::fs::remove_file(&pid_file);
        });
    }
    
    Ok(())
} 