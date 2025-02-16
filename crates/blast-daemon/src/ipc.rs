//! Inter-Process Communication for the daemon service

use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tracing::{error, info};

use crate::update::UpdateRequest;
use crate::DaemonResult;
use crate::update::UpdateType;

/// IPC server for handling client requests
pub struct IPCServer {
    receiver: Arc<Mutex<mpsc::Receiver<UpdateRequest>>>,
}

impl IPCServer {
    /// Create a new IPC server
    pub fn new(receiver: mpsc::Receiver<UpdateRequest>) -> Self {
        Self {
            receiver: Arc::new(Mutex::new(receiver)),
        }
    }

    /// Run the IPC server
    pub async fn run(&self) -> DaemonResult<()> {
        info!("Starting IPC server");

        loop {
            let mut receiver = self.receiver.lock().await;
            match receiver.recv().await {
                Some(request) => {
                    match &request.update_type {
                        UpdateType::PackageUpdate { package, .. } => {
                            info!("Processing update request for {}", package.name());
                        }
                        UpdateType::PackageInstall(package) => {
                            info!("Processing install request for {}", package.name());
                        }
                        UpdateType::PackageRemove(package) => {
                            info!("Processing remove request for {}", package.name());
                        }
                        UpdateType::EnvironmentSync => {
                            info!("Processing environment sync request");
                        }
                    }
                }
                None => {
                    error!("IPC channel closed");
                    break;
                }
            }
        }

        info!("IPC server stopped");
        Ok(())
    }
} 