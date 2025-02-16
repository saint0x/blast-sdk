"""
Blast - A high-performance Python environment manager and venv replacement
"""

__version__ = "0.1.0"

from .environment import Environment, create_environment
from .package import Package, install_package, uninstall_package

__all__ = [
    "Environment",
    "create_environment",
    "Package",
    "install_package",
    "uninstall_package",
] 