# Blast - Intelligent Python Environment Manager

Blast is a powerful Python environment manager that provides seamless environment activation and robust dependency management, powered by a persistent daemon for enhanced performance.

## Quick Start

1. Add blast to your shell configuration:

```bash
# Add to your ~/.bashrc, ~/.zshrc, or equivalent:
blast() {
    if [ "$1" = "start" ]; then
        eval "$(blast-cli start "${@:2}")"
    else
        blast-cli "$@"
    fi
}
```

2. Create and activate a new environment:

```bash
blast start my-project
```

Your prompt will change to `(blast:my-project)`, indicating that you're now working in the blast environment.

3. To deactivate the environment:

```bash
blast deactivate
```

## Features

- **Seamless Environment Activation**: Just like Python's venv, blast provides a smooth activation experience with `(blast)` prompt indication
- **Persistent Daemon**: Background process handles environment management and caching for improved performance
- **Robust State Management**: Automatic state persistence and recovery
- **Multi-Shell Support**: Works with bash, zsh, fish, and PowerShell
- **Security First**: Isolated environments with configurable security policies

## Commands

- `blast start [name]`: Create and activate a new environment
- `blast deactivate`: Deactivate the current environment
- `blast list`: List available environments
- `blast check`: Check environment status
- `blast clean`: Clean environment cache
- `blast save`: Save environment state
- `blast load`: Load saved environment

## How It Works

Blast uses a similar approach to Python's venv for environment activation:

1. When you run `blast start`, the command:
   - Creates the environment if it doesn't exist
   - Starts a background daemon for environment management
   - Outputs shell commands that modify your current shell's environment
   - Updates your prompt to show the active environment

2. The daemon continues running in the background, handling:
   - Package installations and updates
   - Environment state management
   - Resource monitoring
   - Cache management

3. All environment changes are transactional and can be rolled back if needed.

4. When you run `blast deactivate`, it:
   - Restores your original shell environment
   - Cleans up temporary files
   - Signals the daemon to stop monitoring (if no other environments are active)

## Architecture

Blast uses a client-server architecture:

1. **blast-cli**: Command-line interface for user interaction
2. **blast-daemon**: Background service for environment management
3. **blast-core**: Core functionality and state management
4. **blast-sync**: Synchronization primitives
5. **blast-cache**: Caching layer
6. **blast-resolver**: Dependency resolution
7. **blast-image**: Environment image management

## Development

To build from source:

```bash
cargo build --release
```

## License

[Insert your license information here] 