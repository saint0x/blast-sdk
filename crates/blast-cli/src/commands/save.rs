use std::io::{self, Write};
use blast_core::{
    config::BlastConfig,
    error::{BlastError, BlastResult},
    python::{PythonEnvironment, PythonVersion},
    environment::Environment,
};
use blast_daemon::{Daemon, DaemonConfig, state::StateManagement};
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
            let blast_dir = config.project_root.join(".blast");
            let mut existing_name = None;
            
            if blast_dir.exists() {
                for entry in std::fs::read_dir(&blast_dir)? {
                    let entry = entry?;
                    let path = entry.path();
                    if path.is_file() && path.extension().map_or(false, |ext| ext == "blast") {
                        if let Ok(image) = Image::load(&path, &path).map_err(convert_image_error) {
                            if image.name == env_name {
                                existing_name = Some(env_name.clone());
                                break;
                            }
                        }
                    }
                }
            }
            
            if let Some(name) = existing_name {
                // If there's an existing image, use its name for updating
                name
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

    // Get current environment state
    let state_manager = daemon.state_manager();
    let state_guard = state_manager.read().await;
    let state_manager: &dyn StateManagement = &*state_guard;
    let current_state = state_manager.get_current_state().await?;

    if let Some(active_env_name) = &current_state.active_env_name {
        // Create Python environment instance
        let python_version = current_state.active_python_version
            .clone()
            .unwrap_or_else(|| PythonVersion::parse("3.8.0").unwrap());
            
        let env_path = config.project_root.join("environments").join(active_env_name);
        let env = PythonEnvironment::new(
            active_env_name.clone(),
            env_path.clone(),
            python_version.clone(),
        ).await?;

        // Initialize environment if needed
        Environment::init(&env).await?;

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
        info!("  Environment: {}", active_env_name);
        info!("  Python: {}", python_version);
        info!("  Compression ratio: {:.2}x", image.compression_ratio());
        info!("  Total size: {} bytes", image.size());
        info!("  Path: {}", image_path.display());
    } else {
        warn!("No active environment found");
    }

    Ok(())
} 