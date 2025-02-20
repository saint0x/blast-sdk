# Blast Architecture Diagrams

## System Architecture

```mermaid
graph TB
    User[User/Python Code] --> CLI[blast-cli]
    CLI --> Daemon[blast-daemon]
    Daemon --> Core[blast-core]
    
    subgraph "Environment State Layer"
        Core --> Container[Container Runtime]
        Container --> Network[Network Isolation]
        Container --> Resources[Resource Control]
        Container --> Filesystem[Filesystem Security]
    end
    
    subgraph "Package Management Layer"
        Core --> Resolver[Dependency Resolver]
        Resolver --> Installer[Package Installer]
        Installer --> Cache[Package Cache]
    end
    
    Core --> Coordinator[Sync Coordinator]
    Coordinator --> Monitor[Hot-Reload Monitor]
```

## Sandboxing Layers

```mermaid
graph TB
    subgraph "Network Isolation"
        NP[NetworkPolicy] --> NT[Network Tracking]
        NT --> BW[Bandwidth Control]
        NT --> Conn[Connection Monitor]
        NT --> Port[Port Control]
    end
    
    subgraph "Resource Control"
        RL[ResourceLimits] --> CPU[CPU Limits]
        RL --> Mem[Memory Limits]
        RL --> IO[I/O Control]
        RL --> Proc[Process Limits]
    end
    
    subgraph "Filesystem Security"
        FS[FilesystemPolicy] --> Path[Path Control]
        FS --> Mount[Mount Points]
        FS --> Access[Access Control]
        FS --> Size[Size Limits]
    end
    
    Container[Container Runtime] --> |Manages| NP
    Container --> |Enforces| RL
    Container --> |Controls| FS
```

## State Management

```mermaid
stateDiagram-v2
    [*] --> Initializing
    Initializing --> ConfiguringNamespaces
    ConfiguringNamespaces --> ConfiguringCgroups
    ConfiguringCgroups --> ConfiguringNetwork
    ConfiguringNetwork --> ConfiguringFilesystem
    ConfiguringFilesystem --> Running
    Running --> Cleanup
    Cleanup --> [*]
    
    Running --> Error
    Error --> Cleanup
    
    state Running {
        [*] --> Active
        Active --> Syncing
        Syncing --> Active
        Active --> Snapshotting
        Snapshotting --> Active
    }
```

## Component Interaction

```mermaid
sequenceDiagram
    participant User
    participant CLI
    participant Daemon
    participant Container
    participant Network
    participant Resources
    participant Filesystem
    
    User->>CLI: blast start
    CLI->>Daemon: Initialize Environment
    Daemon->>Container: Create Container
    Container->>Network: Configure Network
    Network-->>Container: Network Ready
    Container->>Resources: Set Resource Limits
    Resources-->>Container: Limits Applied
    Container->>Filesystem: Setup Filesystem
    Filesystem-->>Container: Filesystem Ready
    Container-->>Daemon: Container Ready
    Daemon-->>CLI: Environment Active
    CLI-->>User: Ready for Use
```

## Security Policy Flow

```mermaid
graph LR
    subgraph "Security Policies"
        Default[Default Deny] --> Allow[Explicit Allow]
        Allow --> Monitor[Real-time Monitor]
        Monitor --> Enforce[Policy Enforcement]
        Enforce --> Audit[Audit Logging]
    end
    
    subgraph "Resource Quotas"
        CPU --> Memory
        Memory --> IO[I/O]
        IO --> Network
        Network --> Process
    end
    
    subgraph "Access Control"
        Network_AC[Network] --> FS_AC[Filesystem]
        FS_AC --> Proc_AC[Process]
        Proc_AC --> Res_AC[Resources]
    end
```

## Package Management Flow

```mermaid
graph TB
    subgraph "Package Resolution"
        Import[Import Hook] --> Check[Version Check]
        Check --> Resolve[Dependency Resolution]
        Resolve --> Install[Package Installation]
    end
    
    subgraph "State Sync"
        Install --> Update[State Update]
        Update --> Verify[State Verification]
        Verify --> Commit[Commit Changes]
    end
    
    subgraph "Rollback"
        Error[Error Detection] --> Revert[State Reversion]
        Revert --> Cleanup[Resource Cleanup]
    end
```

