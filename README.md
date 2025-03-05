# Blast - Intelligent Python Environment Manager

Blast is a powerful Python environment manager that seamlessly orchestrates dependencies and state, inside an isolated sanboxed enviroment.

## Quick Start

1. Install blast:

```bash
cargo install --path .
```

2. The shell integration will be automatically installed in your shell's rc file (`.zshrc`, `.bashrc`, etc.)

3. Create and activate a new environment:

```bash
blast start
```

Your prompt will change to `(blast)`, indicating that you're now working in the blast environment.

## Core Features

### 🔒 Environment Isolation
- **Process Management**: Managed Python process isolation
- **Resource Monitoring**: CPU and memory usage tracking
- **Health Checks**: Continuous environment health monitoring
- **State Persistence**: Reliable environment state management

### 📦 Package Management
- **Dependency Tracking**: Package state monitoring
- **Environment Validation**: Integrity checks for Python environments
- **State Management**: Environment state persistence
- **Shell Integration**: Seamless shell activation and deactivation

### 🔄 Daemon Architecture
- **Background Service**: Persistent daemon for environment management
- **Health Monitoring**: Real-time health status checks
- **Resource Limits**: Basic resource usage monitoring
- **State Synchronization**: Environment state coordination

### 🛠 Developer Experience
- **Multi-shell Support**: Works with `bash`, `zsh`, `fish`, and `powershell`
- **Clean Integration**: Proper prompt and environment handling
- **Status Monitoring**: Environment health checks
- **Automatic Setup**: Self-installing shell integration

## Implementation Status

### ✅ Completed
- Daemon-based environment management
- Shell integration and activation
- Environment state persistence
- Health monitoring system
- Resource usage tracking
- Multi-shell support
- Process isolation basics

### 🚧 In Progress
- Package management integration
- Enhanced resource limits
- Advanced isolation features
- Custom environment configurations
- Network isolation
- Container runtime support

## Architecture

```rust
// Core Architecture
pub struct Daemon {
    state_manager: StateManager,     // State persistence
    health_manager: HealthManager,   // Health monitoring
    resource_monitor: ResourceMonitor, // Resource tracking
    metrics_manager: MetricsManager,  // Usage metrics
}
```

## Development

To build from source:

```bash
# Clone repository
git clone https://github.com/saint0x/blast-sdk.git
cd blast-rs

# Build release version
cargo build --release

# Install locally
cargo install --path .
```

## Contributing

Contributions are welcome! Please feel free to submit pull requests.

## License

MIT License - see [LICENSE](LICENSE) for details. 