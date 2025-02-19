use anyhow::Result;
use blast_cli;
use tracing::{info, error, debug, Level, warn};
use tracing_subscriber::{FmtSubscriber, EnvFilter};
use tracing_subscriber::fmt::format::FmtSpan;
use tokio::sync::watch;
use std::sync::Arc;
use std::sync::RwLock;
use std::sync::Once;

static LOGGING_INIT: Once = Once::new();

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
    } else {
        info!("Starting blast environment setup...");

        if let Some("start") = std::env::args().nth(1).as_deref() {
            // Get project root
            let project_root = std::env::current_dir()?;
            info!("Creating new environment at {}", project_root.display());
            debug!("Setting up directory structure");
            
            // Verify directory structure
            for dir in &[
                ".blast",
                "environments",
                "cache",
                ".blast/state",
                ".blast/logs",
            ] {
                let path = project_root.join(dir);
                if !path.exists() {
                    std::fs::create_dir_all(&path)?;
                }
                debug!("Created directory: {}", dir);
            }

            debug!("Checking state file");
            let state_file = project_root.join(".blast/state.json");
            if !state_file.exists() {
                std::fs::write(&state_file, "{}")?;
            }

            info!("Directory structure ready");
            debug!("Initializing daemon");

            // Initialize daemon
            let daemon = blast_daemon::Daemon::new(blast_daemon::DaemonConfig {
                max_pending_updates: 100,
                max_snapshot_age_days: 7,
                env_path: project_root.join("environments/default"),
                cache_path: project_root.join("cache"),
            }).await?;

            info!("Daemon initialized successfully");
            debug!("Setting up state management");

            // Initialize state manager
            let state_manager = Arc::new(RwLock::new(blast_daemon::StateManager::new(
                project_root.clone(),
            )));

            {
                let state = state_manager.write().map_err(|e| anyhow::anyhow!("Failed to acquire state lock: {}", e))?;
                debug!("Saving initial state");
                state.save().await?;
                debug!("Loading state");
                state.load().await?;
            }

            info!("State management ready");
            debug!("Verifying daemon access");

            // Verify daemon access
            if let Err(e) = daemon.verify_access().await {
                error!("Access verification failed: {}", e);
                std::process::exit(1);
            }

            info!("Daemon access verified");
            debug!("Preparing activation script");

            // Get environment path and name for logging
            let env_path = project_root.join("environments/default");
            debug!("Environment path: {}", env_path.display());

            // Start daemon in background
            info!("Starting background services");
            daemon.start_background().await?;
            
            debug!("Setting up cleanup handlers");
            let (_shutdown_tx, shutdown_rx) = watch::channel(false);

            // Start the daemon in a truly detached way
            tokio::spawn(async move {
                debug!("Background services started");
                let mut rx = shutdown_rx;
                
                loop {
                    tokio::select! {
                        Ok(()) = rx.changed() => {
                            if *rx.borrow() {
                                debug!("Received shutdown signal");
                                break;
                            }
                        }
                        _ = tokio::time::sleep(tokio::time::Duration::from_secs(1)) => {
                            if let Err(e) = daemon.verify_access().await {
                                if !e.to_string().contains("Transaction not found") {
                                    error!("Daemon verification error: {}", e);
                                    break;
                                }
                            }
                        }
                    }
                }
                debug!("Background services stopped");
            });

            info!("Environment activated successfully");
            info!("Blast is ready - use 'blast deactivate' to exit");

            // Output shell activation script
            println!(r#"
export BLAST_ENV="{}"
export BLAST_ENV_NAME="default"
export PATH="{}/bin:$PATH"
export PS1="(blast) $PS1"
"#, 
                project_root.display(),
                env_path.display()
            );

            // Exit immediately after outputting the activation script
            std::process::exit(0);
        } else {
            debug!("Running regular command");
            blast_cli::run().await
        }
    }
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