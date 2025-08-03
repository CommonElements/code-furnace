# Code Furnace - Development Memory 🔥

**Last Updated:** January 2025  
**Project Status:** Phase 1 Complete - Molten Core Theme Foundation Implemented

## 🏗️ Current Architecture Overview

Code Furnace is a **next-generation terminal and IDE hybrid** built with:
- **Backend:** Rust + Tauri (desktop app framework)
- **Frontend:** React + TypeScript with Monaco Editor
- **Theme System:** Molten Core (volcanic theme with obsidian backgrounds + lava interactive elements)
- **Multi-platform Git Integration:** GitHub, GitLab, Gitea, Forgejo support

### Directory Structure
```
code-furnace/
├── crates/                    # Rust backend modules
│   ├── agents/               # AI agent system
│   ├── workspace/            # Project management + Git integration  
│   ├── terminal/             # Terminal session management
│   ├── editor/               # File editing + LSP integration
│   ├── canvas/               # Visualization engine
│   ├── events/               # Event bus system
│   └── plugins/              # WASM plugin runtime
├── frontend/                 # React frontend
│   ├── src/components/       # UI components
│   ├── src/themes/           # Molten Core theme system
│   └── src/lib/              # Utilities + Tauri API
└── src-tauri/                # Tauri desktop app wrapper
```

## 🌋 Molten Core Theme System (COMPLETED)

