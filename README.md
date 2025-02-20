# Blast: Modern Python Environment Manager

Blast is a high-performance Python environment manager written in Rust that reimagines the traditional venv workflow. It implements a two-layer synchronization architecture where the environment state layer manages containerization and isolation, while the package management layer handles real-time dependency synchronization.

## Features

### Core Features
- **Two-Layer Architecture**
  - Environment State Layer: Manages containerization and isolation
  - Package Management Layer: Handles real-time package management
- **Real-time Dependency Management**
  - Live pip operation interception
  - Automatic dependency resolution
  - Version conflict detection and resolution
- **State Management**
  - Transaction-based state updates
  - State history tracking
  - Rollback capabilities
- **Security**
  - Network isolation
  - Resource limits
  - Filesystem security
  - Process isolation

### Package Layer
- Real-time pip operation interception
- Live dependency graph updates
- Enhanced version conflict resolution
- Robust package state persistence
- Transaction-based updates
- State history tracking

### Integration Layer
- Layer coordination
- Automatic conflict resolution
- Transaction management
- Error recovery system
- Metrics collection
- State synchronization

### Shell Integration
- Multi-shell support (bash, zsh, fish)
- Clean ANSI handling
- Proper prompt management
- Environment variable tracking
- Cross-platform compatibility

## Implementation Status

### Completed
- Basic package state management
- Initial pip operation interception
- Simple dependency graph updates
- Basic conflict checking
- State persistence (save/load)
- Shell integration basics
- Environment isolation foundation

### In Progress
- Real-time pip operation handling
- Live dependency graph updates
- Enhanced conflict resolution
- Transaction-based state management
- Layer coordination improvements
- Automatic conflict resolution
- Error recovery system

### Planned
- State history tracking
- Advanced rollback capabilities
- More sophisticated resolution strategies
- Enhanced error recovery mechanisms
- Improved layer coordination
- Better transaction management
- PowerShell support
- Windows compatibility improvements

## Usage

```bash
# Create and activate a new environment
blast start [options]

# Deactivate and clean up environment
blast kill

# View environment status
blast status

# List installed packages
blast list

# Install packages
blast install <package>

# Update packages
blast update [package]

# Remove packages
blast remove <package>
```

## Architecture

### Core Components
- **blast-core**: Core functionality and types
- **blast-cli**: Command-line interface
- **blast-daemon**: Background service

### Key Features
1. **Environment Management**
   - Robust activation/deactivation
   - State persistence
   - Clean ANSI handling

2. **Package Management**
   - Real-time operation handling
   - Dependency resolution
   - Conflict management

3. **Security**
   - Network isolation
   - Resource limits
   - Filesystem security

## Development Status

Blast is currently in active development. The core functionality is implemented and working, with ongoing work on advanced features and improvements.

### Current Focus
1. Real-time pip operation handling
2. Live dependency graph updates
3. Enhanced conflict resolution
4. Transaction-based state management
5. Layer coordination improvements
6. Automatic conflict resolution
7. Error recovery capabilities

## Contributing

Contributions are welcome! Please read our contributing guidelines and code of conduct before submitting pull requests.

## License

MIT License - see LICENSE for details 