# Jockey Image

Generated: 02-17-2025 at 19:44:24

## Repository Structure

```
blast-cli
│   ├── Cargo.toml
    └── src
    │   ├── output
    │       └── logger.rs
    │   ├── commands
    │   ├── lib.rs
    │   ├── output.rs
        └── progress.rs
```

## File: /Users/saint/Desktop/blast-rs/crates/blast-cli/Cargo.toml

```toml
[package]
name = "blast-cli"
version = "0.1.0"
edition = "2021"
authors = ["Blast Contributors"]
description = "Command-line interface for the Blast Python environment manager"
license = "MIT"

[dependencies]
# Internal dependencies
blast-core = { path = "../blast-core" }
blast-daemon = { path = "../blast-daemon" }
blast-image = { path = "../blast-image" }
blast-resolver = { path = "../blast-resolver" }

# CLI
clap = { version = "4.4", features = ["derive"] }
console = "0.15"
dialoguer = "0.11"
indicatif = "0.17"

# Error handling
anyhow = "1.0"
thiserror = "1.0"

# Async runtime
tokio = { version = "1.0", features = ["full"] }

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Time handling
chrono = { version = "0.4", features = ["serde"] }
humantime = "2.1"

# Utilities
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
uuid = { version = "1.0", features = ["v4", "serde"] }

[dev-dependencies]
assert_cmd = "2.0"
predicates = "3.0"
tempfile = { workspace = true } 
```

## File: /Users/saint/Desktop/blast-rs/crates/blast-cli/src/lib.rs

```rs
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

    /// Check environment status and health
    Check,
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
        Commands::Check => {
            commands::execute_check(&config).await?;
        }
    }

    Ok(())
}

/// Main entry point for the CLI binary
pub fn main() {
    // Set up logging with better formatting
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env()
            .add_directive(tracing::Level::INFO.into()))
        .with_target(false)
        .with_thread_ids(false)
        .with_line_number(false)
        .with_file(false)
        .init();

    if let Err(e) = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(run())
    {
        error!("Error: {}", e);
        std::process::exit(1);
    }
} 
```

## File: /Users/saint/Desktop/blast-rs/crates/blast-cli/src/output/logger.rs

```rs
use console::{style, Term};
use std::fmt::Display;

pub struct Logger {
    term: Term,
}

#[derive(Debug, Clone, Copy)]
pub enum HealthStatus {
    Good,
    Okay,
    Bad,
}

impl HealthStatus {
    pub fn from_resource_usage(cpu_percent: f32, memory_percent: f32, disk_percent: f32) -> Self {
        if cpu_percent > 90.0 || memory_percent > 90.0 || disk_percent > 90.0 {
            HealthStatus::Bad
        } else if cpu_percent > 70.0 || memory_percent > 70.0 || disk_percent > 70.0 {
            HealthStatus::Okay
        } else {
            HealthStatus::Good
        }
    }

    pub fn color(&self) -> console::Style {
        match self {
            HealthStatus::Good => style().green(),
            HealthStatus::Okay => style().yellow(),
            HealthStatus::Bad => style().red(),
        }
    }
}

impl Display for HealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HealthStatus::Good => write!(f, "good"),
            HealthStatus::Okay => write!(f, "okay"),
            HealthStatus::Bad => write!(f, "bad"),
        }
    }
}

impl Logger {
    pub fn new() -> Self {
        Self {
            term: Term::stdout(),
        }
    }

    pub fn header(&self, text: &str) {
        let width = self.term.size().1 as usize;
        let padding = "=".repeat((width - text.len() - 2) / 2);
        println!("\n{} {} {}\n", padding, style(text).bold(), padding);
    }

    pub fn section(&self, text: &str) {
        println!("\n{}", style(text).bold().underlined());
    }

    pub fn status(&self, label: &str, status: HealthStatus) {
        println!("{}: {}", 
            style(label).bold(),
            status.color().apply_to(status.to_string())
        );
    }

    pub fn info(&self, label: &str, value: impl Display) {
        println!("{}: {}", style(label).bold(), value);
    }

    pub fn resource(&self, label: &str, used: u64, total: u64) {
        let percentage = (used as f32 / total as f32) * 100.0;
        let status = if percentage > 90.0 {
            HealthStatus::Bad
        } else if percentage > 70.0 {
            HealthStatus::Okay
        } else {
            HealthStatus::Good
        };

        println!("{}: {} / {} ({:.1}%) {}",
            style(label).bold(),
            self.format_bytes(used),
            self.format_bytes(total),
            percentage,
            status.color().apply_to("●")
        );
    }

    pub fn warning(&self, text: impl Display) {
        println!("{} {}", 
            style("WARNING:").yellow().bold(),
            text
        );
    }

    pub fn error(&self, text: impl Display) {
        println!("{} {}", 
            style("ERROR:").red().bold(),
            text
        );
    }

    fn format_bytes(&self, bytes: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;

        if bytes >= GB {
            format!("{:.1}GB", bytes as f64 / GB as f64)
        } else if bytes >= MB {
            format!("{:.1}MB", bytes as f64 / MB as f64)
        } else if bytes >= KB {
            format!("{:.1}KB", bytes as f64 / KB as f64)
        } else {
            format!("{}B", bytes)
        }
    }
} 
```

