use std::path::PathBuf;
use blast_core::error::BlastResult;
use blast_core::python::PythonVersion;

pub struct Environment {
    root: PathBuf,
    name: String,
    python_version: PythonVersion,
}

impl Environment {
    pub fn new(root: PathBuf, name: String, python_version: PythonVersion) -> Self {
        Self {
            root,
            name,
            python_version,
        }
    }

    pub fn create(&self) -> BlastResult<()> {
        // Create main directories
        self.create_directories()?;
        
        // Create activation scripts
        self.create_activation_scripts()?;
        
        // Create pyvenv.cfg equivalent
        self.create_config_file()?;
        
        // Create bin directory with necessary executables/scripts
        self.create_bin_directory()?;

        Ok(())
    }

    fn create_directories(&self) -> BlastResult<()> {
        let dirs = [
            "bin",                    // Executables and scripts
            "lib",                    // Python packages and libs
            "include",               // Header files
            "state",                 // Environment state
            "logs",                  // Log files
            "cache",                 // Cache files
            "hooks",                 // Pre/post activation hooks
        ];

        for dir in dirs.iter() {
            std::fs::create_dir_all(self.root.join(dir))?;
        }

        Ok(())
    }

    fn create_activation_scripts(&self) -> BlastResult<()> {
        // Create activation scripts for different shells
        let scripts = [
            ("activate", self.generate_bash_activation()),
            ("activate.fish", self.generate_fish_activation()),
            ("activate.ps1", self.generate_powershell_activation()),
        ];

        // Create the hooks directories
        let hook_dirs = [
            "hooks/pre-activate",
            "hooks/post-activate",
        ];

        for dir in hook_dirs.iter() {
            std::fs::create_dir_all(self.root.join(dir))?;
        }

        for (name, content) in scripts.iter() {
            let path = self.root.join("bin").join(name);
            std::fs::write(&path, content)?;
            
            // Make the scripts executable on Unix
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = std::fs::metadata(&path)?.permissions();
                perms.set_mode(0o755);
                std::fs::set_permissions(&path, perms)?;
            }
        }

        Ok(())
    }

    fn create_config_file(&self) -> BlastResult<()> {
        let config_content = format!(
            "home = {}\n\
             implementation = CPython\n\
             version = {}\n\
             blast_version = 0.1.0\n",
            self.root.display(),
            self.python_version
        );

        std::fs::write(
            self.root.join("blast.cfg"),
            config_content
        )?;

        Ok(())
    }

    fn create_bin_directory(&self) -> BlastResult<()> {
        let bin_dir = self.root.join("bin");
        
        // Create Python symlink/copy
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            let python_path = format!("/usr/local/bin/python{}", self.python_version.to_string());
            symlink(python_path, bin_dir.join("python"))?;
        }

        // Create pip symlink/copy and other tools
        // TODO: Implement proper pip installation
        
        Ok(())
    }

    fn generate_bash_activation(&self) -> String {
        format!(
            r#"#!/bin/bash
# Blast environment activation for bash/zsh

# Load blast configuration
BLAST_HOME="{}"
BLAST_ENV_NAME="{}"
BLAST_PYTHON_VERSION="{}"
BLAST_BIN="$BLAST_HOME/bin"
BLAST_LIB="$BLAST_HOME/lib"
BLAST_INCLUDE="$BLAST_HOME/include"
BLAST_CACHE="$BLAST_HOME/cache"
BLAST_LOGS="$BLAST_HOME/logs"
BLAST_STATE="$BLAST_HOME/state"
BLAST_HOOKS="$BLAST_HOME/hooks"

deactivate () {{
    # Force clean environment through exec
    if [ -f "/tmp/blast/daemon.pid" ]; then
        kill $(cat "/tmp/blast/daemon.pid") 2>/dev/null || true
        rm -f "/tmp/blast/daemon.pid"
    fi
    exec env -i HOME=$HOME PATH=/usr/bin:/bin:/usr/sbin:/sbin:/usr/local/bin:$HOME/.cargo/bin TERM=$TERM zsh -l
}}

# Execute any pre-activation hooks
if [ -d "$BLAST_HOOKS/pre-activate" ]; then
    for hook in "$BLAST_HOOKS/pre-activate"/*; do
        if [ -x "$hook" ]; then
            source "$hook"
        fi
    done
fi

# Save old environment variables
_OLD_VIRTUAL_PATH="$PATH"
_OLD_VIRTUAL_PS1="${{PS1:-}}"
if [ -n "$PYTHONHOME" ] ; then
    _OLD_VIRTUAL_PYTHONHOME="$PYTHONHOME"
    unset PYTHONHOME
fi

# Update environment variables
export BLAST_HOME
export BLAST_ENV_NAME
export BLAST_PYTHON_VERSION
export BLAST_BIN
export BLAST_LIB
export BLAST_INCLUDE
export BLAST_CACHE
export BLAST_LOGS
export BLAST_STATE
export BLAST_HOOKS

# Update PATH
if [[ ":$PATH:" != *":$BLAST_BIN:"* ]]; then
    export PATH="$BLAST_BIN:$PATH"
fi

# Update prompt
PS1="(blast:$BLAST_ENV_NAME) $PS1"
export PS1

# Execute any post-activation hooks
if [ -d "$BLAST_HOOKS/post-activate" ]; then
    for hook in "$BLAST_HOOKS/post-activate"/*; do
        if [ -x "$hook" ]; then
            source "$hook"
        fi
    done
fi

# This should detect bash and zsh, which have a hash command that must
# be called to get it to forget past commands.  Without forgetting
# past commands the $PATH changes we made may not be respected
if [ -n "$BASH" -o -n "$ZSH_VERSION" ] ; then
    hash -r 2>/dev/null
fi
"#,
            self.root.display(),
            self.name,
            self.python_version.to_string(),
        )
    }

    fn generate_fish_activation(&self) -> String {
        // TODO: Implement fish shell activation script
        String::new()
    }

    fn generate_powershell_activation(&self) -> String {
        // TODO: Implement PowerShell activation script
        String::new()
    }
} 