"""
Package management functionality
"""

from dataclasses import dataclass
from typing import Optional


@dataclass
class Package:
    """A Python package with version information"""
    name: str
    version: str
    python_version: Optional[str] = None

    def __str__(self) -> str:
        return f"{self.name}=={self.version}"


def install_package(env_path: str, package_name: str, package_version: str) -> None:
    """Install a package into the specified environment"""
    from .environment import Environment
    env = Environment(env_path)
    env.install_package(Package(package_name, package_version))


def uninstall_package(env_path: str, package_name: str) -> None:
    """Uninstall a package from the specified environment"""
    from .environment import Environment
    env = Environment(env_path)
    env.uninstall_package(Package(package_name, "0.0.0"))  # Version doesn't matter for uninstall 