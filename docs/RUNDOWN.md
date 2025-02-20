# Blast Technical Rundown

## Overview

Blast is a high-performance Python environment manager written in Rust that reimagines the traditional venv workflow. It implements a two-layer synchronization architecture where the environment state layer manages containerization and isolation, while the package management layer handles real-time dependency synchronization. This allows users to simply "blast start" their project and have all environment and dependency management handled automatically in the background.

## Core Configuration

### blast.toml - Required Configuration

```toml
# Core Environment Configuration (Required)
[env]
python_version = "3.11"  # Python version for the environment
name = "project_name"    # Project name
version = "1.0.0"       # Project version

# Container Configuration (Required)
[container]
isolation_level = "process"  # Process, namespace, or container
resource_limits = true       # Enable resource limits
network_policy = "isolated"  # Network isolation policy

# Sandboxing Configuration (Required)
[sandbox]
network_isolation = true    # Enable network isolation
resource_control = true     # Enable resource control
filesystem_isolation = true # Enable filesystem isolation

# Security Configuration (Required)
[security]
default_policy = "strict"   # Security policy level
audit_logging = true        # Enable security audit logging

# Sync Configuration (Required)
[sync]
env_layer = "strict"      # Environment layer sync policy
pkg_layer = "automatic"   # Package layer sync policy
conflict_resolution = "interactive"  # How to handle conflicts
```

### Optional Configuration

```toml
# Layer-specific Configuration
[env_layer]
snapshot_interval = 300    # State snapshot interval in seconds
max_snapshots = 10        # Maximum number of state snapshots
recovery_mode = "latest"  # Recovery strategy

[pkg_layer]
auto_sync = true          # Enable automatic package syncing
sync_interval = 60        # Package check interval in seconds
version_policy = "latest" # Version selection policy

# Resolver Configuration
[resolver]
max_concurrent_requests = 10  # Maximum concurrent PyPI requests
request_timeout = 30         # Request timeout in seconds
verify_ssl = true           # Whether to verify SSL certificates
allow_prereleases = false   # Whether to allow pre-release versions

# Cache Configuration
[cache]
ttl = 86400                # Cache TTL in seconds
cache_dir = ".blast/cache" # Custom cache directory

# Update Behavior
[updates]
auto_update = true         # Enable automatic updates
force_updates = false      # Force update packages
update_dependencies = true # Update dependencies with packages

# Monitor Configuration
[monitor]
enabled = true            # Enable file monitoring
watch_paths = ["src"]     # Paths to monitor for changes

# Network Isolation Configuration
[sandbox.network]
allow_outbound = false
allow_inbound = false
allowed_outbound_ports = [443, 80]
allowed_domains = ["pypi.org", "files.pythonhosted.org"]
bandwidth_limit = 10485760  # 10MB/s

# Resource Control Configuration
[sandbox.resources]
cpu_limit = 100.0          # CPU usage percentage
memory_limit = 2147483648  # 2GB memory limit
disk_limit = 10737418240   # 10GB disk limit
iops_limit = 1000          # IO operations per second
process_limit = 50         # Maximum processes

# Filesystem Security Configuration
[sandbox.filesystem]
readonly_paths = ["/usr", "/lib", "/bin"]
hidden_paths = ["/proc", "/sys", "/dev"]
allowed_paths = ["/tmp", "/var/tmp"]
denied_paths = ["/etc/shadow", "/etc/passwd", "/root"]
max_file_size = 104857600  # 100MB max file size
```

## Implemented Features

### 1. Package Management
- Dependency resolution using PubGrub algorithm
- Version constraint handling and validation
- Package metadata caching
- Update checking and notification
- PyPI integration with concurrent requests

### 2. Environment Management
- Environment creation and configuration
- Python version management
- Basic file system monitoring
- Update service for package management

### 3. Sandboxing and Security
- Network Isolation
  - Connection tracking and monitoring
  - Domain and IP allowlisting
  - Port access control
  - Bandwidth throttling
  - Network namespace isolation

- Resource Control
  - CPU usage limits and scheduling
  - Memory allocation controls
  - I/O bandwidth throttling
  - Process/thread limits
  - CGroup-based resource management

- Filesystem Security
  - Path-based access control
  - Read-only enforcement
  - Mount point isolation
  - Size restrictions
  - Access tracking

- Security Policy Enforcement
  - Default deny policies
  - Least privilege access
  - Resource quotas
  - Audit logging
  - Secure recovery procedures

### 4. Background Services
- Daemon service for environment management
- IPC server for command handling
- Update queue for package operations
- Asynchronous operation handling

## Command Set (Implemented)

```bash
# Environment Management
blast start            # Create and activate environment
blast kill             # Terminate environment

# Package Management
blast install <pkg>    # Install packages
blast update [pkg]     # Update packages
blast remove <pkg>     # Remove packages

# Information
blast status          # Show environment status
blast list           # List installed packages
```

## Technical Architecture

### 1. Core Components

```rust
// Two-Layer Sync Architecture
pub struct SyncManager {
    env_layer: EnvironmentLayer,
    pkg_layer: PackageLayer,
    coordinator: SyncCoordinator,
}

// Environment Layer
pub struct EnvironmentLayer {
    container: Container,
    state: EnvironmentState,
    resources: ResourceManager,
}

// Package Layer
pub struct PackageLayer {
    resolver: DependencyResolver,
    installer: PackageInstaller,
    version_manager: VersionManager,
}

// Sync Coordinator
pub struct SyncCoordinator {
    env_state: Arc<RwLock<EnvironmentState>>,
    pkg_state: Arc<RwLock<PackageState>>,
    metrics: Arc<MetricsManager>,
}
```

### 2. Service Architecture

```rust
// Daemon service with layer awareness
pub struct Daemon {
    config: DaemonConfig,
    sync_manager: Arc<SyncManager>,
    update_queue: mpsc::Sender<UpdateRequest>,
    shutdown: broadcast::Sender<()>,
}

// Environment monitor with layer separation
pub struct EnvironmentMonitor {
    env_layer: Arc<EnvironmentLayer>,
    pkg_layer: Arc<PackageLayer>,
    update_queue: mpsc::Sender<UpdateRequest>,
}

// Package monitor
pub struct PackageMonitor {
    resolver: Arc<DependencyResolver>,
    installer: Arc<PackageInstaller>,
    update_queue: mpsc::Sender<UpdateRequest>,
}
```

## Error Handling

```rust
pub enum BlastError {
    Io(std::io::Error),
    Python(String),
    Package(String),
    Environment(String),
    Cache(String),
    Resolution(String),
    Config(String),
    Network(String),
    Daemon(String),
}
```

## Performance Characteristics

### Current Performance
- Package resolution: ~100ms per package
- Environment creation: ~500ms
- Package installation: Varies by package size
- Cache operations: <10ms
- Update checks: ~50ms per package

### Resource Usage
- Idle memory: ~20MB
- Active memory: 50-200MB
- Cache size: Configurable, defaults to system cache
- Network: Concurrent requests limited to 10

## Future Development

The following features are planned but not yet implemented:

1. Sandboxing and Isolation
2. Image Management
3. Resource Controls
4. Network Isolation
5. Parallel Processing Optimizations
6. Memory Pooling
7. Team Collaboration Features
8. CI/CD Integration

These features will be implemented based on user feedback and priority. 