## File: /Users/saint/Desktop/blast-rs/crates/blast-cli/src/output.rs

```rs
//! Output formatting utilities for CLI

use console::style;
use blast_core::{
    package::Package,
    version::Version,
};

/// Format a package for display
pub fn format_package(package: &Package) -> String {
    format!(
        "{} {}",
        style(package.name()).green(),
        style(package.version()).yellow()
    )
}

/// Format a version for display
pub fn format_version(version: &Version) -> String {
    style(version.to_string()).yellow().to_string()
}

/// Format a dependency tree
pub fn format_dependency_tree(package: &Package, depth: usize) -> String {
    let mut output = String::new();
    let indent = "  ".repeat(depth);
    
    output.push_str(&format!(
        "{}{}",
        indent,
        format_package(package)
    ));

    for (name, constraint) in package.metadata().dependencies.iter() {
        output.push_str(&format!(
            "\n{}└── {} {}",
            indent,
            style(name).blue(),
            style(constraint).dim()
        ));
    }

    output
}

/// Format an error message
pub fn format_error(msg: &str) -> String {
    style(format!("Error: {}", msg)).red().to_string()
}

/// Format a success message
pub fn format_success(msg: &str) -> String {
    style(format!("Success: {}", msg)).green().to_string()
}

/// Format a warning message
pub fn format_warning(msg: &str) -> String {
    style(format!("Warning: {}", msg)).yellow().to_string()
}

/// Format an info message
pub fn format_info(msg: &str) -> String {
    style(msg).blue().to_string()
} 
```

## File: /Users/saint/Desktop/blast-rs/crates/blast-cli/src/progress.rs

```rs
//! Progress tracking utilities for CLI operations

use std::time::Duration;
use indicatif::{ProgressBar, ProgressStyle};
use console::style;
use blast_core::package::Package;

/// Manages progress bars for concurrent operations
pub struct ProgressManager {
    resolution_spinner: Option<ProgressBar>,
    installation_progress: Option<ProgressBar>,
}

impl ProgressManager {
    /// Create a new progress manager
    pub fn new() -> Self {
        Self {
            resolution_spinner: None,
            installation_progress: None,
        }
    }

    /// Start the resolution process
    pub fn start_resolution(&mut self) {
        let spinner = ProgressBar::new_spinner();
        spinner.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner} {msg}")
                .unwrap(),
        );
        spinner.set_message("Resolving dependencies...");
        spinner.enable_steady_tick(Duration::from_millis(100));
        self.resolution_spinner = Some(spinner);
    }

    /// Finish the resolution process
    pub fn finish_resolution(&mut self) {
        if let Some(spinner) = self.resolution_spinner.take() {
            spinner.finish_with_message("Dependencies resolved");
        }
    }

    /// Start the installation process
    pub fn start_installation(&mut self, total: usize) {
        let progress = ProgressBar::new(total as u64);
        progress.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
                .unwrap()
                .progress_chars("#>-"),
        );
        progress.set_message("Installing packages...");
        self.installation_progress = Some(progress);
    }

    /// Set the progress for a specific package
    pub fn set_package(&mut self, package: &Package) {
        if let Some(progress) = &self.installation_progress {
            progress.set_message(format!("Installing {}", style(package.id()).cyan()));
        }
    }

    /// Increment the installation progress
    pub fn increment(&mut self) {
        if let Some(progress) = &self.installation_progress {
            progress.inc(1);
        }
    }

    /// Finish the installation process
    pub fn finish_installation(&mut self) {
        if let Some(progress) = self.installation_progress.take() {
            progress.finish_with_message("Installation complete");
        }
    }
}

impl Default for ProgressManager {
    fn default() -> Self {
        Self::new()
    }
} 
```

