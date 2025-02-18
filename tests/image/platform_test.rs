use blast_image::platform::{
    PlatformRequirements,
    PlatformInfo,
    GpuRequirements,
    GpuDevice
};

#[test]
fn test_platform_requirements() {
    let requirements = PlatformRequirements::default();
    assert!(requirements.os.contains(&"linux".to_string()));
    assert!(requirements.arch.contains(&"x86_64".to_string()));
    assert_eq!(requirements.min_cores, 1);
    assert!(requirements.min_memory >= 1024 * 1024 * 1024); // At least 1GB
}

#[test]
fn test_platform_info() {
    let info = PlatformInfo::current();
    assert!(!info.os.is_empty());
    assert!(!info.arch.is_empty());
    
    let requirements = PlatformRequirements {
        os: vec![info.os.clone()],
        arch: vec![info.arch.clone()],
        min_memory: info.min_memory,
        min_disk_space: info.min_disk_space,
        ..Default::default()
    };

    assert!(info.meets_requirements(&requirements));
}

#[test]
fn test_incompatible_requirements() {
    let info = PlatformInfo::current();
    
    let requirements = PlatformRequirements {
        os: vec!["invalid_os".to_string()],
        ..Default::default()
    };

    assert!(!info.meets_requirements(&requirements));
}

#[test]
fn test_gpu_requirements() {
    let gpu_reqs = GpuRequirements {
        min_memory: 4 * 1024 * 1024 * 1024, // 4GB
        cuda_version: Some("11.0".to_string()),
        rocm_version: None,
        required_features: vec!["tensor_cores".to_string()],
    };

    let requirements = PlatformRequirements {
        gpu_requirements: Some(gpu_reqs),
        ..Default::default()
    };

    let info = PlatformInfo::current();
    // This test might fail on systems without GPU
    // We're just testing the requirement structure
    assert!(!info.meets_requirements(&requirements));
}

#[test]
fn test_system_dependencies() {
    let info = PlatformInfo::current();
    
    // Check common system dependencies
    let common_deps = vec![
        "libc".to_string(),
        "libstdc++".to_string(),
        "libssl".to_string(),
    ];

    for dep in common_deps {
        assert!(info.system_deps.contains(&dep));
    }
}

#[test]
fn test_gpu_device_info() {
    let devices = PlatformInfo::get_gpu_devices();
    
    for device in devices {
        // Basic validation of GPU device info
        assert!(!device.name.is_empty());
        assert!(device.memory > 0);
        
        // If CUDA is available, validate version format
        if let Some(cuda_ver) = device.cuda_capability {
            assert!(cuda_ver.contains('.'));
        }
        
        // If ROCm is available, validate version format
        if let Some(rocm_ver) = device.rocm_version {
            assert!(rocm_ver.contains('.'));
        }
    }
}

#[test]
fn test_platform_requirements_validation() {
    let requirements = PlatformRequirements {
        os: vec!["linux".to_string(), "darwin".to_string()],
        arch: vec!["x86_64".to_string(), "aarch64".to_string()],
        min_cores: 2,
        min_memory: 2 * 1024 * 1024 * 1024, // 2GB
        min_disk_space: 10 * 1024 * 1024 * 1024, // 10GB
        required_features: vec!["sse4.2".to_string(), "avx2".to_string()],
        gpu_requirements: None,
    };

    let info = PlatformInfo::current();
    let meets_reqs = info.meets_requirements(&requirements);
    
    // The test result will depend on the system, but we can check the logic
    assert_eq!(
        meets_reqs,
        requirements.os.contains(&info.os) &&
        requirements.arch.contains(&info.arch) &&
        info.min_memory >= requirements.min_memory &&
        info.min_disk_space >= requirements.min_disk_space
    );
}