## Environment Lifecycle

```mermaid
stateDiagram-v2
    [*] --> Created
    Created --> Initializing: blast start
    Initializing --> Configuring: Setup
    Configuring --> Active: Ready
    Active --> Snapshotted: Save
    Snapshotted --> Active: Load
    Active --> Cleanup: Kill
    Cleanup --> [*]
    
    state Active {
        [*] --> Running
        Running --> Updating: Package Change
        Updating --> Running: Complete
        Running --> Syncing: State Sync
        Syncing --> Running: Complete
    }
```

## Network Isolation Model

```mermaid
graph TB
    subgraph "Network Namespace"
        Interface[Virtual Interface] --> Policy[Network Policy]
        Policy --> Outbound[Outbound Control]
        Policy --> Inbound[Inbound Control]
        Policy --> Domains[Domain Allowlist]
    end
    
    subgraph "Bandwidth Management"
        Monitor[Usage Monitor] --> Throttle[Bandwidth Throttle]
        Throttle --> Limits[Rate Limits]
    end
    
    subgraph "Connection Control"
        Track[Connection Tracking] --> Allow[Allowlist Check]
        Allow --> Rate[Rate Limiting]
        Rate --> Log[Access Logging]
    end
```

## Resource Control Model

```mermaid
graph TD
    A[Blast Core] --> B[Environment Layer]
    A --> C[Package Layer]
    A --> D[Security Layer]
    
    B --> E[Resource Management]
    B --> F[State Management]
    B --> G[Container Runtime]
    
    E --> H[CPU Control]
    E --> I[Memory Control]
    E --> J[I/O Control]
    E --> K[Process Control]
    
    H --> L[Quota/Period]
    H --> M[CPU Shares]
    
    I --> N[Hard Limits]
    I --> O[Soft Limits]
    
    J --> P[I/O Weight]
    J --> Q[Bandwidth Limits]
    
    D --> R[Filesystem Security]
    D --> S[Network Security]
    D --> T[Process Security]
    
    R --> U[Mount Validation]
    R --> V[Access Tracking]
    R --> W[Error Recovery]
    
    U --> X[Path Validation]
    U --> Y[Mount Options]
    U --> Z[Recursion Check]
    
    V --> AA[Pattern Detection]
    V --> AB[Violation Logging]
    V --> AC[Access History]
    
    W --> AD[Atomic Operations]
    W --> AE[State Recovery]
    W --> AF[Rollback]
```

## Filesystem Security Model

```mermaid
graph TB
    subgraph "Mount Management"
        Mount[Mount Controller] --> Points[Mount Points]
        Points --> ReadOnly[Read-Only Mounts]
        Points --> Hidden[Hidden Paths]
    end
    
    subgraph "Access Control"
        Access[Access Controller] --> Paths[Path Control]
        Paths --> Perms[Permissions]
        Perms --> Size[Size Control]
    end
    
    subgraph "Monitoring"
        Track[Access Tracking] --> Audit[Audit Log]
        Audit --> Verify[Access Verification]
    end
```

```mermaid
sequenceDiagram
    participant User
    participant Blast
    participant ResourceMgr
    participant FSecurity
    participant Container

    User->>Blast: Create Environment
    Blast->>ResourceMgr: Initialize Limits
    ResourceMgr->>Container: Apply Resource Controls
    
    Note over ResourceMgr: CPU Quota/Period
    Note over ResourceMgr: Memory Limits
    Note over ResourceMgr: I/O Controls
    
    Blast->>FSecurity: Setup Security
    FSecurity->>Container: Configure Mounts
    FSecurity->>Container: Setup Access Tracking
    
    Note over FSecurity: Validate Mounts
    Note over FSecurity: Monitor Access
    Note over FSecurity: Track Violations
    
    Container-->>Blast: Environment Ready
    Blast-->>User: Environment Active
```