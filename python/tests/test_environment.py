import os
import sys
import tempfile
from pathlib import Path
import pytest

from blast import Environment, Package, create_environment


def test_environment_creation():
    with tempfile.TemporaryDirectory() as tmpdir:
        env = Environment(tmpdir)
        env.create()
        
        # Check that Python executable exists
        assert env.python_executable.exists()
        
        # Check that pip executable exists
        assert env.pip_executable.exists()


def test_environment_python_version():
    with tempfile.TemporaryDirectory() as tmpdir:
        version = f"{sys.version_info.major}.{sys.version_info.minor}.{sys.version_info.micro}"
        env = Environment(tmpdir, python_version=version)
        assert env.python_version == version


def test_package_installation():
    with tempfile.TemporaryDirectory() as tmpdir:
        env = create_environment(tmpdir)
        
        # Install a package
        package = Package("six", "1.16.0")
        env.install_package(package)
        
        # Verify it's in the list of installed packages
        packages = env.get_packages()
        assert any(p.name == "six" for p in packages)


def test_package_uninstallation():
    with tempfile.TemporaryDirectory() as tmpdir:
        env = create_environment(tmpdir)
        
        # Install and then uninstall a package
        package = Package("six", "1.16.0")
        env.install_package(package)
        env.uninstall_package(package)
        
        # Verify it's no longer in the list of installed packages
        packages = env.get_packages()
        assert not any(p.name == "six" for p in packages)


def test_environment_name():
    with tempfile.TemporaryDirectory() as tmpdir:
        env = Environment(tmpdir)
        assert env.name is None
        
        env.name = "test-env"
        assert env.name == "test-env"


def test_create_environment_helper():
    with tempfile.TemporaryDirectory() as tmpdir:
        env = create_environment(tmpdir)
        assert env.python_executable.exists()
        assert env.pip_executable.exists()


def test_package_string_representation():
    package = Package("requests", "2.28.2")
    assert str(package) == "requests==2.28.2" 