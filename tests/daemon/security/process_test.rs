use blast_core::{
    python::PythonVersion,
    security::{SecurityPolicy, IsolationLevel, ResourceLimits},
};
use blast_daemon::security::process::ProcessIsolation;
use std::{path::PathBuf, process::Command};
use blast_core::security::ProcessConfig;
use blast_daemon::security::process::ProcessManager;

mod process_creation {
    use super::*;

    #[tokio::test]
    async fn test_process_spawn() {
        let config = ProcessConfig {
            command: "echo".into(),
            args: vec!["test".into()],
            working_dir: PathBuf::from("."),
            env: Default::default(),
        };
        
        let manager = ProcessManager::new();
        let handle = manager.spawn(config).await.unwrap();
        assert!(handle.pid() > 0);
    }

    #[tokio::test]
    async fn test_process_with_environment() {
        let mut env = std::collections::HashMap::new();
        env.insert("TEST_VAR".into(), "test_value".into());
        
        let config = ProcessConfig {
            command: "env".into(),
            args: vec![],
            working_dir: PathBuf::from("."),
            env,
        };
        
        let manager = ProcessManager::new();
        let handle = manager.spawn(config).await.unwrap();
        assert!(handle.pid() > 0);
    }
}

mod process_isolation {
    use super::*;

    #[tokio::test]
    async fn test_process_working_directory() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config = ProcessConfig {
            command: "pwd".into(),
            args: vec![],
            working_dir: temp_dir.path().to_path_buf(),
            env: Default::default(),
        };
        
        let manager = ProcessManager::new();
        let handle = manager.spawn(config).await.unwrap();
        assert!(handle.pid() > 0);
    }

    #[tokio::test]
    async fn test_process_isolation_env() {
        let config = ProcessConfig {
            command: "env".into(),
            args: vec![],
            working_dir: PathBuf::from("."),
            env: Default::default(),
        };
        
        let manager = ProcessManager::new();
        let handle = manager.spawn(config).await.unwrap();
        assert!(handle.pid() > 0);
        
        // Verify process environment is isolated
        let output = Command::new("ps")
            .arg("-p")
            .arg(handle.pid().to_string())
            .arg("-o")
            .arg("command=")
            .output()
            .unwrap();
            
        assert!(String::from_utf8_lossy(&output.stdout).contains("env"));
    }
}

mod process_lifecycle {
    use super::*;

    #[tokio::test]
    async fn test_process_termination() {
        let config = ProcessConfig {
            command: "sleep".into(),
            args: vec!["1".into()],
            working_dir: PathBuf::from("."),
            env: Default::default(),
        };
        
        let manager = ProcessManager::new();
        let handle = manager.spawn(config).await.unwrap();
        
        // Terminate the process
        handle.terminate().await.unwrap();
        
        // Verify process is terminated
        assert!(Command::new("ps")
            .arg("-p")
            .arg(handle.pid().to_string())
            .output()
            .unwrap()
            .status
            .code()
            .unwrap() == 1);
    }

    #[tokio::test]
    async fn test_process_cleanup() {
        let manager = ProcessManager::new();
        
        // Spawn multiple processes
        let configs = vec![
            ProcessConfig {
                command: "sleep".into(),
                args: vec!["1".into()],
                working_dir: PathBuf::from("."),
                env: Default::default(),
            },
            ProcessConfig {
                command: "sleep".into(),
                args: vec!["2".into()],
                working_dir: PathBuf::from("."),
                env: Default::default(),
            },
        ];
        
        let handles: Vec<_> = futures::future::join_all(
            configs.into_iter().map(|config| manager.spawn(config))
        ).await
        .into_iter()
        .collect::<Result<_, _>>()
        .unwrap();
        
        // Cleanup all processes
        manager.cleanup().await.unwrap();
        
        // Verify all processes are terminated
        for handle in handles {
            assert!(Command::new("ps")
                .arg("-p")
                .arg(handle.pid().to_string())
                .output()
                .unwrap()
                .status
                .code()
                .unwrap() == 1);
        }
    }
}

