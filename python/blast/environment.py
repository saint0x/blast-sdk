"""
Environment management functionality
"""

import os
import subprocess
import sys
import venv
from pathlib import Path
from typing import List, Optional

from .package import Package


class Environment:
    """A Python virtual environment managed by Blast"""

    def __init__(self, path: str, python_version: str = None):
        self.path = Path(path).absolute()
        self.python_version = python_version or f"{sys.version_info.major}.{sys.version_info.minor}.{sys.version_info.micro}"
        self._name: Optional[str] = None

    @property
    def python_executable(self) -> Path:
        """Get the Python executable path for this environment"""
        if sys.platform == "win32":
            return self.path / "Scripts" / "python.exe"
        return self.path / "bin" / "python"

    @property
    def pip_executable(self) -> Path:
        """Get the pip executable path for this environment"""
        if sys.platform == "win32":
            return self.path / "Scripts" / "pip.exe"
        return self.path / "bin" / "pip"

    def create(self) -> None:
        """Create the virtual environment"""
        builder = venv.EnvBuilder(
            system_site_packages=False,
            clear=True,
            with_pip=True,
            upgrade_deps=True
        )
        builder.create(str(self.path))

    def install_package(self, package: Package) -> None:
        """Install a package into the environment"""
        cmd = [
            str(self.pip_executable),
            "install",
            f"{package.name}=={package.version}"
        ]
        result = subprocess.run(cmd, capture_output=True, text=True)
        if result.returncode != 0:
            raise RuntimeError(f"Failed to install package {package.name}: {result.stderr}")

    def uninstall_package(self, package: Package) -> None:
        """Uninstall a package from the environment"""
        cmd = [
            str(self.pip_executable),
            "uninstall",
            "--yes",
            package.name
        ]
        result = subprocess.run(cmd, capture_output=True, text=True)
        if result.returncode != 0:
            raise RuntimeError(f"Failed to uninstall package {package.name}: {result.stderr}")

    def get_packages(self) -> List[Package]:
        """Get all installed packages"""
        cmd = [str(self.pip_executable), "freeze"]
        result = subprocess.run(cmd, capture_output=True, text=True)
        if result.returncode != 0:
            raise RuntimeError(f"Failed to get installed packages: {result.stderr}")

        packages = []
        for line in result.stdout.splitlines():
            if "==" in line:
                name, version = line.split("==", 1)
                packages.append(Package(name.strip(), version.strip()))
        return packages

    @property
    def name(self) -> Optional[str]:
        """Get the environment name"""
        return self._name

    @name.setter
    def name(self, value: str) -> None:
        """Set the environment name"""
        self._name = value


def create_environment(path: str, python_version: str = None) -> Environment:
    """Create a new Python environment at the specified path"""
    env = Environment(path, python_version)
    env.create()
    return env 