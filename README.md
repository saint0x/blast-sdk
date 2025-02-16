# Blast

A high-performance Python environment manager and venv replacement written in Rust.

## Features

- Fast dependency resolution
- Efficient environment management
- Python version management
- Caching for improved performance

## Installation

```bash
pip install blast
```

## Usage

```python
import blast

# Create a new environment
env = blast.BlastEnvironment("myenv", "3.8")

# Install dependencies
env.install_packages(["numpy", "pandas"])
```

## Development

To build from source:

1. Install Rust and Python development tools
2. Clone the repository
3. Run `maturin develop` to build and install locally

## License

MIT License 