## File: /Users/saint/Desktop/blast-rs/crates/blast-cli/src/commands/deactivate.rs

```rs
use blast_core::{
    config::BlastConfig,
    error::BlastResult,
};
use blast_daemon::{Daemon, DaemonConfig};
use tracing::{debug, info};

pub async fn execute(config: &BlastConfig) -> BlastResult<()> {
    debug!("Deactivating environment");

    // Create daemon configuration
    let daemon_config = DaemonConfig {
        max_pending_updates: 100,
        max_snapshot_age_days: 7,
        env_path: config.project_root.join("environments/default"),
        cache_path: config.project_root.join("cache"),
    };

    // Connect to daemon
    let daemon = Daemon::new(daemon_config).await?;

    // Deactivate current environment
    daemon.deactivate_environment().await?;

    // Get current state to show what was deactivated
    let state_manager = daemon.state_manager();
    let current_state = state_manager.read().await.get_current_state();

    info!("Deactivated environment: {}", current_state.name());
    info!("Run 'blast list' to see available environments");

    Ok(())
} 
```

## File: /Users/saint/Desktop/blast-rs/crates/blast-cli/src/commands/start.rs

```rs
use std::path::PathBuf;
use blast_core::{
    config::BlastConfig,
    error::BlastResult,
    python::PythonVersion,
    security::{SecurityPolicy, IsolationLevel, ResourceLimits},
    state::EnvironmentState,
};
use blast_daemon::{
    DaemonConfig,
    Daemon,
};
use tracing::{debug, info};
use uuid::Uuid;

const SHELL_ACTIVATION_SCRIPT: &str = r#"
# Store old environment state
_OLD_PATH="$PATH"
_OLD_PS1="${PS1-}"
_OLD_PYTHON_PATH="${PYTHONPATH-}"

# Set up blast environment
export BLAST_ENV="{env_path}"
export BLAST_ENV_NAME="{env_name}"
export BLAST_PYTHON_VERSION="{python_version}"
export BLAST_ENV_PATH="{env_path}"
export PATH="{env_path}/bin:$PATH"
export PYTHONPATH="{env_path}/lib/python{python_major}/site-packages:$PYTHONPATH"

# Define the blast function for environment management
blast() {
    if [ "$1" = "deactivate" ]; then
        command blast deactivate > /dev/null 2>&1
        export PATH="$_OLD_PATH"
        export PS1="$_OLD_PS1"
        export PYTHONPATH="$_OLD_PYTHON_PATH"
        unset BLAST_ENV BLAST_ENV_NAME BLAST_PYTHON_VERSION BLAST_ENV_PATH _OLD_PATH _OLD_PS1 _OLD_PYTHON_PATH
        unset -f blast
    else
        BLAST_ACTIVE_ENV="{env_name}" BLAST_ENV_PATH="{env_path}" command blast "$@"
    fi
}

# Set up shell prompt
if [ -n "$ZSH_VERSION" ]; then
    setopt PROMPT_SUBST
    _OLD_PROMPT_COMMAND=${PROMPT_COMMAND:-}
    _blast_update_prompt() { PS1="(blast) ${PS1#\(blast\) }"; }
    precmd_functions+=(_blast_update_prompt)
elif [ -n "$BASH_VERSION" ]; then
    _OLD_PROMPT_COMMAND=${PROMPT_COMMAND:-}
    _blast_update_prompt() { PS1="(blast) ${PS1#\(blast\) }"; }
    PROMPT_COMMAND="_blast_update_prompt;${_OLD_PROMPT_COMMAND}"
else
    PS1="(blast) $PS1"
fi

hash -r 2>/dev/null"#;

/// Execute the start command
pub async fn execute(
    python: Option<String>,
    name: Option<String>,
    path: Option<PathBuf>,
    _env_vars: Vec<String>,
    config: &BlastConfig,
) -> BlastResult<()> {
    debug!("Starting environment creation");
    
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
    let env_state = EnvironmentState::new(
        env_name.clone(),
        python_version.clone(),
        Default::default(),
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
    )?;
    debug!("Created initial checkpoint");

    // Generate shell script
    if std::env::var("BLAST_EVAL").is_ok() {
        let script = SHELL_ACTIVATION_SCRIPT
            .replace("{env_path}", &env.path().display().to_string())
            .replace("{env_name}", &env_name)
            .replace("{python_version}", &env.python_version().to_string())
            .replace("{python_major}", &env.python_version().major().to_string());

        println!("{}", script);
    }

    info!("Environment ready: {} (Python {})", env_name, python_version);
    Ok(())
} 
```

