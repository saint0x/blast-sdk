# Blast - Context & Design Philosophy

## Overview

Blast is a modern Python environment manager designed to provide a seamless, venv-like experience with enhanced functionality. The project aims to solve common pain points in Python environment management while maintaining simplicity and reliability.

## Core Philosophy

1. **Seamless Developer Experience (DX)**
   - Zero-configuration setup - everything just works
   - Automatic shell integration
   - Familiar workflow similar to Python's built-in venv
   - Clear visual indicators (shell prompt changes)
   - Cross-platform compatibility

2. **Reliability First**
   - Robust state management with transactions
   - Automatic recovery from failures
   - Proper cleanup on deactivation
   - Secure by default

3. **Simplicity Over Complexity**
   - Minimal dependencies
   - Platform-agnostic approach
   - Easy installation and setup
   - Clear, predictable behavior

## Implementation Details

### 1. Architecture Overview

The project is organized into several crates:

1. **blast-core**
   - Core functionality and types
   - Environment management
   - Shell script generation
   - State management
   - Error handling

2. **blast-cli**
   - Command-line interface
   - Shell integration
   - User interaction
   - Command routing

3. **blast-daemon**
   - Background service
   - Environment monitoring
   - Package management
   - State persistence

### 2. Shell Integration

#### Activation Scripts
Shell-specific activation scripts are generated for each environment:

```rust
pub struct ActivationScripts {
    pub bash: String,
    pub fish: String,
    pub powershell: String,
}
```

Each script handles:
- Environment variable management (PATH, PYTHONPATH)
- Shell prompt customization
- Deactivation function
- Socket path management for daemon communication

#### Shell Detection and Configuration
The system automatically:
1. Detects the user's shell type (bash, zsh, fish, powershell)
2. Generates appropriate activation scripts
3. Manages shell state persistence
4. Handles cross-platform compatibility

### 3. Command Structure

The CLI uses a unified command structure:

```rust
pub enum Commands {
    Start {
        python: Option<String>,
        name: Option<String>,
        path: Option<PathBuf>,
        env: Vec<String>,
    },
    Kill { force: bool },
    Clean,
    Save { name: Option<String> },
    Load { name: Option<String> },
    List,
    Check,
}
```

#### Command Flow

1. **Start Command**
   ```
   blast start [options]
   ```
   - Creates environment if needed
   - Starts daemon process
   - Generates activation scripts
   - Updates shell environment
   - Sets up Python paths
   - Initializes state tracking

2. **Kill Command**
   ```
   blast kill [--force]
   ```
   - Graceful shutdown of daemon
   - Cleanup of environment
   - Shell state restoration
   - Process termination

### 4. State Management

#### Environment State
```rust
pub struct EnvironmentState {
    name: String,
    python_version: PythonVersion,
    packages: HashMap<String, Version>,
    version_histories: HashMap<String, VersionHistory>,
}
```

#### Shell State
```rust
pub struct ShellState {
    original_path: Option<String>,
    original_pythonpath: Option<String>,
    original_prompt: Option<String>,
    active_env_name: Option<String>,
    active_env_path: Option<PathBuf>,
    socket_path: Option<String>,
}
```

### 5. Daemon Architecture

The daemon process manages:

1. **Environment Monitoring**
   - Resource usage tracking
   - Package state monitoring
   - File system changes

2. **State Persistence**
   - Transaction-based operations
   - Automatic checkpointing
   - State recovery
   - Rollback capability

3. **Package Management**
   - Dependency resolution
   - Version constraint handling
   - Cache management
   - Update impact analysis

### 6. Security Model

1. **Process Isolation**
   ```rust
   pub struct SecurityPolicy {
       isolation_level: IsolationLevel,
       python_version: PythonVersion,
       resource_limits: ResourceLimits,
   }
   ```

2. **Resource Management**
   - Memory limits
   - Disk usage quotas
   - Network access control
   - Process restrictions

## Usage Workflow

### 1. Environment Creation
```bash
$ blast start my-project
# Internally:
# 1. Creates environment structure
# 2. Starts daemon process
# 3. Generates activation scripts
# 4. Updates shell environment
```

### 2. Environment Activation
```bash
$ blast start my-project
# Shell evaluates:
# 1. Saves current shell state
# 2. Updates PATH and PYTHONPATH
# 3. Sets environment variables
# 4. Updates prompt
```

### 3. Package Management
```bash
$ pip install package-name
# Daemon:
# 1. Intercepts pip operations
# 2. Manages transactions
# 3. Updates state
# 4. Handles rollback if needed
```

### 4. Deactivation
```bash
$ blast kill
# 1. Restores original shell state
# 2. Stops daemon process
# 3. Cleans up resources
# 4. Persists final state
```

## Future Considerations

1. **Enhanced Security**
   - Sandboxing improvements
   - Dependency verification
   - Enhanced resource isolation

2. **Performance Optimizations**
   - Parallel package operations
   - Improved caching
   - Reduced activation overhead

3. **Extended Functionality**
   - Remote environments
   - Team synchronization
   - CI/CD integration
   - Development tool integration

## Conclusion

Blast provides a robust, user-friendly Python environment manager that combines the familiarity of venv with enhanced functionality. Through careful architecture and implementation choices, it maintains simplicity while offering powerful features for Python development workflows. 