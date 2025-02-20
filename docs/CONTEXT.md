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

## Sandboxing Philosophy

### 1. **Defense in Depth**
   - Multiple layers of security controls
   - Each layer provides independent protection
   - Failure of one layer doesn't compromise overall security
   - Comprehensive security coverage

### 2. **Zero Trust Architecture**
   - No implicit trust for any component
   - All access must be explicitly granted
   - Continuous verification of security boundaries
   - Real-time monitoring and enforcement

### 3. **Resource Control**
   - Fine-grained resource allocation
   - Predictable resource usage
   - Protection against resource exhaustion
   - Fair resource sharing

### 4. **Isolation First**
   - Complete process isolation
   - Network access control
   - Filesystem boundaries
   - Resource partitioning

### 5. **Security by Design**
   - Security integrated from the start
   - Not an afterthought or add-on
   - Built into core architecture
   - Influences all design decisions

## Sandboxing Implementation Principles

### 1. Network Security
```rust
pub struct NetworkPolicy {
    // Network access controls
    pub allow_outbound: bool,
    pub allow_inbound: bool,
    pub allowed_outbound_ports: Vec<u16>,
    pub allowed_domains: Vec<String>,
    
    // Resource limits
    pub bandwidth_limit: Option<u64>,
    pub interface_config: NetworkInterfaceConfig,
}
```

- Default-deny network access
- Explicit allowlisting required
- Bandwidth control and monitoring
- Connection tracking

### 2. Resource Management
```rust
pub struct ResourceLimits {
    pub cpu: CpuLimits,
    pub memory: MemoryLimits,
    pub io: IoLimits,
    pub process: ProcessLimits,
}
```

- CPU usage control
- Memory allocation limits
- I/O operation throttling
- Process count restrictions

### 3. Filesystem Security
```rust
pub struct FilesystemPolicy {
    pub root_dir: PathBuf,
    pub readonly_paths: Vec<PathBuf>,
    pub hidden_paths: Vec<PathBuf>,
    pub allowed_paths: Vec<PathBuf>,
    pub denied_paths: Vec<PathBuf>,
}
```

- Path-based access control
- Read-only enforcement
- Hidden path masking
- Size restrictions

### 4. Security State Management
```rust
pub struct ContainerState {
    pub namespaces_created: bool,
    pub cgroups_configured: bool,
    pub network_configured: bool,
    pub filesystem_configured: bool,
    pub initialized: bool,
}
```

- Real-time state tracking
- Security boundary verification
- Resource usage monitoring
- Violation detection

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