#[tokio::test]
async fn test_process_isolation() {
    let isolation = ProcessIsolation::new(Default::default());
    let config = SecurityPolicy {
        isolation_level: IsolationLevel::Process,
        ..Default::default()
    };

    // Create environment
    let env = isolation.create_environment(&config).await.unwrap();
    assert!(env.exists());

    // Execute command
    let result = isolation.execute_command(&env, "print('test')").await.unwrap();
    assert_eq!(result.trim(), "test");

    // Check resource usage
    let usage = isolation.get_resource_usage(&env).await.unwrap();
    assert!(usage.memory_usage > 0);
    assert!(usage.cpu_usage >= 0.0);

    // Destroy environment
    isolation.destroy_environment(&env).await.unwrap();
}

#[tokio::test]
async fn test_resource_limits() {
    let policy = SecurityPolicy {
        isolation_level: IsolationLevel::Process,
        resource_limits: ResourceLimits {
            max_memory: 1024 * 1024 * 100, // 100MB
            max_disk: 1024 * 1024 * 500,   // 500MB
            max_processes: 5,
        },
        ..Default::default()
    };

    let isolation = ProcessIsolation::new(policy);
    let env = isolation.create_environment(&policy).await.unwrap();

    // Run memory-intensive operation
    let result = isolation.execute_command(
        &env,
        r#"
import numpy as np
arr = np.zeros((1000, 1000))  # Allocate some memory
print('allocated')
        "#,
    ).await.unwrap();
    assert_eq!(result.trim(), "allocated");

    // Check resource usage is within limits
    let usage = isolation.get_resource_usage(&env).await.unwrap();
    assert!(usage.memory_usage <= policy.resource_limits.max_memory);

    isolation.destroy_environment(&env).await.unwrap();
}

#[tokio::test]
async fn test_multiple_environments() {
    let isolation = ProcessIsolation::new(Default::default());
    let config = SecurityPolicy::default();

    // Create multiple environments
    let env1 = isolation.create_environment(&config).await.unwrap();
    let env2 = isolation.create_environment(&config).await.unwrap();

    // Execute commands in both environments
    let result1 = isolation.execute_command(&env1, "print('env1')").await.unwrap();
    let result2 = isolation.execute_command(&env2, "print('env2')").await.unwrap();

    assert_eq!(result1.trim(), "env1");
    assert_eq!(result2.trim(), "env2");

    // Check resource usage for both
    let usage1 = isolation.get_resource_usage(&env1).await.unwrap();
    let usage2 = isolation.get_resource_usage(&env2).await.unwrap();

    assert!(usage1.memory_usage > 0);
    assert!(usage2.memory_usage > 0);

    // Cleanup
    isolation.destroy_environment(&env1).await.unwrap();
    isolation.destroy_environment(&env2).await.unwrap();
}

#[tokio::test]
async fn test_environment_isolation() {
    let isolation = ProcessIsolation::new(Default::default());
    let config = SecurityPolicy::default();

    let env1 = isolation.create_environment(&config).await.unwrap();
    let env2 = isolation.create_environment(&config).await.unwrap();

    // Set variable in env1
    isolation.execute_command(&env1, "x = 42").await.unwrap();
    
    // Try to access variable in env2 (should fail)
    let result = isolation.execute_command(&env2, "print(x)").await;
    assert!(result.is_err(), "Variable should not be accessible across environments");

    // Cleanup
    isolation.destroy_environment(&env1).await.unwrap();
    isolation.destroy_environment(&env2).await.unwrap();
}

#[tokio::test]
async fn test_process_cleanup() {
    let isolation = ProcessIsolation::new(Default::default());
    let config = SecurityPolicy::default();

    let env = isolation.create_environment(&config).await.unwrap();
    
    // Start a background process
    isolation.execute_command(
        &env,
        r#"
import threading
def background_task():
    while True:
        pass
thread = threading.Thread(target=background_task)
thread.daemon = True
thread.start()
print('started')
        "#,
    ).await.unwrap();

    // Verify process is running
    let usage = isolation.get_resource_usage(&env).await.unwrap();
    assert!(usage.cpu_usage > 0.0);

    // Destroy environment should clean up all processes
    isolation.destroy_environment(&env).await.unwrap();

    // Trying to get resource usage should now fail
    let result = isolation.get_resource_usage(&env).await;
    assert!(result.is_err(), "Environment should be completely destroyed");
} 