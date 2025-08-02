# Code Furnace Architecture 🏗️

This document provides a comprehensive overview of Code Furnace's architecture, design decisions, and implementation patterns.

## 📋 Table of Contents
- [Overview](#overview)
- [System Architecture](#system-architecture)
- [Backend Architecture](#backend-architecture)
- [Frontend Architecture](#frontend-architecture)
- [Data Flow](#data-flow)
- [Security Model](#security-model)
- [Performance Considerations](#performance-considerations)
- [Extensibility](#extensibility)

## 🎯 Overview

Code Furnace is built using a **modular, event-driven architecture** that separates concerns across multiple layers while maintaining high performance and extensibility. The system is designed around the principle of **"composable development tools"** where each component can operate independently while contributing to a cohesive user experience.

### Core Design Principles
1. **Modularity**: Clear separation of concerns with well-defined interfaces
2. **Performance**: Async-first design with efficient resource utilization
3. **Extensibility**: Plugin architecture for unlimited customization
4. **Security**: Sandboxed execution and secure inter-process communication
5. **User Experience**: Responsive, intuitive interface with AI assistance

## 🏛️ System Architecture

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                                 Frontend Layer                                 │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐                │
│  │   Terminal      │  │   Code Editor   │  │   Canvas        │                │
│  │   Component     │  │   Component     │  │   Component     │                │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘                │
│                                  │                                             │
│  ┌─────────────────────────────────────────────────────────────────────────┐   │
│  │                      React Application Shell                           │   │
│  │  • State Management (Zustand)                                          │   │
│  │  • Routing & Navigation                                                 │   │
│  │  • Theme & Layout Management                                            │   │
│  │  • WebSocket & HTTP Communication                                       │   │
│  └─────────────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────────────┘
                                         │
                                   Tauri IPC Layer
                                         │
┌─────────────────────────────────────────────────────────────────────────────────┐
│                                Backend Layer (Rust)                            │
│                                                                                 │
│  ┌─────────────────────────────────────────────────────────────────────────┐   │
│  │                         Application State                               │   │
│  │  • Shared managers and configurations                                   │   │
│  │  • Cross-component communication                                        │   │
│  └─────────────────────────────────────────────────────────────────────────┘   │
│                                         │                                       │
│  ┌─────────────┬───────────────┬─────────────┬─────────────┬─────────────────┐ │
│  │  Terminal   │   Editor      │  Workspace  │  AI Agents  │   Plugins       │ │
│  │  Manager    │   Manager     │  Manager    │  System     │   Runtime       │ │
│  │             │               │             │             │                 │ │
│  │ • Sessions  │ • Buffers     │ • Projects  │ • Providers │ • WASM Engine   │ │
│  │ • Processes │ • LSP Client  │ • Git Ops   │ • Routing   │ • API Bindings  │ │
│  │ • I/O       │ • File Tree   │ • Monitors  │ • Context   │ • Sandboxing    │ │
│  └─────────────┴───────────────┴─────────────┴─────────────┴─────────────────┘ │
│                                         │                                       │
│  ┌─────────────────────────────────────────────────────────────────────────┐   │
│  │                          Event Bus (Pub/Sub)                           │   │
│  │  • Cross-manager communication                                          │   │
│  │  • Async event distribution                                             │   │
│  │  • Plugin event integration                                             │   │
│  └─────────────────────────────────────────────────────────────────────────┘   │
│                                         │                                       │
│  ┌─────────────────────────────────────────────────────────────────────────┐   │
│  │                        Persistence Layer                               │   │
│  │  • SQLite Database (Projects, Sessions, Config)                        │   │
│  │  • File System (Code, Logs, Artifacts)                                 │   │
│  │  • Git Repositories (Version Control)                                  │   │
│  └─────────────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────────────┘
```

## 🦀 Backend Architecture

The backend is implemented in **Rust** using an **async-first approach** with **Tokio** as the runtime. The architecture emphasizes modularity, type safety, and performance.

### Component Structure

```
crates/
├── agents/           # AI Agent System
├── terminal/         # Terminal Management  
├── editor/           # Code Editor Backend
├── workspace/        # Project & Git Management
├── canvas/           # Visualization Engine
├── plugins/          # Plugin Runtime
├── events/           # Event System
└── utils/            # Shared Utilities
```

### Core Managers

#### 🖥️ Terminal Manager (`crates/terminal`)

```rust
pub struct TerminalManager {
    sessions: Arc<RwLock<HashMap<Uuid, TerminalSession>>>,
    active_session: Arc<RwLock<Option<Uuid>>>,
    event_bus: EventBus,
}

pub struct TerminalSession {
    pub id: Uuid,
    pub name: String,
    pub working_directory: PathBuf,
    pub status: SessionStatus,
    pub blocks: Vec<CommandBlock>,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
}
```

**Key Features:**
- **Multi-session Management**: Create, switch, and manage multiple terminal sessions
- **Process Execution**: Spawn and monitor child processes with real-time I/O
- **Command History**: Persistent command history with search capabilities
- **Session Persistence**: Save and restore terminal sessions across app restarts

**Implementation Patterns:**
- **Async Process Management**: Uses `tokio::process::Command` for non-blocking execution
- **Stream Processing**: Handles stdout/stderr streams with `BufReader` and async iteration
- **Resource Cleanup**: Proper cleanup of processes and file handles on session termination

#### ✏️ Editor Manager (`crates/editor`)

```rust
pub struct EditorManager {
    buffers: Arc<RwLock<HashMap<Uuid, FileBuffer>>>,
    workspace_root: Arc<RwLock<Option<PathBuf>>>,
    lsp_clients: Arc<RwLock<HashMap<String, LSPClient>>>,
    event_bus: EventBus,
}

pub struct FileBuffer {
    pub id: Uuid,
    pub path: PathBuf,
    pub content: String,
    pub language: String,
    pub is_dirty: bool,
    pub last_modified: DateTime<Utc>,
}
```

**Key Features:**
- **File Buffer Management**: In-memory file editing with dirty state tracking
- **LSP Integration**: Language Server Protocol support for IntelliSense
- **Workspace Navigation**: File tree generation and workspace-aware operations
- **Syntax Highlighting**: Language detection and syntax highlighting support

**Implementation Patterns:**
- **Lazy Loading**: Files are loaded into buffers only when accessed
- **Change Tracking**: Efficient diff-based change tracking for large files
- **LSP Communication**: JSON-RPC communication with language servers

#### 📁 Workspace Manager (`crates/workspace`)

```rust
pub struct WorkspaceManager {
    projects: Arc<RwLock<HashMap<Uuid, Project>>>,
    active_project: Arc<RwLock<Option<Uuid>>>,
    background_processes: Arc<RwLock<HashMap<Uuid, BackgroundProcess>>>,
    running_processes: Arc<RwLock<HashMap<Uuid, Child>>>,
    git_manager: Arc<RwLock<GitManager>>,
    event_bus: EventBus,
}
```

**Key Features:**
- **Project Detection**: Automatic detection of project types (Rust, Node.js, Python)
- **Background Processes**: Dev server management with auto-restart capabilities
- **Git Integration**: Full Git workflow support with platform-specific APIs
- **Process Monitoring**: Real-time process monitoring with log aggregation

**Implementation Patterns:**
- **Process Lifecycle Management**: Spawn, monitor, and cleanup background processes
- **Git Operations**: Integration with `git2` library for native Git operations
- **Event-Driven Updates**: Real-time updates via event system

#### 🤖 AI Agent System (`crates/agents`)

```rust
pub struct AgentBridge {
    providers: HashMap<String, Box<dyn AgentProvider>>,
    default_provider: Option<String>,
    specialized_agents: HashMap<AgentType, Box<dyn AgentProvider>>,
    conversations: HashMap<Uuid, ConversationThread>,
    active_conversation: Option<Uuid>,
}

pub trait AgentProvider: Send + Sync {
    async fn process_request(&self, request: AgentRequest) -> Result<AgentResponse>;
    fn supported_features(&self) -> Vec<AgentFeature>;
}
```

**Key Features:**
- **Provider Abstraction**: Support for multiple AI providers (Claude, GPT, etc.)
- **Specialized Agents**: Task-specific agents with domain expertise
- **Conversation Management**: Persistent conversation threads with context
- **Context Awareness**: File and project context integration

**Implementation Patterns:**
- **Plugin Architecture**: Providers implement common trait interface
- **Async Communication**: Non-blocking HTTP requests to AI APIs
- **Context Management**: Efficient context window management for large codebases

### Event System (`crates/events`)

```rust
pub struct EventBus {
    channels: Arc<RwLock<HashMap<String, Vec<EventSender>>>>,
}

pub struct Event {
    pub id: Uuid,
    pub event_type: String,
    pub source: String,
    pub data: serde_json::Value,
    pub timestamp: DateTime<Utc>,
}
```

**Key Features:**
- **Pub/Sub Messaging**: Decoupled communication between components
- **Type-Safe Events**: Strongly typed event data with serialization
- **Async Dispatch**: Non-blocking event distribution
- **Plugin Integration**: Events accessible to plugins via API

## ⚛️ Frontend Architecture

The frontend is built with **React** and **TypeScript**, following modern React patterns with hooks and functional components.

### Component Hierarchy

```
src/
├── components/        # Reusable UI components
│   ├── Terminal/     # Terminal-related components
│   ├── Editor/       # Code editor components  
│   ├── Workspace/    # Workspace management
│   ├── Canvas/       # Canvas and visualization
│   └── Common/       # Shared UI components
├── views/            # Main application views
├── hooks/            # Custom React hooks
├── stores/           # State management
├── utils/            # Frontend utilities
└── types/            # TypeScript definitions
```

### State Management

Code Furnace uses **Zustand** for state management, providing a lightweight and TypeScript-friendly alternative to Redux.

```typescript
interface AppState {
  // Terminal state
  terminalSessions: TerminalSession[];
  activeSessionId: string | null;
  
  // Editor state
  openBuffers: FileBuffer[];
  activeBufferId: string | null;
  workspaceRoot: string | null;
  
  // Workspace state
  projects: Project[];
  activeProjectId: string | null;
  backgroundProcesses: BackgroundProcess[];
  
  // UI state
  layout: LayoutConfig;
  theme: ThemeConfig;
  
  // Actions
  actions: {
    terminal: TerminalActions;
    editor: EditorActions;
    workspace: WorkspaceActions;
    ui: UIActions;
  };
}
```

### Communication Layer

Frontend-backend communication uses **Tauri's IPC system** with strongly typed command interfaces:

```typescript
// Terminal commands
await invoke<string>('create_terminal_session', {
  name: 'Main Terminal',
  workingDirectory: '/Users/dev/project'
});

// Editor commands  
await invoke<string>('open_file', {
  filePath: '/path/to/file.rs'
});

// Workspace commands
await invoke<Project[]>('list_projects');
```

## 🔄 Data Flow

### Command Execution Flow

```
User Input → React Component → Tauri Command → Rust Handler → Manager → Response
     ↓                                                           ↓
Event Bus ← Process Update ← Background Task ← Manager State Update
     ↓
Frontend Update ← WebSocket/Event Stream ← Event Distribution
```

### Example: Terminal Command Execution

1. **User types command** in terminal component
2. **Frontend sends IPC command** to backend
3. **TerminalManager receives command** and spawns process
4. **Process output streams** are captured asynchronously  
5. **Output events published** to event bus
6. **Frontend receives events** via WebSocket
7. **Terminal component updates** with new output

### Git Operation Flow

```
Git Action → WorkspaceManager → GitManager → git2 Library → Git Repository
     ↓                             ↓              ↓
Event Bus ← Status Update ← Platform API ← Remote Repository
     ↓
Frontend Update ← Git Status Event ← Event Distribution
```

## 🔒 Security Model

### Tauri Security
- **Sandboxed WebView**: Frontend runs in isolated context
- **API Allowlisting**: Only explicitly allowed commands accessible
- **CSP Headers**: Content Security Policy prevents XSS attacks
- **Secure Context**: All communication over secure channels

### Plugin Security
- **WASM Sandboxing**: Plugins run in isolated WebAssembly environment
- **Capability-Based Security**: Plugins request specific permissions
- **Resource Limits**: CPU and memory limits for plugin execution
- **API Surface Control**: Limited API surface exposed to plugins

### Data Security
- **Local Storage**: All data stored locally by default
- **Encrypted Configuration**: Sensitive config data encrypted at rest
- **Secure Defaults**: Secure configuration defaults throughout
- **Audit Logging**: Security-relevant events logged for analysis

## ⚡ Performance Considerations

### Backend Performance
- **Async I/O**: Non-blocking I/O operations throughout
- **Memory Management**: Efficient memory usage with Arc/Rc patterns
- **Process Pooling**: Reuse of expensive resources like LSP clients
- **Lazy Initialization**: Components initialized only when needed

### Frontend Performance
- **Virtual Scrolling**: Efficient rendering of large terminal outputs
- **Code Splitting**: Dynamic imports for feature modules
- **Memoization**: React.memo and useMemo for expensive operations
- **Debounced Updates**: Rate-limited updates for high-frequency events

### Database Performance
- **SQLite Optimization**: Proper indexing and query optimization
- **Connection Pooling**: Efficient database connection management
- **Batch Operations**: Bulk operations for large datasets
- **Background Cleanup**: Periodic cleanup of old data

## 🔌 Extensibility

### Plugin Architecture

```rust
// Plugin trait definition
pub trait Plugin: Send + Sync {
    fn initialize(&mut self, api: &PluginAPI) -> Result<()>;
    fn handle_command(&self, command: &str, args: &[String]) -> Result<String>;
    fn handle_event(&self, event: &Event) -> Result<()>;
}

// Plugin API surface
pub struct PluginAPI {
    pub terminal: TerminalAPI,
    pub editor: EditorAPI,
    pub workspace: WorkspaceAPI,
    pub canvas: CanvasAPI,
}
```

### Extension Points
- **Custom Commands**: Plugins can register new commands
- **Event Handlers**: React to system events
- **UI Components**: Custom React components via portal system
- **Language Support**: Add new language servers and syntax highlighting
- **Git Platforms**: Implement new Git platform integrations
- **AI Providers**: Add support for additional AI services

### Configuration System
- **Layered Configuration**: User, project, and system-level configs
- **Schema Validation**: JSON schema validation for all config
- **Hot Reloading**: Configuration changes applied without restart
- **Environment Variables**: Support for environment-based configuration

---

This architecture enables Code Furnace to be both powerful and extensible while maintaining excellent performance and security. The modular design allows for independent development and testing of components while the event system ensures they work together seamlessly.