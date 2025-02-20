use std::collections::HashSet;
use serde::{Deserialize, Serialize};
use crate::error::BlastResult;

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Linux capabilities configuration
    pub capabilities: CapabilitiesConfig,
    /// Seccomp filter configuration
    pub seccomp: SeccompConfig,
    /// AppArmor profile configuration
    pub apparmor: AppArmorConfig,
    /// SELinux configuration
    pub selinux: SELinuxConfig,
    /// Privileged mode
    pub privileged: bool,
    /// No new privileges flag
    pub no_new_privileges: bool,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            capabilities: CapabilitiesConfig::default(),
            seccomp: SeccompConfig::default(),
            apparmor: AppArmorConfig::default(),
            selinux: SELinuxConfig::default(),
            privileged: false,
            no_new_privileges: true,
        }
    }
}

/// Linux capabilities configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilitiesConfig {
    /// Allowed capabilities
    pub allowed: HashSet<Capability>,
    /// Dropped capabilities
    pub dropped: HashSet<Capability>,
    /// Ambient capabilities
    pub ambient: HashSet<Capability>,
}

impl Default for CapabilitiesConfig {
    fn default() -> Self {
        let mut dropped = HashSet::new();
        dropped.insert(Capability::SysAdmin);
        dropped.insert(Capability::NetAdmin);
        dropped.insert(Capability::DacOverride);
        
        Self {
            allowed: HashSet::new(),
            dropped,
            ambient: HashSet::new(),
        }
    }
}

/// Linux capability
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum Capability {
    /// CAP_AUDIT_CONTROL
    AuditControl,
    /// CAP_AUDIT_READ
    AuditRead,
    /// CAP_AUDIT_WRITE
    AuditWrite,
    /// CAP_BLOCK_SUSPEND
    BlockSuspend,
    /// CAP_BPF
    Bpf,
    /// CAP_CHECKPOINT_RESTORE
    CheckpointRestore,
    /// CAP_CHOWN
    Chown,
    /// CAP_DAC_OVERRIDE
    DacOverride,
    /// CAP_DAC_READ_SEARCH
    DacReadSearch,
    /// CAP_FOWNER
    Fowner,
    /// CAP_FSETID
    Fsetid,
    /// CAP_IPC_LOCK
    IpcLock,
    /// CAP_IPC_OWNER
    IpcOwner,
    /// CAP_KILL
    Kill,
    /// CAP_LEASE
    Lease,
    /// CAP_LINUX_IMMUTABLE
    LinuxImmutable,
    /// CAP_MAC_ADMIN
    MacAdmin,
    /// CAP_MAC_OVERRIDE
    MacOverride,
    /// CAP_MKNOD
    Mknod,
    /// CAP_NET_ADMIN
    NetAdmin,
    /// CAP_NET_BIND_SERVICE
    NetBindService,
    /// CAP_NET_BROADCAST
    NetBroadcast,
    /// CAP_NET_RAW
    NetRaw,
    /// CAP_PERFMON
    Perfmon,
    /// CAP_SETFCAP
    Setfcap,
    /// CAP_SETGID
    Setgid,
    /// CAP_SETPCAP
    Setpcap,
    /// CAP_SETUID
    Setuid,
    /// CAP_SYS_ADMIN
    SysAdmin,
    /// CAP_SYS_BOOT
    SysBoot,
    /// CAP_SYS_CHROOT
    SysChroot,
    /// CAP_SYS_MODULE
    SysModule,
    /// CAP_SYS_NICE
    SysNice,
    /// CAP_SYS_PACCT
    SysPacct,
    /// CAP_SYS_PTRACE
    SysPtrace,
    /// CAP_SYS_RAWIO
    SysRawio,
    /// CAP_SYS_RESOURCE
    SysResource,
    /// CAP_SYS_TIME
    SysTime,
    /// CAP_SYS_TTY_CONFIG
    SysTtyConfig,
    /// CAP_SYSLOG
    Syslog,
    /// CAP_WAKE_ALARM
    WakeAlarm,
}

/// Seccomp filter configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeccompConfig {
    /// Default action
    pub default_action: SeccompAction,
    /// Syscall filters
    pub syscalls: Vec<SyscallFilter>,
    /// Architecture filters
    pub architectures: Vec<SeccompArch>,
}

