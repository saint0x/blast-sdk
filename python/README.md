# Blast Environment Manager

A high-performance Python environment manager and venv replacement.

## Installation

```bash
pip install blast-env
```

## Usage

### Creating a New Environment

```python
from blast import create_environment

# Create a new environment with the current Python version
env = create_environment("path/to/env")

# Create an environment with a specific Python version
env = create_environment("path/to/env", python_version="3.9.6")
```

### Managing Packages

```python
from blast import Environment, Package

# Create and use an environment
env = Environment("path/to/env")
env.create()

# Install a package
package = Package("requests", "2.28.2")
env.install_package(package)

# List installed packages
packages = env.get_packages()
for package in packages:
    print(f"{package.name} {package.version}")

# Uninstall a package
env.uninstall_package(package)
```

### Helper Functions

```python
from blast import install_package, uninstall_package

# Install a package into an environment
install_package("path/to/env", "requests", "2.28.2")

# Uninstall a package from an environment
uninstall_package("path/to/env", "requests")
```

## Features

- Pure Python implementation - no external dependencies
- Uses Python's built-in venv module
- Type hints for better IDE support
- Simple, intuitive API
- Cross-platform support (Windows, macOS, Linux)
- Python 3.7+ compatibility

## Development

To run tests:

```bash
cd python
python -m pytest tests/
```

## License

MIT License 