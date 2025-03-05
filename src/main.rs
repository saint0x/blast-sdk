use anyhow::Result;
use blast_cli;
use tracing::{info, error, debug, Level, warn};
use tracing_subscriber::{FmtSubscriber, EnvFilter};
use tracing_subscriber::fmt::format::FmtSpan;
use tokio::sync::{watch, mpsc};
use std::sync::Once;
use blast_daemon::{
    Daemon, DaemonConfig, state::StateManagement,
    monitor::{PythonResourceMonitor, PythonResourceLimits},
    metrics::MetricsManager,
    error::DaemonResult,
};
use blast_core::python::PythonVersion;
use chrono;
use std::time::Duration;
use tokio::time::sleep;
use std::sync::Arc;

static LOGGING_INIT: Once = Once::new();

const MAX_STARTUP_ATTEMPTS: u32 = 3;
const STARTUP_RETRY_DELAY: Duration = Duration::from_secs(2);
const HEALTH_CHECK_INTERVAL: Duration = Duration::from_secs(30);

/// Health check status
#[derive(Debug)]
struct HealthStatus {
    daemon_healthy: bool,
    state_healthy: bool,
    monitor_healthy: bool,
}

/// Health check manager
struct HealthManager {
    daemon: Arc<Daemon>,
    monitor: PythonResourceMonitor,
    metrics: MetricsManager,
}

impl HealthManager {
    fn new(daemon: Daemon, monitor: PythonResourceMonitor, metrics: MetricsManager) -> Self {
        Self {
            daemon: Arc::new(daemon),
            monitor,
            metrics,
        }
    }