impl Default for SeccompConfig {
    fn default() -> Self {
        Self {
            default_action: SeccompAction::Allow,
            syscalls: Vec::new(),
            architectures: vec![SeccompArch::X86_64],
        }
    }
}

/// Seccomp action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SeccompAction {
    /// Allow syscall
    Allow,
    /// Kill process
    Kill,
    /// Return error
    Errno(i32),
    /// Trap
    Trap,
    /// Log
    Log,
    /// Trace
    Trace,
}

/// Syscall filter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyscallFilter {
    /// Syscall names
    pub names: Vec<String>,
    /// Action
    pub action: SeccompAction,
    /// Arguments
    pub args: Vec<SyscallArg>,
}

/// Syscall argument filter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyscallArg {
    /// Index
    pub index: usize,
    /// Operation
    pub op: ArgOperation,
    /// Value
    pub value: u64,
    /// Value mask
    pub value_mask: u64,
}

/// Argument operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ArgOperation {
    /// Not equal
    NotEqual,
    /// Less than
    LessThan,
    /// Less than or equal
    LessThanEqual,
    /// Equal
    Equal,
    /// Greater than or equal
    GreaterThanEqual,
    /// Greater than
    GreaterThan,
    /// Masked equal
    MaskedEqual,
}

/// Seccomp architecture
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SeccompArch {
    /// x86
    X86,
    /// x86_64
    X86_64,
    /// x32
    X32,
    /// ARM
    ARM,
    /// ARM64
    ARM64,
    /// MIPS
    MIPS,
    /// MIPS64
    MIPS64,
    /// MIPSEL
    MIPSEL,
    /// MIPSEL64
    MIPSEL64,
    /// PPC
    PPC,
    /// PPC64
    PPC64,
    /// PPC64LE
    PPC64LE,
    /// S390X
    S390X,
}

/// AppArmor profile configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppArmorConfig {
    /// Profile name
    pub profile_name: Option<String>,
    /// Custom profile
    pub custom_profile: Option<String>,
}

impl Default for AppArmorConfig {
    fn default() -> Self {
        Self {
            profile_name: Some("blast-default".to_string()),
            custom_profile: None,
        }
    }
}

/// SELinux configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SELinuxConfig {
    /// SELinux type
    pub selinux_type: Option<String>,
    /// SELinux level
    pub level: Option<String>,
    /// SELinux user
    pub user: Option<String>,
    /// SELinux role
    pub role: Option<String>,
}

impl Default for SELinuxConfig {
    fn default() -> Self {
        Self {
            selinux_type: Some("blast_t".to_string()),
            level: None,
            user: None,
            role: None,
        }
    }
}

/// Security manager implementation
pub struct SecurityManager {
    /// Security configuration
    #[allow(dead_code)]  // Used in async methods
    config: SecurityConfig,
}

impl SecurityManager {
    /// Create new security manager
    pub fn new(config: SecurityConfig) -> Self {
        Self { config }
    }

    /// Apply security configuration
    pub async fn apply_config(&self) -> BlastResult<()> {
        self.apply_capabilities().await?;
        self.apply_seccomp().await?;
        self.apply_apparmor().await?;
        self.apply_selinux().await?;
        self.apply_privileges().await?;
        Ok(())
    }

    /// Apply capabilities configuration
    async fn apply_capabilities(&self) -> BlastResult<()> {
        #[cfg(target_os = "linux")]
        {
            use caps::{CapSet, Capability as LinuxCapability};
            
            // Drop capabilities
            for cap in &self.config.capabilities.dropped {
                if let Some(linux_cap) = self.to_linux_capability(cap) {
                    caps::drop(None, CapSet::Effective, linux_cap)?;
                    caps::drop(None, CapSet::Permitted, linux_cap)?;
                    caps::drop(None, CapSet::Inheritable, linux_cap)?;
                }
            }
            
            // Set allowed capabilities
            for cap in &self.config.capabilities.allowed {
                if let Some(linux_cap) = self.to_linux_capability(cap) {
                    caps::raise(None, CapSet::Effective, linux_cap)?;
                    caps::raise(None, CapSet::Permitted, linux_cap)?;
                }
            }
            
            // Set ambient capabilities
            for cap in &self.config.capabilities.ambient {
                if let Some(linux_cap) = self.to_linux_capability(cap) {
                    caps::raise(None, CapSet::Ambient, linux_cap)?;
                }
            }
        }
        
        Ok(())
    }

