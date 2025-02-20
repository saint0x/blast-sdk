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

2. **Two-Layer Synchronization**
   - Environment State Layer: Manages containerized environment state
   - Package Management Layer: Handles package version synchronization
   - Real-time sync between layers
   - Automatic conflict resolution
   - Transparent to end users

3. **Reliability First**
   - Robust state management with transactions
   - Automatic recovery from failures
   - Proper cleanup on deactivation
   - Secure by default

4. **Simplicity Over Complexity**
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
- Layer synchronization state tracking

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
    Sync { force: bool },  // New command for manual sync
}
```

#### Command Flow

1. **Start Command**
   ```
   blast start [options]
   ```