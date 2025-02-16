//! Command-line interface for the Blast Python environment manager.

use std::path::PathBuf;
use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing::error;

use blast_core::config::BlastConfig;
use blast_core::python::PythonVersion;

mod commands;
mod output;
mod progress;

pub use commands::*;
pub use output::*;
pub use progress::*;

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
    command: Commands,
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
}

/// Run the CLI application
pub async fn run() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    if cli.verbose {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .init();
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
            commands::execute_kill(force, &config).await?;
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
    }

    Ok(())
}

/// Main entry point for the CLI binary
pub fn main() {
    if let Err(e) = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(run())
    {
        error!("Error: {}", e);
        std::process::exit(1);
    }
} 