    /// Apply seccomp configuration
    async fn apply_seccomp(&self) -> BlastResult<()> {
        #[cfg(target_os = "linux")]
        {
            use seccomp::{
                SeccompFilter as Filter,
                SeccompAction as Action,
                SeccompRule as Rule,
                SeccompCmp as Cmp,
            };
            
            let mut filter = Filter::new(
                self.to_seccomp_action(&self.config.seccomp.default_action)
            )?;
            
            // Add syscall filters
            for syscall in &self.config.seccomp.syscalls {
                let mut rules = Vec::new();
                
                for arg in &syscall.args {
                    rules.push(Rule::new(
                        arg.index as u32,
                        self.to_seccomp_cmp(&arg.op),
                        arg.value,
                        arg.value_mask,
                    )?);
                }
                
                for name in &syscall.names {
                    filter.add_rule(
                        name,
                        self.to_seccomp_action(&syscall.action),
                        &rules,
                    )?;
                }
            }
            
            // Load filter
            filter.load()?;
        }
        
        Ok(())
    }

    /// Apply AppArmor configuration
    async fn apply_apparmor(&self) -> BlastResult<()> {
        #[cfg(target_os = "linux")]
        {
            if let Some(profile) = &self.config.apparmor.profile_name {
                // Load AppArmor profile
                std::fs::write("/proc/self/attr/current", profile)?;
            }
            
            if let Some(custom_profile) = &self.config.apparmor.custom_profile {
                // Load custom AppArmor profile
                let profile_path = format!("/etc/apparmor.d/blast.{}", custom_profile);
                std::fs::write(&profile_path, custom_profile)?;
                
                std::process::Command::new("apparmor_parser")
                    .arg("-r")
                    .arg(&profile_path)
                    .output()?;
            }
        }
        
        Ok(())
    }

    /// Apply SELinux configuration
    async fn apply_selinux(&self) -> BlastResult<()> {
        #[cfg(target_os = "linux")]
        {
            if let Some(selinux_type) = &self.config.selinux.selinux_type {
                // Set SELinux type
                std::fs::write("/proc/self/attr/current", selinux_type)?;
            }
            
            if let Some(level) = &self.config.selinux.level {
                // Set SELinux level
                std::fs::write("/proc/self/attr/mls", level)?;
            }
        }
        
        Ok(())
    }

    /// Apply privileges configuration
    async fn apply_privileges(&self) -> BlastResult<()> {
        #[cfg(target_os = "linux")]
        {
            if !self.config.privileged {
                // Drop all capabilities if not privileged
                use caps::{CapSet, Capability};
                for cap in Capability::iter() {
                    caps::drop(None, CapSet::Effective, cap)?;
                    caps::drop(None, CapSet::Permitted, cap)?;
                    caps::drop(None, CapSet::Inheritable, cap)?;
                }
            }
            
            if self.config.no_new_privileges {
                // Set no new privileges flag
                prctl::set_no_new_privileges(true)?;
            }
        }
        
        Ok(())
    }

