# Code Furnace 🔥

**A next-generation terminal and IDE hybrid powered by AI agents**

Code Furnace represents the future of development environments, seamlessly blending the power of a terminal with the intelligence of modern IDEs and AI assistance. Built with Rust and React, it provides a blazing-fast, extensible platform for modern software development.

## 🚀 Overview

Code Furnace is a revolutionary development environment that combines:

- **Intelligent Terminal Management**: Multi-session terminals with context-aware AI assistance
- **Advanced Code Editor**: Monaco-based editor with full LSP support and IntelliSense
- **AI Agent System**: Specialized AI agents for code review, generation, debugging, and documentation
- **Comprehensive Git Integration**: Native support for GitHub, GitLab, Gitea, and Forgejo platforms
- **Workspace Management**: Project-aware background process monitoring and management
- **Plugin Architecture**: Extensible WASM-based plugin system for custom functionality

## 🏗️ Architecture

Code Furnace is built using a modular, event-driven architecture:

```
┌─────────────────────────────────────────────────────────────────┐
│                        Frontend (React)                        │
├─────────────────────────────────────────────────────────────────┤
│                     Tauri IPC Layer                           │
├─────────────────────────────────────────────────────────────────┤
│                    Rust Backend (Tokio)                       │
├──────────┬──────────┬──────────┬──────────┬─────────┬──────────┤
│ Terminal │ Editor   │ AI       │ Workspace│ Canvas  │ Plugins  │
│ Manager  │ Manager  │ Agents   │ Manager  │ Engine  │ Runtime  │
├──────────┴──────────┴──────────┴──────────┴─────────┴──────────┤
│                      Event Bus (Pub/Sub)                      │
└─────────────────────────────────────────────────────────────────┘
```

### Core Components

#### 🖥️ Terminal Management (`crates/terminal`)
- **Multi-session support**: Create, manage, and switch between terminal sessions
- **Process monitoring**: Real-time stdout/stderr capture and logging
- **Context awareness**: Integration with workspace and project context
- **AI assistance**: Intelligent command suggestions and explanations

#### ✏️ Code Editor (`crates/editor`)
- **Monaco integration**: Full-featured code editor with syntax highlighting
- **LSP support**: Language Server Protocol integration for IntelliSense
- **File management**: Tree view, buffer management, and workspace navigation
- **AI-powered features**: Code completion, refactoring suggestions, and documentation

#### 🤖 AI Agent System (`crates/agents`)
Specialized AI agents for different development tasks:

- **Code Explainer**: Analyzes and explains complex code
- **Code Reviewer**: Provides detailed code review feedback
- **Test Generator**: Creates comprehensive test suites
- **Git Assistant**: Helps with Git operations and conflict resolution
- **UI Designer**: Assists with interface design and styling
- **System Architect**: Provides high-level design guidance
- **Documentation Writer**: Generates and maintains project documentation
- **Debugger**: Assists with debugging and error resolution

#### 📁 Workspace Management (`crates/workspace`)
- **Project detection**: Automatic recognition of Rust, Node.js, Python projects
- **Background processes**: Start, monitor, and manage dev servers and build tools
- **Git integration**: Full Git workflow support with platform-specific APIs
- **Process monitoring**: Real-time logs and performance metrics

#### 🎨 Canvas Engine (`crates/canvas`)
- **Interactive visualizations**: System diagrams, dependency graphs
- **Real-time collaboration**: Shared whiteboards and design sessions
- **Export capabilities**: PNG, SVG, and PDF generation

#### 🔌 Plugin System (`crates/plugins`)
- **WASM runtime**: Secure, sandboxed plugin execution
- **Language support**: Plugins written in Rust, AssemblyScript, or C++
- **API bindings**: Full access to Code Furnace functionality
- **Package manager**: Built-in plugin discovery and installation

## 🛠️ Technology Stack

### Backend (Rust)
- **Tauri**: Cross-platform desktop application framework
- **Tokio**: Async runtime for high-performance I/O
- **Git2**: Native Git integration with libgit2
- **Reqwest**: HTTP client for Git platform APIs
- **Wasmtime**: WebAssembly runtime for plugins
- **SQLx**: Async SQL toolkit for data persistence
- **Tracing**: Structured logging and observability

### Frontend (React)
- **TypeScript**: Type-safe JavaScript development
- **Monaco Editor**: VS Code editor component
- **Styled Components**: CSS-in-JS styling solution
- **React Query**: Data fetching and caching
- **Zustand**: Lightweight state management
- **React DnD**: Drag and drop interactions

## 🚀 Getting Started

