mod types;
pub use types::*;

use std::path::PathBuf;
use blast_core::{
    error::BlastResult,
    python::PythonEnvironment,
    environment::Environment as CoreEnvironment,
    EnvironmentManager as CoreEnvironmentManager,
    config::BlastConfig,
    ActivationScripts,
};
use crate::error::DaemonError;

pub struct EnvManager {
    root_path: PathBuf,
}

impl EnvManager {
    pub fn new(root_path: PathBuf) -> Self {
        Self { root_path }
    }

    pub fn root_path(&self) -> &PathBuf {
        &self.root_path
    }
}

#[async_trait::async_trait]
impl CoreEnvironmentManager for EnvManager {
    async fn create_environment(&self, config: &BlastConfig) -> BlastResult<PythonEnvironment> {
        let env_path = self.root_path.join(&config.name);

        // Create environment directory
        tokio::fs::create_dir_all(&env_path).await.map_err(|e| {
            DaemonError::environment(format!("Failed to create environment directory: {}", e))
        })?;

        // Initialize Python environment
        let env = PythonEnvironment::new(
            config.name.clone(),
            env_path.clone(),
            config.python_version.clone(),
        ).await?;

        // Create standard directories
        for dir in ["bin", "lib", "include"] {
            tokio::fs::create_dir_all(CoreEnvironment::path(&env).join(dir)).await.map_err(|e| {
                DaemonError::environment(format!("Failed to create {} directory: {}", dir, e))
            })?;
        }

        // Create site-packages directory
        tokio::fs::create_dir_all(CoreEnvironment::path(&env).join("lib").join("python3").join("site-packages"))
            .await
            .map_err(|e| {
                DaemonError::environment(format!("Failed to create site-packages directory: {}", e))
            })?;

        // Install activation scripts
        self.install_activation_scripts(&env_path, &config.name).await?;

        Ok(env)
    }

    async fn update_environment(&self, _env: &PythonEnvironment) -> BlastResult<()> {
        // Not implemented yet
        Ok(())
    }

    async fn activate_environment(&self, env: &PythonEnvironment) -> BlastResult<()> {
        // Just verify the environment exists for now
        let env_path = self.root_path.join(env.name());
        if !env_path.exists() {
            return Err(DaemonError::environment(format!("Environment {} does not exist", env.name())).into());
        }
        Ok(())
    }

    async fn deactivate_environment(&self, env: &PythonEnvironment) -> BlastResult<()> {
        // Just verify the environment exists for now
        let env_path = self.root_path.join(env.name());
        if !env_path.exists() {
            return Err(DaemonError::environment(format!("Environment {} does not exist", env.name())).into());
        }
        Ok(())
    }
}

impl EnvManager {
    async fn install_activation_scripts(&self, env_path: &PathBuf, env_name: &str) -> BlastResult<()> {
        let scripts = ActivationScripts::generate(env_path, env_name);
        let bin_path = env_path.join("bin");

        // Create scripts directory if it doesn't exist
        tokio::fs::create_dir_all(&bin_path).await.map_err(|e| {
            DaemonError::environment(format!("Failed to create bin directory: {}", e))
        })?;

        // Write activation scripts
        let script_files = [
            ("activate", scripts.bash),
            ("activate.fish", scripts.fish),
            ("activate.ps1", scripts.powershell),
        ];

        for (filename, content) in script_files.iter() {
            let script_path = bin_path.join(filename);
            tokio::fs::write(&script_path, content).await.map_err(|e| {
                DaemonError::environment(format!("Failed to write {} script: {}", filename, e))
            })?;

            // Set executable permissions on Unix systems
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = tokio::fs::metadata(&script_path).await?.permissions();
                perms.set_mode(0o755);
                tokio::fs::set_permissions(&script_path, perms).await?;
            }
        }

        Ok(())
    }
} 