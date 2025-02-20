# Blast - Intelligent Python Environment Manager

Blast is a powerful Python environment manager that provides seamless environment activation and robust dependency management, powered by a persistent daemon for enhanced performance.

## Quick Start

1. Add blast to your shell configuration:

```bash
# Add to your ~/.bashrc, ~/.zshrc, or equivalent:
blast() {
    if [ "$1" = "start" ]; then
        eval "$(command blast start "${@:2}")"
    else
        command blast "$@"
    fi
}
```

2. Create and activate a new environment:

```bash
blast start my-project
```

Your prompt will change to `(blast:my-project)`, indicating that you're now working in the blast environment.

3. Manage your environment:

```bash
blast install requests    # Install packages
blast update numpy       # Update specific package
blast remove pandas      # Remove packages
blast list              # List installed packages
blast status            # Check environment status
blast kill              # Terminate environment
```

## Core Features

### 🔒 Advanced Isolation
- **Network Control**: Fine-grained network access policies
- **Resource Limits**: CPU, memory, and I/O constraints
- **Filesystem Security**: Path-based access control and monitoring
- **Process Isolation**: Complete process and namespace separation

### 📦 Smart Package Management
- **Real-time Dependency Resolution**: Live package operation handling
- **Conflict Prevention**: Proactive dependency conflict detection
- **State Management**: Transaction-based package operations
- **Operation Interception**: Intelligent pip command handling

### 🔄 State Synchronization
- **Atomic Updates**: All changes are transactional
- **State History**: Complete environment state tracking
- **Rollback Support**: Revert to any previous state
- **Error Recovery**: Automatic error detection and recovery

### 🛠 Developer Experience
- **Multi-shell Support**: Works with `bash`, `zsh`, and `fish`
- **Clean Integration**: Proper prompt and environment handling
- **Status Monitoring**: Real-time environment health checks
- **Extensible Design**: Plugin support for custom workflows

## Implementation Status

### ✅ Completed
- Basic package state management
- Initial pip operation interception
- Simple dependency graph updates
- Basic conflict checking
- State persistence (save/load)
- Shell integration basics
- Environment isolation foundation

### 🚧 In Progress
- Real-time pip operation handling
- Live dependency graph updates
- Enhanced conflict resolution
- Transaction-based state management
- Layer coordination improvements
- Automatic conflict resolution
- Error recovery system

## Architecture

```rust
// Two-Layer Architecture
pub struct Environment {
    // Environment State Layer
    container: Container,         // Isolation control
    resources: ResourceManager,   // Resource limits
    security: SecurityManager,    // Security policies
    
    // Package Management Layer
    packages: PackageManager,     // Package operations
    resolver: DependencyResolver, // Dependency handling
    state: StateManager,         // State tracking
}
```

## Development

To build from source:

```bash
# Clone repository
git clone https://github.com/blast-rs/blast
cd blast

# Build release version
cargo build --release

# Run tests
cargo test --all
```

## Contributing

Contributions are welcome! Please read our [contributing guidelines](CONTRIBUTING.md) before submitting pull requests.

## License

MIT License - see [LICENSE](LICENSE) for details. 