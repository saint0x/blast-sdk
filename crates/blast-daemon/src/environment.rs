use std::path::PathBuf;
use tokio::fs;
use blast_core::{
    error::BlastResult,
    python::{PythonVersion, PythonEnvironment},
    ActivationScripts,
};
use crate::error::DaemonError;

/// Environment management trait
pub trait Environment {
    /// Get environment path
    fn path(&self) -> &PathBuf;

    /// Get Python version
    fn python_version(&self) -> &PythonVersion;

    /// Start monitoring the environment
    fn start_monitoring(&self) -> impl std::future::Future<Output = BlastResult<()>> + Send;
}

pub trait PythonEnvironmentExt {
    fn create(&self) -> impl std::future::Future<Output = BlastResult<()>> + Send;
}

impl PythonEnvironmentExt for PythonEnvironment {
    fn create(&self) -> impl std::future::Future<Output = BlastResult<()>> + Send {
        async move {
            // Create the virtual environment directory
            tokio::fs::create_dir_all(self.path()).await?;

            // Create bin directory
            let bin_dir = self.path().join("bin");
            tokio::fs::create_dir_all(&bin_dir).await?;

            // Create lib directory
            let lib_dir = self.path().join("lib");
            tokio::fs::create_dir_all(&lib_dir).await?;

            // Create include directory
            let include_dir = self.path().join("include");
            tokio::fs::create_dir_all(&include_dir).await?;

            // Create site-packages directory
            let site_packages = lib_dir.join("python3").join("site-packages");
            tokio::fs::create_dir_all(&site_packages).await?;

            // Create pyvenv.cfg
            let python_path = bin_dir.join("python");
            let pyvenv_cfg = format!(
                "home = {}\nversion = {}\ninclude-system-site-packages = false\n",
                python_path.display(),
                self.python_version()
            );
            tokio::fs::write(self.path().join("pyvenv.cfg"), pyvenv_cfg).await?;

            Ok(())
        }
    }
}

#[derive(Debug)]
pub struct EnvironmentManager {
    root_path: PathBuf,
}

impl EnvironmentManager {
    pub fn new(root_path: PathBuf) -> Self {
        Self { root_path }
    }

    pub fn root_path(&self) -> &PathBuf {
        &self.root_path
    }

    /// Install activation scripts for all supported shells
    async fn install_activation_scripts(&self, env_path: &PathBuf, env_name: &str) -> BlastResult<()> {
        let scripts = ActivationScripts::generate(env_path, env_name);
        let bin_path = env_path.join("bin");

        // Create scripts directory if it doesn't exist
        fs::create_dir_all(&bin_path).await.map_err(|e| {
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
            fs::write(&script_path, content).await.map_err(|e| {
                DaemonError::environment(format!("Failed to write {} script: {}", filename, e))
            })?;

            // Set executable permissions on Unix systems
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = fs::metadata(&script_path).await?.permissions();
                perms.set_mode(0o755);
                fs::set_permissions(&script_path, perms).await?;
            }
        }

        Ok(())
    }

    /// Create a new Python environment
    pub async fn create_environment(
        &self,
        name: &str,
        python_version: &PythonVersion,
    ) -> BlastResult<PythonEnvironment> {
        let env_path = self.root_path.join(name);

        // Create environment directory
        fs::create_dir_all(&env_path).await.map_err(|e| {
            DaemonError::environment(format!("Failed to create environment directory: {}", e))
        })?;

        // Initialize Python environment
        let env = PythonEnvironment::new(env_path.clone(), python_version.clone());
        PythonEnvironmentExt::create(&env).await?;

        // Create standard directories
        for dir in ["bin", "lib", "include"] {
            fs::create_dir_all(env.path().join(dir)).await.map_err(|e| {
                DaemonError::environment(format!("Failed to create {} directory: {}", dir, e))
            })?;
        }

        // Create site-packages directory
        fs::create_dir_all(env.path().join("lib").join("python3").join("site-packages"))
            .await
            .map_err(|e| {
                DaemonError::environment(format!("Failed to create site-packages directory: {}", e))
            })?;

        // Install activation scripts
        self.install_activation_scripts(&env_path, name).await?;

        Ok(env)
    }
} 