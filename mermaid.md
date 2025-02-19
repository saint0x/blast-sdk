# Blast System Architecture

```mermaid
%%{init: {
  'theme': 'base',
  'themeVariables': {
    'primaryColor': '#2D3748',
    'primaryTextColor': '#fff',
    'primaryBorderColor': '#2D3748',
    'lineColor': '#4A5568',
    'secondaryColor': '#EDF2F7',
    'tertiaryColor': '#E2E8F0'
  }
}}%%

flowchart LR
    %% Styling
    classDef shellClass fill:#4299E1,color:#FFF,stroke:#2B6CB0,stroke-width:2px
    classDef cliClass fill:#48BB78,color:#FFF,stroke:#2F855A,stroke-width:2px
    classDef envClass fill:#9F7AEA,color:#FFF,stroke:#6B46C1,stroke-width:2px
    classDef daemonClass fill:#ED64A6,color:#FFF,stroke:#B83280,stroke-width:2px
    classDef actClass fill:#F6AD55,color:#FFF,stroke:#C05621,stroke-width:2px
    classDef stateClass fill:#667EEA,color:#FFF,stroke:#434190,stroke-width:2px
    classDef defaultClass fill:#718096,color:#FFF,stroke:#4A5568,stroke-width:2px
    classDef nodeClass rx:10,ry:10

    %% User Shell Environment
    subgraph Shell["1. Shell Environment"]
        direction LR
        U(("👤 User")) --> |"blast start"| SF["🔄 Shell Function"]
        SF --> |"Creates"| TF["📄 Temp File"]
    end

    %% Blast CLI
    subgraph CLI["2. Blast CLI"]
        direction LR
        BC["⚡ blast binary"] --> |"Parse Args"| CM["🔍 Command Module"]
        CM --> |"Check Mode"| SP{"🔀 Setup or<br/>Activate?"}
        SP --> |"New Env"| Setup["🏗️ Setup Phase"]
        SP --> |"Existing"| Act["🔌 Activation Phase"]
    end

    %% Environment Creation
    subgraph Env["3. Environment Setup"]
        direction LR
        Setup --> |"Create"| Dir["📁 Directory<br/>Structure"]
        Dir --> Bin["/bin"]
        Dir --> Lib["/lib"]
        Dir --> State["/state"]
        Dir --> Hooks["/hooks"]
        Setup --> |"Generate"| AS["📜 Activation<br/>Scripts"]
        Setup --> |"Configure"| CF["⚙️ blast.cfg"]
    end

    %% Daemon Service
    subgraph Daemon["4. Background Services"]
        direction LR
        DS["👾 Daemon Service"] --> |"Manage"| SM["💾 State Manager"]
        DS --> |"Handle"| UM["🔄 Update Manager"]
        DS --> |"Control"| PM["⚡ Process Manager"]
        SM --> |"Track"| ES["📊 Environment<br/>State"]
        UM --> |"Monitor"| Deps["📦 Dependencies"]
        PM --> |"Isolate"| Proc["🔒 Processes"]
    end

    %% Script Activation
    subgraph Act["5. Activation"]
        direction LR
        Act --> |"Read"| AS
        AS --> |"Output"| TF
        TF --> |"Source"| SF
    end

    %% Environment State
    subgraph State["6. Active Environment"]
        direction LR
        SF --> |"Set"| EV["🔧 Environment<br/>Variables"]
        SF --> |"Update"| PT["🛣️ PATH"]
        SF --> |"Modify"| PS["💻 Shell Prompt"]
        EV --> |"Configure"| PY["🐍 Python<br/>Environment"]
    end

    %% Flow Connections with curved edges
    Shell --> |"Command"| CLI
    CLI --> |"Setup"| Env
    Env --> |"Start"| Daemon
    Daemon --> |"Configure"| State
    Act --> |"Modify"| State

    %% Apply classes
    class Shell,U,SF,TF shellClass
    class CLI,BC,CM,SP,Setup cliClass
    class Env,Dir,Bin,Lib,State,Hooks,AS,CF envClass
    class Daemon,DS,SM,UM,PM,ES,Deps,Proc daemonClass
    class Act,AS actClass
    class State,EV,PT,PS,PY stateClass
    class U,SF,TF,BC,CM,SP,Setup,Dir,AS,CF,DS,SM,UM,PM,ES,Deps,Proc,EV,PT,PS,PY nodeClass

style Shell fill:#4299E1,stroke:#2B6CB0,stroke-width:4px
style CLI fill:#48BB78,stroke:#2F855A,stroke-width:4px
style Env fill:#9F7AEA,stroke:#6B46C1,stroke-width:4px
style Daemon fill:#ED64A6,stroke:#B83280,stroke-width:4px
style Act fill:#F6AD55,stroke:#C05621,stroke-width:4px
style State fill:#667EEA,stroke:#434190,stroke-width:4px
```

## Flow Description

1. **Shell Environment** <span style="color: #4299E1">🔄</span>
   - User invokes `blast start`
   - Shell function intercepts command
   - Creates temporary file for activation

2. **Blast CLI** <span style="color: #48BB78">⚡</span>
   - Binary processes command
   - Determines if setup or activation needed
   - Routes to appropriate handler

3. **Environment Setup** <span style="color: #9F7AEA">🏗️</span>
   - Creates directory structure
   - Generates activation scripts
   - Sets up configuration
   - Prepares Python environment

4. **Background Services** <span style="color: #ED64A6">👾</span>
   - Daemon manages state
   - Handles dependency updates
   - Controls process isolation
   - Maintains environment state

5. **Activation** <span style="color: #F6AD55">🔌</span>
   - Reads activation script
   - Outputs to temp file
   - Shell sources the script

6. **Active Environment** <span style="color: #667EEA">💻</span>
   - Sets environment variables
   - Updates PATH
   - Modifies shell prompt
   - Configures Python environment

The system follows a clear left-to-right flow, with each component building on the previous one. The daemon service runs continuously in the background, while the activation process is a one-time operation that sets up the shell environment.

---
*Note: Colors are based on the Tailwind CSS color palette for professional, modern aesthetics. Emojis are used to enhance visual understanding of each component's function.* 