    async fn check_health(&mut self) -> DaemonResult<HealthStatus> {
        let mut status = HealthStatus {
            daemon_healthy: false,
            state_healthy: false,
            monitor_healthy: false,
        };

        // Check daemon access
        if self.daemon.verify_access().await.is_ok() {
            status.daemon_healthy = true;
        }

        // Check state manager
        let state_manager = self.daemon.state_manager();
        if state_manager.read().await.verify().await.is_ok() {
            status.state_healthy = true;
        }

        // Check monitor - this is not async
        if self.monitor.check_limits() {
            status.monitor_healthy = true;
        }

        Ok(status)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let eval_mode = std::env::var("BLAST_EVAL").is_ok();
    init_logging(eval_mode);

    // Always try to ensure shell integration is set up
    if let Err(e) = blast_cli::initialize() {
        warn!("Failed to set up shell integration: {}", e);
        warn!("You may need to manually add shell integration. See documentation for details.");
    }

    if eval_mode {
        info!("Initializing blast environment...");
        // We're in eval mode, just run the command and output the results
        blast_cli::run().await
    } else if let Some("daemon") = std::env::args().nth(1).as_deref() {
        run_daemon().await
    } else if let Some("start") = std::env::args().nth(1).as_deref() {
        start_environment().await
    } else {
        debug!("Running regular command");
        blast_cli::run().await
    }
}

async fn run_daemon() -> Result<()> {
    info!("Starting blast daemon...");
    
    // Get project root
    let project_root = std::env::current_dir()?;
    
    // Initialize daemon with configuration
    let daemon = Daemon::new(DaemonConfig {
        max_pending_updates: 100,
        max_snapshot_age_days: 7,
        env_path: project_root.join("environments/default"),
        cache_path: project_root.join("cache"),
    }).await?;

    // Create channels for component communication
    let (monitor_tx, mut monitor_rx) = mpsc::channel(100);
    let (_shutdown_tx, mut shutdown_rx) = watch::channel(false);

    // Initialize components
    let metrics_manager = MetricsManager::new();
    let resource_monitor = PythonResourceMonitor::new(
        project_root.join("environments/default"),
        project_root.join("cache"),
        PythonResourceLimits::default(),
    );

    // Create health manager
    let mut health_manager = HealthManager::new(
        daemon,
        resource_monitor,
        metrics_manager,
    );

    // Start background services
    health_manager.daemon.start_background().await?;

    // Main daemon loop with integrated health checks
    let mut health_check_interval = tokio::time::interval(HEALTH_CHECK_INTERVAL);
    loop {
        tokio::select! {
            _ = shutdown_rx.changed() => {
                info!("Received shutdown signal");
                break;
            }
            Some(event) = monitor_rx.recv() => {
                if let Err(e) = handle_monitor_event(event).await {
                    error!("Failed to handle monitor event: {}", e);
                }
            }
            _ = health_check_interval.tick() => {
                match health_manager.check_health().await {
                    Ok(status) => {
                        if !status.daemon_healthy {
                            error!("Daemon health check failed");
                        }
                        if !status.state_healthy {
                            error!("State manager health check failed");
                        }
                        if !status.monitor_healthy {
                            error!("Resource monitor health check failed");
                        }
                    }
                    Err(e) => {
                        error!("Health check failed: {}", e);
                    }
                }
            }
        }
    }

    info!("Daemon shutting down");
    Ok(())
}

async fn start_environment() -> Result<()> {
    info!("Starting blast environment setup...");
    
    // Get project root
    let project_root = std::env::current_dir()?;
    
    // Create directory structure
    for dir in &[
        ".blast",
        "environments",
        "environments/default",
        "environments/default/bin",
        "environments/default/lib",
        "environments/default/lib/python3",
        "environments/default/lib/python3/site-packages",
        "cache",
        ".blast/state",
        ".blast/logs",
        ".blast/metrics",
    ] {
        let path = project_root.join(dir);
        if !path.exists() {
            std::fs::create_dir_all(&path)?;
        }
    }

    // Initialize daemon with retries
    let mut daemon = None;
    for attempt in 1..=MAX_STARTUP_ATTEMPTS {
        match Daemon::new(DaemonConfig {
            max_pending_updates: 100,
            max_snapshot_age_days: 7,
            env_path: project_root.join("environments/default"),
            cache_path: project_root.join("cache"),
        }).await {
            Ok(d) => {
                daemon = Some(d);
                break;
            }
            Err(e) => {
                error!("Attempt {} to initialize daemon failed: {}", attempt, e);
                if attempt < MAX_STARTUP_ATTEMPTS {
                    sleep(STARTUP_RETRY_DELAY).await;
                } else {
                    return Err(anyhow::anyhow!("Failed to initialize daemon after {} attempts", MAX_STARTUP_ATTEMPTS));
                }
            }
        }
    }

    let daemon = daemon.unwrap();

    // Initialize state manager
    let state_manager = daemon.state_manager();
    let state_manager = state_manager.read().await;

    // Initialize and verify state
    let mut state = state_manager.get_current_state().await?;
    state.active_env_name = Some("default".to_string());
    state.active_env_path = Some(project_root.join("environments/default"));
    state.active_python_version = Some(PythonVersion::parse("3.8")?);
    state.last_update = Some(chrono::Utc::now());

    // Update and save state
    state_manager.update_current_state(state).await?;
    state_manager.save().await?;

    // Start daemon and verify it's running
    daemon.start_background().await?;
    
    // Verify daemon access with retries
    for attempt in 1..=MAX_STARTUP_ATTEMPTS {
        match daemon.verify_access().await {
            Ok(_) => break,
            Err(e) => {
                error!("Attempt {} to verify daemon access failed: {}", attempt, e);
                if attempt < MAX_STARTUP_ATTEMPTS {
                    sleep(STARTUP_RETRY_DELAY).await;
                } else {
                    return Err(anyhow::anyhow!("Failed to verify daemon access after {} attempts", MAX_STARTUP_ATTEMPTS));
                }
            }
        }
    }

    // Get environment path
    let env_path = project_root.join("environments/default");
    
    // Create an activator for shell integration
    let activator = blast_cli::shell::EnvironmentActivator::new(
        env_path.clone(),
        "default".to_string(),
    );

    // Save the shell state
    activator.save_state()?;

    // Generate the activation script
    let mut activation_script = activator.generate_activation_script();
    
    // Only modify the PS1 prompt to show (blast) instead of (blast:default)
    activation_script = activation_script.replace(
        r#"export PS1="(blast:default) $PS1""#,
        r#"export PS1="(blast) $PS1""#
    );

    // Output the activation script for shell sourcing
    print!("{}", activation_script);

    Ok(())
}

fn init_logging(eval_mode: bool) {
    LOGGING_INIT.call_once(|| {
        let builder = FmtSubscriber::builder()
            .with_env_filter(EnvFilter::from_default_env()
                .add_directive(if eval_mode {
                    Level::INFO.into()
                } else {
                    Level::DEBUG.into()
                }));

        // Configure based on mode
        let builder = if eval_mode {
            builder
                .with_target(false)
                .with_ansi(false)
                .with_thread_ids(false)
                .with_thread_names(false)
                .with_file(false)
                .with_line_number(false)
                .with_span_events(FmtSpan::NONE)
        } else {
            builder
                .with_target(false)
                .with_ansi(true)
                .with_thread_ids(true)
                .with_thread_names(true)
                .with_file(true)
                .with_line_number(true)
                .with_span_events(FmtSpan::ACTIVE)
        };

        let _ = builder.try_init();
    });
}

async fn handle_monitor_event(event: blast_daemon::monitor::MonitorEvent) -> Result<()> {
    match event {
        blast_daemon::monitor::MonitorEvent::ResourceCheck => {
            debug!("Processing resource check event");
        }
        blast_daemon::monitor::MonitorEvent::ResourceUpdate(usage) => {
            debug!("Resource usage update: {:?}", usage);
        }
        blast_daemon::monitor::MonitorEvent::PackageChanged => {
            info!("Package changes detected, syncing state");
        }
        blast_daemon::monitor::MonitorEvent::FileChanged(path) => {
            debug!("File change detected: {:?}", path);
        }
        blast_daemon::monitor::MonitorEvent::StopMonitoring { env_path } => {
            info!("Stopping monitoring for environment: {:?}", env_path);
        }
    }
    Ok(())
} 