    /// Convert capability to Linux capability
    #[cfg(target_os = "linux")]
    fn to_linux_capability(&self, cap: &Capability) -> Option<caps::Capability> {
        match cap {
            Capability::AuditControl => Some(caps::Capability::CAP_AUDIT_CONTROL),
            Capability::AuditRead => Some(caps::Capability::CAP_AUDIT_READ),
            Capability::AuditWrite => Some(caps::Capability::CAP_AUDIT_WRITE),
            Capability::BlockSuspend => Some(caps::Capability::CAP_BLOCK_SUSPEND),
            Capability::Bpf => Some(caps::Capability::CAP_BPF),
            Capability::CheckpointRestore => Some(caps::Capability::CAP_CHECKPOINT_RESTORE),
            Capability::Chown => Some(caps::Capability::CAP_CHOWN),
            Capability::DacOverride => Some(caps::Capability::CAP_DAC_OVERRIDE),
            Capability::DacReadSearch => Some(caps::Capability::CAP_DAC_READ_SEARCH),
            Capability::Fowner => Some(caps::Capability::CAP_FOWNER),
            Capability::Fsetid => Some(caps::Capability::CAP_FSETID),
            Capability::IpcLock => Some(caps::Capability::CAP_IPC_LOCK),
            Capability::IpcOwner => Some(caps::Capability::CAP_IPC_OWNER),
            Capability::Kill => Some(caps::Capability::CAP_KILL),
            Capability::Lease => Some(caps::Capability::CAP_LEASE),
            Capability::LinuxImmutable => Some(caps::Capability::CAP_LINUX_IMMUTABLE),
            Capability::MacAdmin => Some(caps::Capability::CAP_MAC_ADMIN),
            Capability::MacOverride => Some(caps::Capability::CAP_MAC_OVERRIDE),
            Capability::Mknod => Some(caps::Capability::CAP_MKNOD),
            Capability::NetAdmin => Some(caps::Capability::CAP_NET_ADMIN),
            Capability::NetBindService => Some(caps::Capability::CAP_NET_BIND_SERVICE),
            Capability::NetBroadcast => Some(caps::Capability::CAP_NET_BROADCAST),
            Capability::NetRaw => Some(caps::Capability::CAP_NET_RAW),
            Capability::Perfmon => Some(caps::Capability::CAP_PERFMON),
            Capability::Setfcap => Some(caps::Capability::CAP_SETFCAP),
            Capability::Setgid => Some(caps::Capability::CAP_SETGID),
            Capability::Setpcap => Some(caps::Capability::CAP_SETPCAP),
            Capability::Setuid => Some(caps::Capability::CAP_SETUID),
            Capability::SysAdmin => Some(caps::Capability::CAP_SYS_ADMIN),
            Capability::SysBoot => Some(caps::Capability::CAP_SYS_BOOT),
            Capability::SysChroot => Some(caps::Capability::CAP_SYS_CHROOT),
            Capability::SysModule => Some(caps::Capability::CAP_SYS_MODULE),
            Capability::SysNice => Some(caps::Capability::CAP_SYS_NICE),
            Capability::SysPacct => Some(caps::Capability::CAP_SYS_PACCT),
            Capability::SysPtrace => Some(caps::Capability::CAP_SYS_PTRACE),
            Capability::SysRawio => Some(caps::Capability::CAP_SYS_RAWIO),
            Capability::SysResource => Some(caps::Capability::CAP_SYS_RESOURCE),
            Capability::SysTime => Some(caps::Capability::CAP_SYS_TIME),
            Capability::SysTtyConfig => Some(caps::Capability::CAP_SYS_TTY_CONFIG),
            Capability::Syslog => Some(caps::Capability::CAP_SYSLOG),
            Capability::WakeAlarm => Some(caps::Capability::CAP_WAKE_ALARM),
        }
    }

    /// Convert seccomp action to seccomp-rs action
    #[cfg(target_os = "linux")]
    fn to_seccomp_action(&self, action: &SeccompAction) -> seccomp::SeccompAction {
        match action {
            SeccompAction::Allow => seccomp::SeccompAction::Allow,
            SeccompAction::Kill => seccomp::SeccompAction::Kill,
            SeccompAction::Errno(errno) => seccomp::SeccompAction::Errno(*errno),
            SeccompAction::Trap => seccomp::SeccompAction::Trap,
            SeccompAction::Log => seccomp::SeccompAction::Log,
            SeccompAction::Trace => seccomp::SeccompAction::Trace(0),
        }
    }

    /// Convert argument operation to seccomp-rs comparison
    #[cfg(target_os = "linux")]
    fn to_seccomp_cmp(&self, op: &ArgOperation) -> seccomp::SeccompCmp {
        match op {
            ArgOperation::NotEqual => seccomp::SeccompCmp::NotEqual,
            ArgOperation::LessThan => seccomp::SeccompCmp::Less,
            ArgOperation::LessThanEqual => seccomp::SeccompCmp::LessOrEqual,
            ArgOperation::Equal => seccomp::SeccompCmp::Equal,
            ArgOperation::GreaterThanEqual => seccomp::SeccompCmp::GreaterOrEqual,
            ArgOperation::GreaterThan => seccomp::SeccompCmp::Greater,
            ArgOperation::MaskedEqual => seccomp::SeccompCmp::MaskedEqual,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_security_config() {
        let config = SecurityConfig::default();
        assert!(!config.privileged);
        assert!(config.no_new_privileges);
        assert!(config.capabilities.dropped.contains(&Capability::SysAdmin));
    }

    #[tokio::test]
    async fn test_security_manager() {
        let config = SecurityConfig::default();
        let manager = SecurityManager::new(config);
        
        // Test applying configuration
        manager.apply_config().await.unwrap();
    }
} 