## File: /Users/saint/Desktop/blast-rs/crates/blast-cli/src/commands/check.rs

```rs
use blast_core::{
    config::BlastConfig,
    error::BlastResult,
};
use blast_daemon::{Daemon, DaemonConfig};
use tracing::{debug, info};

/// Execute the check command
pub async fn execute(config: &BlastConfig) -> BlastResult<()> {
    debug!("Checking environment status");

    // Create daemon configuration
    let daemon_config = DaemonConfig {
        max_pending_updates: 100,
        max_snapshot_age_days: 7,
        env_path: config.project_root.join("environments/default"),
        cache_path: config.project_root.join("cache"),
    };

    // Connect to daemon
    let daemon = Daemon::new(daemon_config).await?;

    // Get current environment state
    let state_manager = daemon.state_manager();
    let state_guard = state_manager.read().await;
    let current_state = state_guard.get_current_state();

    // Get performance metrics
    let metrics = daemon.get_performance_metrics().await?;

    info!("Environment Status:");
    if current_state.name() == "default" {
        info!("  No active environment");
        return Ok(());
    }

    // Show environment details
    info!("  Name: {}", current_state.name());
    info!("  Python: {}", current_state.python_version);
    info!("  Status: {}", if current_state.is_active() { "Active" } else { "Inactive" });
    info!("  Packages: {}", current_state.packages.len());

    // Show performance metrics
    info!("\nPerformance Metrics:");
    info!("  Average pip install time: {:?}", metrics.avg_pip_install_time);
    info!("  Average sync time: {:?}", metrics.avg_sync_time);
    info!("  Cache hit rate: {:.1}%", metrics.cache_hit_rate * 100.0);

    // Show verification status
    if let Ok(verification) = state_guard.verify_state() {
        info!("\nVerification Status:");
        info!("  Verified: {}", verification.is_verified);
        if !verification.issues.is_empty() {
            info!("  Issues:");
            for issue in verification.issues {
                info!("    - {} ({:?})", issue.description, issue.severity);
                if let Some(context) = issue.context {
                    info!("      Context: {}", context);
                }
                if let Some(recommendation) = issue.recommendation {
                    info!("      Recommendation: {}", recommendation);
                }
            }
        }
    }

    Ok(())
} 
```

## File: /Users/saint/Desktop/blast-rs/crates/blast-cli/src/commands/list.rs

```rs
use blast_core::{
    config::BlastConfig,
    error::BlastResult,
};
use blast_daemon::{Daemon, DaemonConfig};
use tracing::{debug, info};

/// Execute the list command
pub async fn execute(config: &BlastConfig) -> BlastResult<()> {
    debug!("Listing environments");

    // Create daemon configuration with resolved paths
    let daemon_config = DaemonConfig {
        max_pending_updates: 100,
        max_snapshot_age_days: 7,
        env_path: config.project_root.join("environments"),
        cache_path: config.project_root.join("cache"),
    };

    // Connect to daemon
    let daemon = Daemon::new(daemon_config).await?;

    // Get state manager and ensure it's loaded
    let state_manager = daemon.state_manager();
    let state = state_manager.read().await;
    state.load().await?;

    // Get active environment and environment list
    let active_env = state.get_active_environment().await?;
    let environments = state.list_environments().await?;
    debug!("Retrieved environment list");

    info!("Blast environments:");
    if environments.is_empty() {
        info!("  No environments found");
        return Ok(());
    }

    for (name, env) in environments {
        let status = if let Some(active) = &active_env {
            // Use the name getter method
            if name == active.name() {
                "*active*"
            } else {
                ""
            }
        } else {
            ""
        };

        // Get environment path relative to project root
        let env_path = state.get_environment_path(&name);
        let path_display = env_path.strip_prefix(&config.project_root)
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| env_path.display().to_string());

        info!(
            "  {} {} (Python {}) [{}]",
            name,
            status,
            env.python_version,
            path_display
        );
    }

    Ok(())
} 
```

