use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use std::sync::Arc;
use crate::error::BlastResult;

/// Network policy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkPolicy {
    /// Allow outbound connections
    pub allow_outbound: bool,
    /// Allow inbound connections
    pub allow_inbound: bool,
    /// Allowed outbound ports
    pub allowed_outbound_ports: Vec<u16>,
    /// Allowed inbound ports
    pub allowed_inbound_ports: Vec<u16>,
    /// Allowed domains
    pub allowed_domains: Vec<String>,
    /// Allowed IPs
    pub allowed_ips: Vec<String>,
    /// DNS servers
    pub dns_servers: Vec<String>,
    /// Network bandwidth limit (bytes/sec)
    pub bandwidth_limit: Option<u64>,
    /// Network interface configuration
    pub interface_config: NetworkInterfaceConfig,
}

/// Network interface configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInterfaceConfig {
    /// Interface name
    pub name: String,
    /// IP address
    pub ip_address: Option<String>,
    /// Network mask
    pub netmask: Option<String>,
    /// Gateway
    pub gateway: Option<String>,
    /// MTU
    pub mtu: Option<u32>,
}

/// Network state tracking
#[derive(Debug)]
pub struct NetworkState {
    /// Active connections
    connections: Arc<RwLock<HashMap<String, ConnectionInfo>>>,
    /// Bandwidth usage
    bandwidth_usage: Arc<RwLock<BandwidthUsage>>,
    /// Network policy
    policy: NetworkPolicy,
}

/// Connection information
#[derive(Debug)]
pub struct ConnectionInfo {
    /// Source address
    pub source: String,
    /// Destination address
    pub destination: String,
    /// Protocol
    pub protocol: Protocol,
    /// Connection state
    pub state: ConnectionState,
    /// Bytes sent
    pub bytes_sent: u64,
    /// Bytes received
    pub bytes_received: u64,
    /// Created timestamp
    pub created_at: std::time::SystemTime,
    /// Last activity timestamp
    pub last_activity: std::time::SystemTime,
}

impl Clone for ConnectionInfo {
    fn clone(&self) -> Self {
        Self {
            source: self.source.clone(),
            destination: self.destination.clone(),
            protocol: self.protocol,
            state: self.state,
            bytes_sent: self.bytes_sent,
            bytes_received: self.bytes_received,
            created_at: self.created_at,
            last_activity: self.last_activity,
        }
    }
}

impl Default for ConnectionInfo {
    fn default() -> Self {
        let now = std::time::SystemTime::now();
        Self {
            source: String::new(),
            destination: String::new(),
            protocol: Protocol::TCP,
            state: ConnectionState::New,
            bytes_sent: 0,
            bytes_received: 0,
            created_at: now,
            last_activity: now,
        }
    }
}

/// Network protocol
#[derive(Debug, Clone, Copy)]
pub enum Protocol {
    TCP,
    UDP,
    ICMP,
}

/// Connection state
#[derive(Debug, Clone, Copy)]
pub enum ConnectionState {
    New,
    Established,
    Closing,
    Closed,
}

/// Bandwidth usage tracking
#[derive(Debug, Clone)]
pub struct BandwidthUsage {
    /// Total bytes sent
    pub bytes_sent: u64,
    /// Total bytes received
    pub bytes_received: u64,
    /// Current upload rate (bytes/sec)
    pub upload_rate: f64,
    /// Current download rate (bytes/sec)
    pub download_rate: f64,
    /// Last update timestamp
    pub last_update: std::time::SystemTime,
}

impl Default for BandwidthUsage {
    fn default() -> Self {
        Self {
            bytes_sent: 0,
            bytes_received: 0,
            upload_rate: 0.0,
            download_rate: 0.0,
            last_update: std::time::SystemTime::now(),
        }
    }
}

impl NetworkState {
    /// Create new network state
    pub fn new(policy: NetworkPolicy) -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            bandwidth_usage: Arc::new(RwLock::new(BandwidthUsage::default())),
            policy,
        }
    }

    /// Track new connection
    pub async fn track_connection(&self, info: ConnectionInfo) -> BlastResult<()> {
        let mut connections = self.connections.write().await;
        let key = format!("{}:{}", info.source, info.destination);
        connections.insert(key, info);
        Ok(())
    }

    /// Update bandwidth usage
    pub async fn update_bandwidth(&self, bytes_sent: u64, bytes_received: u64) -> BlastResult<()> {
        let mut usage = self.bandwidth_usage.write().await;
        let now = std::time::SystemTime::now();
        let elapsed = now.duration_since(usage.last_update).unwrap_or_default();
        
        // Update totals
        usage.bytes_sent += bytes_sent;
        usage.bytes_received += bytes_received;
        
        // Calculate rates
        if elapsed.as_secs_f64() > 0.0 {
            usage.upload_rate = bytes_sent as f64 / elapsed.as_secs_f64();
            usage.download_rate = bytes_received as f64 / elapsed.as_secs_f64();
        }
        
        usage.last_update = now;
        Ok(())
    }

    /// Check if connection is allowed
    pub fn is_connection_allowed(&self, _source: &str, dest: &str, protocol: Protocol) -> bool {
        // Check basic allow/deny
        match protocol {
            Protocol::TCP | Protocol::UDP => {
                if !self.policy.allow_outbound {
                    return false;
                }
            }
            Protocol::ICMP => return false, // Block ICMP by default
        }
        
        // Check IP/domain allowlist
        if !self.policy.allowed_ips.iter().any(|ip| dest.starts_with(ip)) &&
           !self.policy.allowed_domains.iter().any(|domain| dest.ends_with(domain)) {
            return false;
        }
        
        // Check port allowlist
        if let Some(port) = dest.split(':').nth(1) {
            if let Ok(port_num) = port.parse::<u16>() {
                return self.policy.allowed_outbound_ports.contains(&port_num);
            }
        }
        
        true
    }

    /// Get current bandwidth usage
    pub async fn get_bandwidth_usage(&self) -> BlastResult<BandwidthUsage> {
        let guard = self.bandwidth_usage.read().await;
        Ok(BandwidthUsage {
            bytes_sent: guard.bytes_sent,
            bytes_received: guard.bytes_received,
            upload_rate: guard.upload_rate,
            download_rate: guard.download_rate,
            last_update: guard.last_update,
        })
    }

    /// Get active connections
    pub async fn get_active_connections(&self) -> BlastResult<Vec<ConnectionInfo>> {
        let connections = self.connections.read().await;
        Ok(connections.values().map(|conn| conn.clone()).collect())
    }
}

impl Default for NetworkPolicy {
    fn default() -> Self {
        Self {
            allow_outbound: false,
            allow_inbound: false,
            allowed_outbound_ports: vec![443, 80], // HTTPS and HTTP only
            allowed_inbound_ports: vec![],
            allowed_domains: vec![
                "pypi.org".to_string(),
                "files.pythonhosted.org".to_string(),
            ],
            allowed_ips: vec![],
            dns_servers: vec!["1.1.1.1".to_string()],
            bandwidth_limit: Some(1024 * 1024 * 10), // 10 MB/s
            interface_config: NetworkInterfaceConfig {
                name: "blast0".to_string(),
                ip_address: None,
                netmask: None,
                gateway: None,
                mtu: Some(1500),
            },
        }
    }
} 