### Prerequisites
- **Rust** (1.75+): Install from [rustup.rs](https://rustup.rs/)
- **Node.js** (18+): Install from [nodejs.org](https://nodejs.org/)
- **Git**: Version control system

### Installation

1. **Clone the repository**:
   ```bash
   git clone https://github.com/CommonElements/code-furnace.git
   cd code-furnace
   ```

2. **Install dependencies**:
   ```bash
   # Install Rust dependencies
   cargo fetch
   
   # Install Node.js dependencies
   npm install
   ```

3. **Build and run in development mode**:
   ```bash
   npm run tauri:dev
   ```

4. **Build for production**:
   ```bash
   npm run tauri:build
   ```

### Configuration

Create a `config.json` file in your user data directory:

```json
{
  "agent_provider": "Claude",
  "agent_api_key": "your-anthropic-api-key",
  "git_platforms": {
    "github": {
      "token": "your-github-token"
    },
    "gitlab": {
      "url": "https://gitlab.com",
      "token": "your-gitlab-token"
    },
    "gitea": {
      "url": "https://your-gitea-instance.com",
      "token": "your-gitea-token"
    }
  },
  "workspace": {
    "default_projects_path": "~/Projects",
    "auto_detect_projects": true
  }
}
```

## 🌋 Molten Core Theme System

Code Furnace features a unique **volcanic theme system** inspired by the concept of molten lava flowing through dark obsidian rock. This creates an immersive coding environment that matches our "furnace" branding.

### Theme Philosophy
- **Dark Foundation**: Obsidian backgrounds (#0D0D0D to #2D2D2D) for reduced eye strain
- **Energetic Accents**: Lava-colored interactive elements (#FF4500 OrangeRed, #FF8C00 DarkOrange)
- **Visual Hierarchy**: Bright, warm highlights draw attention to important elements
- **Consistent Experience**: Unified theme across all components including Monaco editor

### Key Features
- **Dynamic Theme Switching**: CSS custom properties for instant theme changes
- **Monaco Integration**: Custom syntax highlighting with volcanic colors
- **Volcanic Animations**: Subtle lava-pulse and ember-glow effects
- **Semantic Color System**: Intuitive color naming (obsidian, lava, ember)

```typescript
// Example usage in components
<button className="bg-lava-primary text-text-inverse hover:bg-lava-secondary shadow-glow">
  Molten Button
</button>
```

## 📖 Features in Detail

### AI-Powered Development
- **Context-aware assistance**: AI agents understand your project structure and codebase
- **File context selection**: Choose specific files to provide context to AI agents
- **Intelligent code generation**: Generate boilerplate, tests, and documentation
- **Smart refactoring**: AI-suggested improvements and modernization
- **Bug detection**: Proactive identification of potential issues

### Git Integration
- **Multi-platform support**: GitHub, GitLab, Gitea, Forgejo, and generic Git
- **Visual diff viewer**: Side-by-side and unified diff views
- **Branch management**: Create, switch, merge, and rebase branches
- **Conflict resolution**: AI-assisted merge conflict resolution
- **Pull request integration**: Create, review, and manage PRs/MRs directly

### Workspace Features
- **Project templates**: Quick setup for different project types
- **Environment management**: Isolated development environments
- **Docker integration**: Container-based development workflows
- **Cloud sync**: Synchronize settings and projects across devices

### Customization
- **Molten Core Theme**: Volcanic-inspired dark theme with lava accents
- **Theme System**: Extensible theme architecture for future themes
- **Keybindings**: Fully customizable keyboard shortcuts
- **Layout**: Flexible panel arrangement and sizing
- **Extensions**: Rich plugin ecosystem for additional functionality

## 🧪 Development

### Project Structure
```
code-furnace/
├── crates/
│   ├── agents/          # AI agent system
│   ├── terminal/        # Terminal management
│   ├── editor/          # Code editor functionality
│   ├── workspace/       # Project and Git management
│   ├── canvas/          # Visualization engine
│   ├── plugins/         # Plugin runtime
│   ├── events/          # Event system
│   └── utils/           # Shared utilities
├── src/                 # React frontend
├── src-tauri/           # Tauri application wrapper
├── docs/                # Documentation
└── examples/            # Usage examples
```

### Building from Source

1. **Development build**:
   ```bash
   cargo build
   cd frontend && npm run dev
   ```

2. **Full Tauri development**:
   ```bash
   npm run tauri:dev
   ```

3. **Run tests**:
   ```bash
   cargo test --all
   npm test
   ```

4. **Lint and format**:
   ```bash
   cargo clippy --all-targets --all-features
   cargo fmt --all
   npm run lint
   ```

### Theme Development

Code Furnace uses a comprehensive theme system built with:

1. **Theme Definition** (`frontend/src/themes/molten-core.ts`):
   ```typescript
   export const moltenCoreColors: ThemeColors = {
     primary: '#FF4500',     // OrangeRed lava
     obsidian: {
       darkest: '#0D0D0D',   // Deep volcanic rock
       dark: '#1A1A1A',      // Standard obsidian
     },
     // ... complete color system
   };
   ```

2. **React Theme Provider** (`frontend/src/themes/MoltenCoreProvider.tsx`):
   ```typescript
   const { theme, setTheme } = useMoltenTheme();
   const colors = useThemeColors();
   ```

3. **Tailwind Integration** - Volcanic colors available as utility classes:
   ```html
   <div className="bg-obsidian-dark text-lava-primary border-ui-border">
     Themed Component
   </div>
   ```

4. **Monaco Editor Theme** - Custom syntax highlighting with volcanic colors

### Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details on:
- Code style and standards
- Pull request process
- Issue reporting
- Development setup
- Testing requirements

## 📄 License

Code Furnace is licensed under the [MIT License](LICENSE).

## 🤝 Community

- **Discord**: [Join our community](https://discord.gg/code-furnace)
- **GitHub Discussions**: [Share ideas and ask questions](https://github.com/CommonElements/code-furnace/discussions)
- **Twitter**: [@CodeFurnaceApp](https://twitter.com/CodeFurnaceApp)
- **Blog**: [Latest updates and tutorials](https://blog.code-furnace.dev)

## 🙏 Acknowledgments

Code Furnace is built on the shoulders of giants. Special thanks to:
- The Rust community for creating an amazing ecosystem
- The Tauri team for the excellent desktop framework
- Microsoft for the Monaco editor
- The Git community for version control excellence
- All open-source contributors who make projects like this possible

---

**Ready to forge the future of development?** 🔥

[Download Code Furnace](https://github.com/CommonElements/code-furnace/releases) | [Documentation](https://docs.code-furnace.dev) | [Community](https://discord.gg/code-furnace)