## File: /Users/saint/Desktop/blast-rs/crates/blast-cli/src/commands/clean.rs

```rs
use blast_core::{
    config::BlastConfig,
    error::BlastResult,
};
use blast_daemon::{Daemon, DaemonConfig};
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

    // Get active environment
    if let Some(env) = daemon.get_active_environment().await? {
        info!("Cleaning environment: {}", env_name);
        
        // Save current state for recovery if needed
        daemon.save_environment_state(&env).await?;
        
        // Remove all packages
        daemon.clean_environment(&env).await?;
        
        // Reinitialize environment
        daemon.reinitialize_environment(&env).await?;
        
        // Restore essential packages
        daemon.restore_essential_packages(&env).await?;

        info!("Environment cleaned and reinitialized");
    } else {
        info!("No active environment found");
    }

    Ok(())
} 
```

## File: /Users/saint/Desktop/blast-rs/crates/blast-cli/src/commands/register.rs

```rs
use std::path::PathBuf;
use blast_core::{
    config::BlastConfig,
    error::BlastResult,
    state::EnvironmentState,
    python::PythonVersion,
};
use blast_daemon::{Daemon, DaemonConfig};
use tracing::{debug, info};
use std::collections::HashMap;

pub async fn execute(
    name: String,
    path: PathBuf,
    config: &BlastConfig,
) -> BlastResult<()> {
    debug!("Registering environment: {} at {}", name, path.display());

    // Create daemon configuration
    let daemon_config = DaemonConfig {
        max_pending_updates: 100,
        max_snapshot_age_days: 7,
        env_path: path.clone(),
        cache_path: config.project_root.join("cache"),
    };

    // Connect to daemon
    let daemon = Daemon::new(daemon_config).await?;

    // Get Python version from environment
    let python_version = if let Ok(version) = std::env::var("BLAST_PYTHON_VERSION") {
        PythonVersion::parse(&version)?
    } else {
        // Default to Python 3.8 if not specified
        PythonVersion::parse("3.8")?
    };

    // Create environment state
    let env_state = EnvironmentState::new(
        name.clone(),
        python_version,
        HashMap::new(), // Empty packages initially
        HashMap::new(), // Empty env vars initially
    );

    // Update state manager
    let state_manager = daemon.state_manager();
    state_manager.write().await.update_current_state(env_state.clone())?;

    // Register as active environment
    daemon.register_active_environment(name.clone()).await?;

    info!("Successfully registered environment:");
    info!("  Name: {}", name);
    info!("  Path: {}", path.display());
    info!("  Python: {}", python_version);

    Ok(())
} 
```

## File: /Users/saint/Desktop/blast-rs/crates/blast-cli/src/commands/save.rs

