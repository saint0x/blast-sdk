//! Command-line interface for the Blast Python environment manager.

use std::path::PathBuf;
use anyhow::Result;
use clap::{Parser, Subcommand};
use once_cell::sync::OnceCell;

use blast_core::config::BlastConfig;
use blast_core::python::PythonVersion;

mod commands;
pub mod output;
mod progress;
pub mod shell;
pub mod setup;

pub use commands::*;
pub use output::*;
pub use progress::*;
pub use setup::initialize;

static LOGGING: OnceCell<()> = OnceCell::new();

fn init_logging(eval_mode: bool) {
    let _ = LOGGING.get_or_init(|| {
        // If we're in script output mode, don't initialize logging at all
        if std::env::var("BLAST_SCRIPT_OUTPUT").is_ok() {
            return;
        }

        // Ensure no color output
        std::env::set_var("NO_COLOR", "1");
        std::env::set_var("CLICOLOR", "0");
        std::env::set_var("CLICOLOR_FORCE", "0");
        std::env::set_var("RUST_LOG_STYLE", "never");

        let builder = tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::from_default_env()
                    .add_directive(if eval_mode {
                        tracing::Level::INFO.into()
                    } else {
                        tracing::Level::DEBUG.into()
                    })
            )
            .with_ansi(false)
            .with_target(false)
            .with_thread_ids(false)
            .with_thread_names(false)
            .with_file(false)
            .with_line_number(false)
            .with_span_events(tracing_subscriber::fmt::format::FmtSpan::NONE)
            .with_timer(())
            .with_writer(std::io::stderr)
            .with_level(false);

        // Try to initialize, but don't panic if it fails
        let _ = builder.try_init();
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
    /// Start a new blast environment
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

    /// Save environment state
    Save {
        /// State name
        #[arg(short, long)]
        name: Option<String>,
    },

    /// Load environment state
    Load {
        /// State name
        #[arg(short, long)]
        name: Option<String>,
    },

    /// List all environments
    List,

    /// Check environment status
    Check,
}

/// Run the CLI application
pub async fn run() -> Result<()> {
    // If we're in script output mode, skip all initialization
    if std::env::var("BLAST_SCRIPT_OUTPUT").is_ok() {
        let cli = Cli::parse();
        match cli.command {
            Commands::Start {
                python,
                name,
                path,
                env,
            } => {
                let current_dir = std::env::current_dir()?;
                let config = BlastConfig::new(
                    current_dir.file_name().unwrap().to_string_lossy().to_string(),
                    "0.1.0",
                    PythonVersion::parse("3.8")?,
                    current_dir,
                );
                commands::execute_start(python, name, path, env, &config).await?;
            }
            _ => {}
        }
        return Ok(());
    }

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
    // Ensure no ANSI colors in output
    std::env::set_var("NO_COLOR", "1");
    std::env::set_var("CLICOLOR", "0");
    std::env::set_var("CLICOLOR_FORCE", "0");

    // Initialize logging with default settings
    init_logging(false);

    if let Err(e) = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(run())
    {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
} 