use std::io::{self, Write};
use blast_core::{
    config::BlastConfig,
    error::BlastResult,
};
use blast_daemon::Daemon;
use blast_image::Image;
use tracing::{debug, info, warn};

/// Execute the load command
pub async fn execute(name: Option<String>, config: &BlastConfig) -> BlastResult<()> {
    // Check if we're already in a blast environment
    if let Ok(current_env) = std::env::var("BLAST_ENV_NAME") {
        warn!("Already in blast environment: {}", current_env);
        warn!("Please run 'blast kill' first");
        return Ok(());
    }

    // Create daemon configuration
    let daemon_config = blast_daemon::DaemonConfig {
        max_pending_updates: 100,
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
    let image = Image::load(&image_path)?;
    
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

    // Restore image environment variables
    for (key, value) in image.metadata().env_vars.iter() {
        std::env::set_var(key, value);
    }

    info!("Successfully loaded environment image:");
    info!("  Name: {}", image_name);
    info!("  Python: {}", env.python_version());
    info!("  Path: {}", env.path().display());
    info!("  Created: {}", image.metadata().tags.iter()
        .find(|t| t.starts_with("created="))
        .map(|t| t.split('=').nth(1).unwrap_or("unknown"))
        .unwrap_or("unknown"));

    Ok(())
} 