```rs
use std::io::{self, Write};
use blast_core::{
    config::BlastConfig,
    error::{BlastError, BlastResult},
};
use blast_daemon::{Daemon, DaemonConfig};
use blast_image::{
    layer::{Layer as Image, LayerType},
    compression::{CompressionType, CompressionLevel},
    error::Error as ImageError,
};
use tracing::{debug, info, warn};

// Helper function to convert ImageError to BlastError
fn convert_image_error(err: ImageError) -> BlastError {
    BlastError::environment(err.to_string())
}

/// Execute the save command
pub async fn execute(name: Option<String>, config: &BlastConfig) -> BlastResult<()> {
    // Check if we're in a blast environment
    let env_name = match std::env::var("BLAST_ENV_NAME") {
        Ok(name) => name,
        Err(_) => {
            warn!("Not in a blast environment");
            return Ok(());
        }
    };

    // Get image name from parameter or prompt user
    let image_name = match name {
        Some(n) => n,
        None => {
            // Check if there's an existing image for this environment
            let daemon_config = DaemonConfig {
                max_pending_updates: 100,
                max_snapshot_age_days: 7,
                env_path: config.project_root.join("environments/default"),
                cache_path: config.project_root.join("cache"),
            };
            let daemon = Daemon::new(daemon_config).await?;
            let existing_images = daemon.list_environment_images(&env_name).await?;
            
            if let Some(existing) = existing_images.first() {
                // If there's an existing image, use its name for updating
                existing.name.clone()
            } else {
                // Prompt for name
                print!("Enter image name: ");
                io::stdout().flush()?;
                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                input.trim().to_string()
            }
        }
    };

    debug!("Saving environment image: {}", image_name);

    // Create .blast directory if it doesn't exist
    let blast_dir = config.project_root.join(".blast");
    if !blast_dir.exists() {
        std::fs::create_dir_all(&blast_dir)?;
    }

    // Create daemon configuration
    let daemon_config = DaemonConfig {
        max_pending_updates: 100,
        max_snapshot_age_days: 7,
        env_path: config.project_root.join("environments/default"),
        cache_path: config.project_root.join("cache"),
    };

    // Connect to daemon
    let daemon = Daemon::new(daemon_config).await?;

    // Get active environment
    if let Some(env) = daemon.get_active_environment().await? {
        // Create image from environment with options
        let mut image = Image::from_environment_with_options(
            &env,
            LayerType::Packages,
            CompressionType::Zstd,
            CompressionLevel::default(),
        ).map_err(convert_image_error)?;

        // Save image
        let image_path = blast_dir.join(&image_name);
        image.save(&image_path).map_err(convert_image_error)?;

        info!("Successfully saved environment image:");
        info!("  Name: {}", image_name);
        info!("  Environment: {}", env_name);
        info!("  Python: {}", env.python_version());
        info!("  Compression ratio: {:.2}x", image.compression_ratio());
        info!("  Total size: {} bytes", image.size());
        info!("  Path: {}", image_path.display());
    } else {
        warn!("No active environment found");
    }

    Ok(())
} 
```

## File: /Users/saint/Desktop/blast-rs/crates/blast-cli/src/commands/mod.rs

```rs
//! CLI command implementations

pub mod start;
pub mod kill;
pub mod clean;
pub mod save;
pub mod load;
pub mod list;
pub mod check;

// Export command functions with clear names
pub use start::execute as execute_start;
pub use kill::execute as execute_kill;
pub use clean::execute as execute_clean;
pub use save::execute as execute_save;
pub use load::execute as execute_load;
pub use list::execute as execute_list;
pub use check::execute as execute_check; 
```

## File: /Users/saint/Desktop/blast-rs/crates/blast-cli/src/commands/load.rs

