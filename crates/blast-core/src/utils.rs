use std::path::PathBuf;
use std::fs;

use crate::error::{BlastError, BlastResult};

/// Create a directory and all its parent directories
pub fn create_dir_all(path: impl AsRef<std::path::Path>) -> BlastResult<()> {
    fs::create_dir_all(path).map_err(BlastError::from)
}

/// Remove a directory and all its contents
pub fn remove_dir_all(path: impl AsRef<std::path::Path>) -> BlastResult<()> {
    fs::remove_dir_all(path).map_err(BlastError::from)
}

/// Copy a directory recursively
pub fn copy_dir_all(src: impl AsRef<std::path::Path>, dst: impl AsRef<std::path::Path>) -> BlastResult<()> {
    let src = src.as_ref();
    let dst = dst.as_ref();

    if !src.is_dir() {
        return Err(BlastError::InvalidPath(src.to_path_buf()));
    }

    if !dst.exists() {
        create_dir_all(dst)?;
    }

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if ty.is_dir() {
            copy_dir_all(src_path, dst_path)?;
        } else {
            fs::copy(src_path, dst_path)?;
        }
    }

    Ok(())
}

/// Create a hardlink if possible, otherwise copy
pub fn hardlink_or_copy(src: impl AsRef<std::path::Path>, dst: impl AsRef<std::path::Path>) -> BlastResult<()> {
    match fs::hard_link(src.as_ref(), dst.as_ref()) {
        Ok(()) => Ok(()),
        Err(_) => fs::copy(src.as_ref(), dst.as_ref()).map(|_| ()).map_err(BlastError::from),
    }
}

/// Get the size of a directory recursively
pub fn dir_size(path: impl AsRef<std::path::Path>) -> BlastResult<u64> {
    let mut total_size = 0;
    let path = path.as_ref();

    if !path.is_dir() {
        return Err(BlastError::InvalidPath(path.to_path_buf()));
    }

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_file() {
            total_size += entry.metadata()?.len();
        } else if ty.is_dir() {
            total_size += dir_size(entry.path())?;
        }
    }

    Ok(total_size)
}

/// Find Python interpreters in the system
pub fn find_python_interpreters() -> BlastResult<Vec<(PathBuf, String)>> {
    let mut interpreters = Vec::new();

    // Common paths to look for Python
    let paths = if cfg!(windows) {
        vec![
            r"C:\Python*",
            r"C:\Program Files\Python*",
            r"C:\Program Files (x86)\Python*",
        ]
    } else {
        vec![
            "/usr/bin/python*",
            "/usr/local/bin/python*",
            "/opt/python*/bin/python*",
        ]
    };

    for glob_pattern in paths {
        for entry in glob::glob(glob_pattern)? {
            if let Ok(path) = entry {
                if let Ok(version) = get_python_version(&path) {
                    interpreters.push((path, version));
                }
            }
        }
    }

    Ok(interpreters)
}

/// Get Python version from interpreter path
fn get_python_version(path: impl AsRef<std::path::Path>) -> BlastResult<String> {
    use std::process::Command;

    let output = Command::new(path.as_ref())
        .arg("--version")
        .output()
        .map_err(|e| BlastError::python(format!("Failed to execute Python: {}", e)))?;

    if !output.status.success() {
        return Err(BlastError::python("Failed to get Python version"));
    }

    let version = String::from_utf8_lossy(&output.stdout);
    Ok(version.trim().to_string())
} 