//! Command-line interface for the Blast Python environment manager.

use std::path::PathBuf;
use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing::error;
use once_cell::sync::OnceCell;

use blast_core::config::BlastConfig;
use blast_core::python::PythonVersion;

mod commands;
mod output;
mod progress;
pub mod shell;
mod setup;

pub use commands::*;
pub use output::*;
pub use progress::*;

static LOGGING: OnceCell<()> = OnceCell::new();

fn init_logging(eval_mode: bool) {
    let _ = LOGGING.get_or_init(|| {
        let builder = tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::from_default_env()
                    .add_directive(if eval_mode {
                        tracing::Level::INFO.into()
                    } else {
                        tracing::Level::DEBUG.into()
                    })
            );

        // Configure based on mode
        let builder = if eval_mode {
            builder
                .with_target(false)
                .with_ansi(false)
                .with_thread_ids(false)
                .with_thread_names(false)
                .with_file(false)
                .with_line_number(false)
                .with_span_events(tracing_subscriber::fmt::format::FmtSpan::NONE)
        } else {
            builder
                .with_target(false)
                .with_ansi(true)
                .with_thread_ids(true)
                .with_thread_names(true)
                .with_file(true)
                .with_line_number(true)
                .with_span_events(tracing_subscriber::fmt::format::FmtSpan::ACTIVE)
        };

        builder.try_init().expect("Failed to initialize logging");
    });
}

/// CLI arguments parser
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Config file path
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Start a new blast environment (stacks if one exists)
    Start {
        /// Python version to use
        #[arg(short, long)]
        python: Option<String>,

        /// Environment name
        #[arg(short, long)]
        name: Option<String>,

        /// Environment path
        #[arg(short, long)]
        path: Option<PathBuf>,

        /// Additional environment variables
        #[arg(short, long)]
        env: Vec<String>,
    },

    /// Kill the current blast environment
    Kill {
        /// Force kill without graceful shutdown
        #[arg(short, long)]
        force: bool,
    },

    /// Clean and reinstall all dependencies
    Clean,

    /// Save environment image
    Save {
        /// Image name
        #[arg(short, long)]
        name: Option<String>,
    },

    /// Load environment image
    Load {
        /// Image name
        #[arg(short, long)]
        name: Option<String>,
    },

    /// List all environments from most to least recently used
    List,

    /// Check environment status and health
    Check,
}

/// Run the CLI application
pub async fn run() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    init_logging(cli.verbose);

    // Check for first run and initialize if needed
    if !is_initialized() {
        setup::initialize()?;
        mark_as_initialized()?;
    }

    // Load or create config
    let config = if let Some(path) = cli.config {
        BlastConfig::from_file(path)?
    } else {
        let current_dir = std::env::current_dir()?;
        BlastConfig::new(
            current_dir.file_name().unwrap().to_string_lossy().to_string(),
            "0.1.0",
            PythonVersion::parse("3.8")?,
            current_dir,
        )
    };

    // Execute command
    match cli.command {
        Commands::Start {
            python,
            name,
            path,
            env,
        } => {
            commands::execute_start(python, name, path, env, &config).await?;
        }
        Commands::Kill { force } => {
            let env_name = std::env::var("BLAST_ENV_NAME")
                .unwrap_or_else(|_| "default".to_string());
            commands::execute_kill(env_name, force, &config).await?;
        }
        Commands::Clean => {
            commands::execute_clean(&config).await?;
        }
        Commands::Save { name } => {
            commands::execute_save(name, &config).await?;
        }
        Commands::Load { name } => {
            commands::execute_load(name, &config).await?;
        }
        Commands::List => {
            commands::execute_list(&config).await?;
        }
        Commands::Check => {
            commands::execute_check(&config).await?;
        }
    }

    Ok(())
}

fn is_initialized() -> bool {
    let config_dir = dirs::config_dir()
        .map(|d| d.join("blast"))
        .unwrap_or_else(|| PathBuf::from(".blast"));
    
    config_dir.join(".initialized").exists()
}

fn mark_as_initialized() -> Result<()> {
    let config_dir = dirs::config_dir()
        .map(|d| d.join("blast"))
        .unwrap_or_else(|| PathBuf::from(".blast"));
    
    std::fs::create_dir_all(&config_dir)?;
    std::fs::write(config_dir.join(".initialized"), "")?;
    Ok(())
}

/// Main entry point for the CLI binary
pub fn main() {
    // Initialize logging with default settings
    init_logging(false);

    if let Err(e) = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(run())
    {
        error!("Error: {}", e);
        std::process::exit(1);
    }
} 