```rs
use std::io::{self, Write};
use blast_core::{
    config::BlastConfig,
    error::{BlastError, BlastResult},
};
use blast_daemon::{Daemon, DaemonConfig};
use blast_image::{
    layer::Layer as Image,
    error::Error as ImageError,
};
use tracing::{debug, info, warn};

// Helper function to convert ImageError to BlastError
fn convert_image_error(err: ImageError) -> BlastError {
    BlastError::environment(err.to_string())
}

/// Execute the load command
pub async fn execute(name: Option<String>, config: &BlastConfig) -> BlastResult<()> {
    // Check if we're already in a blast environment
    if let Ok(current_env) = std::env::var("BLAST_ENV_NAME") {
        warn!("Already in blast environment: {}", current_env);
        warn!("Please run 'blast kill' first");
        return Ok(());
    }

    // Create daemon configuration
    let daemon_config = DaemonConfig {
        max_pending_updates: 100,
        max_snapshot_age_days: 7,
        env_path: config.project_root.join("environments/default"),
        cache_path: config.project_root.join("cache"),
    };

    // Connect to daemon
    let daemon = Daemon::new(daemon_config).await?;

    // Get list of available images
    let blast_dir = config.project_root.join(".blast");
    let available_images = daemon.list_all_images().await?;

    if available_images.is_empty() {
        warn!("No saved images found");
        return Ok(());
    }

    // Determine which image to load
    let image_name = match name {
        Some(n) => {
            // Verify the specified image exists
            if !available_images.iter().any(|img| img.name == n) {
                warn!("Image not found: {}", n);
                return Ok(());
            }
            n
        }
        None => {
            if available_images.len() == 1 {
                // Auto-load if only one image exists
                available_images[0].name.clone()
            } else {
                // Show available images and prompt for selection
                info!("Available images:");
                for (i, img) in available_images.iter().enumerate() {
                    info!(
                        "  {}. {} (created: {}, Python {})",
                        i + 1,
                        img.name,
                        img.created.format("%Y-%m-%d %H:%M:%S"),
                        img.python_version
                    );
                }

                print!("Enter image number to load (1-{}): ", available_images.len());
                io::stdout().flush()?;
                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                
                match input.trim().parse::<usize>() {
                    Ok(n) if n > 0 && n <= available_images.len() => {
                        available_images[n - 1].name.clone()
                    }
                    _ => {
                        warn!("Invalid selection");
                        return Ok(());
                    }
                }
            }
        }
    };

    let image_path = blast_dir.join(&image_name);
    debug!("Loading image: {}", image_name);

    // Load image
    let image = Image::load(&image_path, &image_path).map_err(convert_image_error)?;
    
    // Create environment from image
    let env = daemon.create_environment_from_image(&image).await?;

    // Set up environment variables
    std::env::set_var("BLAST_ENV_NAME", env.name().unwrap_or("unnamed"));
    std::env::set_var("BLAST_ENV_PATH", env.path().display().to_string());
    std::env::set_var("BLAST_SOCKET_PATH", format!("/tmp/blast_{}.sock", image_name));

    // Set up shell prompt
    if let Ok(shell) = std::env::var("SHELL") {
        if shell.contains("bash") {
            std::env::set_var("PS1", "(blast) $PS1");
        } else if shell.contains("zsh") {
            std::env::set_var("PROMPT", "(blast) $PROMPT");
        }
    }

    info!("Successfully loaded environment image:");
    info!("  Name: {}", image_name);
    info!("  Python: {}", env.python_version());
    info!("  Path: {}", env.path().display());
    info!("  Created: {}", image.metadata.created_at.to_rfc3339());

    Ok(())
} 
```

## File: /Users/saint/Desktop/blast-rs/crates/blast-cli/src/commands/kill.rs

```rs
use blast_core::{
    config::BlastConfig,
    error::BlastResult,
};
use blast_daemon::{Daemon, DaemonConfig};
use tracing::{debug, info, warn};

/// Execute the kill command
pub async fn execute(
    force: bool,
    config: &BlastConfig,
) -> BlastResult<()> {
    debug!("Killing blast environment");
    debug!("Force: {}", force);

    // Check if we're in a blast environment
    let env_name = match std::env::var("BLAST_ENV_NAME") {
        Ok(name) => name,
        Err(_) => {
            warn!("Not in a blast environment");
            return Ok(());
        }
    };

    let env_path = match std::env::var("BLAST_ENV_PATH") {
        Ok(path) => path,
        Err(_) => {
            warn!("Environment path not found");
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

    // Get active environment
    if let Some(env) = daemon.get_active_environment().await? {
        if force {
            // Force kill
            daemon.destroy_environment(&env).await?;
        } else {
            // Graceful shutdown
            info!("Gracefully shutting down environment");
            
            // Save state if needed
            daemon.save_environment_state(&env).await?;
            
            // Stop monitoring
            daemon.stop_monitoring(&env).await?;
            
            // Destroy environment
            daemon.destroy_environment(&env).await?;
        }

        // Clean up environment variables
        std::env::remove_var("BLAST_ENV_NAME");
        std::env::remove_var("BLAST_ENV_PATH");
        std::env::remove_var("BLAST_SOCKET_PATH");

        // Restore shell prompt
        if let Ok(shell) = std::env::var("SHELL") {
            if shell.contains("bash") {
                std::env::remove_var("PS1");
            } else if shell.contains("zsh") {
                std::env::remove_var("PROMPT");
            }
        }

        info!("Killed blast environment:");
        info!("  Name: {}", env_name);
        info!("  Path: {}", env_path);
    } else {
        warn!("Environment not found: {}", env_name);
    }

    Ok(())
} 
```



---

> 📸 Generated with [Jockey CLI](https://github.com/saint0x/jockey-cli)
