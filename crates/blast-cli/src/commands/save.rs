use std::io::{self, Write};
use blast_core::{
    config::BlastConfig,
    error::BlastResult,
};
use blast_daemon::Daemon;
use blast_image::{Image, ImageConfig};
use chrono::Utc;
use tracing::{debug, info, warn};

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
            let daemon_config = blast_daemon::DaemonConfig {
                max_pending_updates: 100,
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
    let daemon_config = blast_daemon::DaemonConfig {
        max_pending_updates: 100,
    };

    // Connect to daemon
    let daemon = Daemon::new(daemon_config).await?;

    // Get active environment
    if let Some(env) = daemon.get_active_environment().await? {
        // Create image configuration
        let image_config = ImageConfig::new()
            .with_name(&image_name)
            .with_tag("created", Utc::now().to_rfc3339())
            .with_tag("python_version", env.python_version().to_string())
            .with_tag("env_name", env_name.clone());

        // Create image from environment
        let mut image = Image::create_from_environment_with_config(&env, image_config)?;

        // Save image
        let image_path = blast_dir.join(&image_name);
        image.save(&image_path)?;

        info!("Successfully saved environment image:");
        info!("  Name: {}", image_name);
        info!("  Environment: {}", env_name);
        info!("  Python: {}", env.python_version());
        info!("  Compression ratio: {:.2}x", image.compression_ratio());
        info!("  Total size: {} bytes", image.total_size());
        info!("  Path: {}", image_path.display());
    } else {
        warn!("No active environment found");
    }

    Ok(())
} 