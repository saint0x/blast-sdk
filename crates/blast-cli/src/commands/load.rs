use std::io::{self, Write};
use blast_core::{
    config::BlastConfig,
    error::{BlastError, BlastResult},
    security::SecurityPolicy,
    environment::Environment,
    python::PythonVersion,
};
use blast_daemon::{
    Daemon, 
    DaemonConfig,
};
use blast_image::{
    layer::Layer as Image,
    error::Error as ImageError,
    Manifest,
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

    // Get list of available images from the .blast directory
    let blast_dir = config.project_root.join(".blast");
    let mut available_images = Vec::new();
    let mut manifests = Vec::new();

    if blast_dir.exists() {
        for entry in std::fs::read_dir(&blast_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |ext| ext == "blast") {
                if let Ok(image) = Image::load(&path, &path).map_err(convert_image_error) {
                    // Load manifest from the same directory
                    let manifest_path = path.with_extension("toml");
                    if let Ok(manifest) = Manifest::load(manifest_path).await {
                        available_images.push(image);
                        manifests.push(manifest);
                    }
                }
            }
        }
    }

    if available_images.is_empty() {
        warn!("No saved images found");
        return Ok(());
    }

    // Determine which image to load
    let (image_idx, image_name) = match name {
        Some(n) => {
            // Verify the specified image exists
            if let Some((idx, _)) = available_images.iter().enumerate()
                .find(|(_, img)| img.name == n) {
                (idx, n)
            } else {
                warn!("Image not found: {}", n);
                return Ok(());
            }
        }
        None => {
            if available_images.len() == 1 {
                // Auto-load if only one image exists
                (0, available_images[0].name.clone())
            } else {
                // Show available images and prompt for selection
                info!("Available images:");
                for (i, (img, manifest)) in available_images.iter().zip(manifests.iter()).enumerate() {
                    info!(
                        "  {}. {} (created: {}, Python {})",
                        i + 1,
                        img.name,
                        img.metadata.created_at.format("%Y-%m-%d %H:%M:%S"),
                        manifest.python_version
                    );
                }

                print!("Enter image number to load (1-{}): ", available_images.len());
                io::stdout().flush()?;
                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                
                match input.trim().parse::<usize>() {
                    Ok(n) if n > 0 && n <= available_images.len() => {
                        (n - 1, available_images[n - 1].name.clone())
                    }
                    _ => {
                        warn!("Invalid selection");
                        return Ok(());
                    }
                }
            }
        }
    };

    let mut image = available_images.into_iter().nth(image_idx).unwrap();
    let manifest = manifests.into_iter().nth(image_idx).unwrap();
    
    debug!("Loading image: {}", image_name);

    // Create environment with image's Python version
    let policy = SecurityPolicy {
        python_version: PythonVersion::parse(&manifest.python_version)?,
        ..SecurityPolicy::default()
    };
    
    let env = daemon.create_environment(&policy).await?;

    // Apply image contents to environment
    image.save(&env.path()).map_err(convert_image_error)?;

    // TODO: Implement environment activation
    // For now, just set up the environment variables and state
    let state_manager = daemon.state_manager();
    let state_manager = state_manager.write().await;
    let state_manager: &dyn blast_daemon::state::StateManagement = &*state_manager;
    state_manager.set_active_environment(
        image_name.clone(),
        env.path().to_path_buf(),
        policy.python_version.clone()
    ).await?;

    // Set up environment variables
    std::env::set_var("BLAST_ENV_NAME", &image_name);
    std::env::set_var("BLAST_ENV_PATH", env.path().display().to_string());
    std::env::set_var("BLAST_SOCKET_PATH", format!("/tmp/blast_{}.sock", &image_name));

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