### Design Philosophy
**"Dark obsidian rock with glowing lava flowing through cracks"**
- **Backgrounds:** Obsidian colors (#0D0D0D darkest → #2D2D2D light)
- **Interactive Elements:** Lava colors (#FF4500 primary, #FF8C00 secondary, #FFD700 highlights)
- **Text:** Hot metal colors (warm off-whites to cool grays)

### Implementation Details

#### 1. Theme Definition (`src/themes/molten-core.ts`)
```typescript
export const moltenCoreColors: ThemeColors = {
  primary: '#FF4500',     // OrangeRed - main interactive
  secondary: '#FF8C00',   // DarkOrange - secondary interactive
  accent: '#FFD700',      // Gold - special highlights
  
  obsidian: {
    darkest: '#0D0D0D',   // Deepest volcanic rock
    dark: '#1A1A1A',      // Standard obsidian
    medium: '#242424',    // Lighter volcanic rock
    light: '#2D2D2D',     // Ash gray
  },
  
  lava: {
    primary: '#FF4500',   // Main interactive lava
    secondary: '#FF8C00', // Hover/secondary lava
    tertiary: '#FFD700',  // Gold highlights
    ember: '#FF7043',     // Soft orange glow
  },
  // ... extensive color system
};
```

#### 2. React Theme Provider (`src/themes/MoltenCoreProvider.tsx`)
- **CSS Variable Injection:** Dynamic theme switching via CSS custom properties
- **Theme Persistence:** localStorage integration for user preferences
- **Utility Hooks:** `useMoltenTheme()`, `useThemeColors()`, `useThemeStyles()`

#### 3. Tailwind Integration (`tailwind.config.js`)
- **Complete Color System:** All volcanic colors available as Tailwind classes
- **Custom Animations:** `lava-pulse`, `ember-glow` with volcanic keyframes
- **Semantic Classes:** `bg-primary`, `text-lava-primary`, `shadow-glow`, etc.

#### 4. Monaco Editor Theme (`src/themes/monaco-molten-theme.ts`)
- **Volcanic Syntax Highlighting:** 
  - Keywords: Molten orange (#FF4500)
  - Strings: Green flame (#4CAF50) 
  - Numbers: Ember glow (#FFB74D)
  - Comments: Cool ash gray (#6A737D)
- **Editor UI:** Matching obsidian backgrounds with lava accents

### Themed Components Status
✅ **App.tsx** - Main navigation with lava-colored active tabs  
✅ **AgentPanel.tsx** - Complete volcanic styling + file context selection  
✅ **Terminal.tsx** - Obsidian command blocks with lava prompts  
✅ **Editor.tsx** - File tree + Monaco integration  
✅ **Canvas.tsx** - Design tools with volcanic color palette  

## 🤖 AI Agent System

### Specialized Agents (Implemented)
- **Code Explainer** - Analyzes complex code
- **Code Reviewer** - Provides review feedback  
- **Test Generator** - Creates test suites
- **Git Assistant** - Helps with Git operations
- **UI Designer** - Interface design assistance
- **Debugger** - Error resolution help

### Current Implementation (`crates/agents/src/lib.rs`)
```rust
pub struct AgentRouter {
    memory: Arc<RwLock<AgentMemory>>,
    specialized_agents: HashMap<String, Box<dyn SpecializedAgent>>,
    event_bus: EventBus,
}

// Agent selection logic with context awareness
impl AgentRouter {
    pub async fn route_request(&self, request: &AgentRequest) -> Result<AgentResponse> {
        let selected_agent = self.select_agent_for_request(request).await?;
        // Route to appropriate specialized agent...
    }
}
```

### Agent Memory System
- **Long-term Memory:** Persistent storage of conversations
- **Context Windows:** File-aware prompts with project understanding
- **Learning:** Adapts responses based on user patterns

## 🖥️ Terminal System

### Multi-Session Management (`crates/terminal/src/lib.rs`)
```rust
pub struct TerminalManager {
    sessions: Arc<RwLock<HashMap<Uuid, TerminalSession>>>,
    active_session: Arc<RwLock<Option<Uuid>>>,
    background_processes: Arc<RwLock<HashMap<Uuid, BackgroundProcess>>>,
    event_bus: EventBus,
}
```

### Features Implemented
- **Real Process Execution:** PTY support with stdout/stderr capture
- **Background Process Monitoring:** Dev servers, build tools, etc.
- **AI Integration:** Command explanations and suggestions
- **Session Persistence:** Resume sessions across app restarts

## 📁 Workspace & Git Integration

### Comprehensive Git Support (`crates/workspace/src/git.rs`)
```rust
pub struct GitManager {
    repositories: HashMap<PathBuf, GitRepository>,
    platforms: GitPlatforms,
}

pub struct GitPlatforms {
    pub github: Option<GitHubClient>,
    pub gitlab: Option<GitLabClient>, 
    pub gitea: Option<GiteaClient>,
}
```

### Platform Integration Status
✅ **GitHub API** - PRs, issues, actions  
✅ **GitLab API** - MRs, issues, pipelines  
✅ **Gitea/Forgejo** - Self-hosted Git support  
✅ **Core Git Operations** - Status, commit, branch, diff  
✅ **AI Commit Messages** - Generated based on staged changes  

## 📝 Editor System

### Monaco Integration (`src/components/Editor.tsx`)
- **LSP Support:** Language Server Protocol for IntelliSense
- **File Tree:** Project navigation with search
- **Multi-buffer:** Tabbed editing interface
- **Molten Core Theme:** Integrated volcanic syntax highlighting

### LSP Integration Status (`crates/editor/src/lsp.rs`)
```rust
pub struct LSPManager {
    servers: HashMap<String, LanguageServer>,
    client_capabilities: ClientCapabilities,
}
```
⚠️ **TODO:** Complete response parsing for completion items and hover info (lines 297, 329)

## 🎨 Canvas System

### Visualization Engine (`src/components/Canvas.tsx`)
- **Multiple Modes:** Freeform, wireframe, flowchart, system design
- **Tool Palette:** Select, rectangle, circle, text tools
- **Volcanic Styling:** Lava-colored tools with obsidian backgrounds

### Future Enhancements
- **Export Capabilities:** PNG, SVG, PDF generation
- **Real-time Collaboration:** Shared design sessions
- **Architecture Diagrams:** System design templates

## 🚀 Build & Development

### Development Commands
```bash
# Frontend development
cd frontend && npm run dev

# Build frontend
cd frontend && npm run build

# Full Tauri development  
npm run tauri:dev

# Production build
npm run tauri:build
```

### Key Dependencies
**Frontend:**
- React 18 + TypeScript
- Monaco Editor for code editing
- Tailwind CSS with custom theme
- Lucide React for icons

**Backend:**
- Tauri 2.0 for desktop framework
- Tokio for async runtime
- Git2 for Git integration
- Reqwest for HTTP clients

## ⚠️ Known TODOs & Issues

### High Priority (Functional)
1. **LSP Response Parsing** (`crates/editor/src/lsp.rs:297,329`)
   - Complete completion items and hover info parsing

2. **Folder Dialog** (`src/components/Editor.tsx:293`)
   - Implement native folder selection dialog

3. **Agent API Integration** (`src/components/AgentPanel.tsx:63`)
   - Complete Tauri command integration for AI agents

### Medium Priority (Features)
4. **Dynamic Panel System** - react-grid-layout integration
5. **Command Palette System** - Keyboard-driven command interface
6. **Plugin System** - WASM runtime for extensions

## 📊 Project Status

### Phase 1: Molten Core Theme Foundation ✅ COMPLETE
- [x] Complete volcanic theme system
- [x] React theme provider with CSS variables
- [x] Tailwind integration with volcanic colors
- [x] Monaco editor theme with syntax highlighting
- [x] All components themed consistently
- [x] File context selection for AI agents

### Phase 2: Core Functionality (IN PROGRESS)
- [ ] Complete LSP integration
- [ ] Folder dialog implementation
- [ ] Agent API integration
- [ ] Testing and refinement

### Phase 3: Advanced Features (PLANNED)
- [ ] Dynamic panel system
- [ ] Command palette
- [ ] Plugin architecture
- [ ] Performance optimization

## 🔧 Development Guidelines

### Theme Usage Patterns
```tsx
// Using theme colors in components
<button className="bg-lava-primary text-text-inverse hover:bg-lava-secondary transition-fast shadow-glow">
  Action Button
</button>

// Using theme hooks
const { theme } = useMoltenTheme();
const colors = useThemeColors();
const styles = useThemeStyles();
```

### Component Architecture
- **Consistent theming** - Always use semantic color classes
- **Volcanic animations** - Leverage `lava-pulse`, `ember-glow` animations
- **Transition classes** - Use `transition-fast` for smooth interactions
- **Shadow effects** - Apply `shadow-glow` to interactive elements

### Git Workflow
- **Feature branches** from main
- **Conventional commits** with scope prefixes
- **Theme consistency** across all new components
- **Test builds** before committing theme changes

## 🎯 Next Steps

1. **Complete Critical TODOs** - LSP parsing, folder dialogs, Agent API
2. **Documentation Update** - README.md with theme showcase
3. **Advanced UI Features** - Dynamic panels, command palette
4. **Performance Optimization** - Bundle splitting, lazy loading
5. **Testing Suite** - Component tests, integration tests

---

**Remember:** Code Furnace aims to be a **volcanic development environment** - dark, powerful foundations with bright, energetic interactive elements that draw attention and provide an immersive coding experience. The Molten Core theme system is now fully implemented and ready for advanced features! 🔥