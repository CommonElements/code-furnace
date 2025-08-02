# Contributing to Code Furnace 🔥

Thank you for your interest in contributing to Code Furnace! This document provides guidelines and information for contributors.

## 🚀 Getting Started

### Prerequisites
- **Rust** 1.75+ with `rustup`
- **Node.js** 18+ with `npm`
- **Git** for version control
- **VS Code** or similar editor (recommended)

### Development Setup

1. **Fork and clone the repository**:
   ```bash
   git clone https://github.com/your-username/code-furnace.git
   cd code-furnace
   ```

2. **Install dependencies**:
   ```bash
   cargo fetch
   npm install
   ```

3. **Run in development mode**:
   ```bash
   npm run tauri:dev
   ```

4. **Run tests**:
   ```bash
   cargo test --all
   npm test
   ```

## 🏗️ Project Architecture

Code Furnace follows a modular architecture with clear separation of concerns:

### Backend (Rust)
- **`crates/agents/`**: AI agent system with provider abstraction
- **`crates/terminal/`**: Terminal session management and process execution  
- **`crates/editor/`**: File editing, LSP integration, and buffer management
- **`crates/workspace/`**: Project management and Git integration
- **`crates/canvas/`**: Visualization and diagramming engine
- **`crates/plugins/`**: WASM plugin runtime and API
- **`crates/events/`**: Event bus for component communication
- **`crates/utils/`**: Shared utilities and configuration

### Frontend (React/TypeScript)
- **`src/components/`**: Reusable UI components
- **`src/views/`**: Main application views
- **`src/hooks/`**: Custom React hooks
- **`src/stores/`**: State management with Zustand
- **`src/utils/`**: Frontend utilities and helpers

## 📝 Code Style and Standards

### Rust Code Style
- Use `rustfmt` for formatting: `cargo fmt --all`
- Use `clippy` for linting: `cargo clippy --all-targets --all-features`
- Follow standard Rust naming conventions
- Write comprehensive documentation comments for public APIs
- Prefer explicit error handling over `unwrap()`/`expect()`

Example:
```rust
/// Represents a Git repository with enhanced metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitRepository {
    /// Absolute path to the repository root
    pub path: PathBuf,
    /// Current branch name
    pub branch: String,
    /// Repository status (staged, unstaged, etc.)
    pub status: GitStatus,
    /// Remote origin URL if available
    pub remote_url: Option<String>,
    /// Detected Git platform (GitHub, GitLab, etc.)
    pub platform: GitPlatform,
}

impl GitRepository {
    /// Opens and analyzes a Git repository at the specified path
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, GitError> {
        // Implementation...
    }
}
```

### TypeScript/React Code Style
- Use `prettier` for formatting: `npm run format`
- Use `eslint` for linting: `npm run lint`
- Follow React functional component patterns
- Use TypeScript strict mode
- Prefer custom hooks for complex state logic

Example:
```typescript
interface TerminalSessionProps {
  sessionId: string;
  onCommand?: (command: string) => void;
}

export const TerminalSession: React.FC<TerminalSessionProps> = ({
  sessionId,
  onCommand,
}) => {
  const { session, isLoading, error } = useTerminalSession(sessionId);
  
  if (isLoading) return <LoadingSpinner />;
  if (error) return <ErrorMessage error={error} />;
  
  return (
    <div className="terminal-session">
      {/* Implementation */}
    </div>
  );
};
```

### Documentation Standards
- All public functions and structs must have doc comments
- Include examples in doc comments where helpful
- Update README.md for significant feature additions
- Write integration tests for major features

## 🐛 Issue Reporting

### Before Submitting an Issue
1. Check existing issues to avoid duplicates
2. Update to the latest version
3. Provide a minimal reproduction case
4. Include system information (OS, Rust version, Node version)

### Issue Template
```markdown
**Describe the bug**
A clear description of what the bug is.

**To Reproduce**
Steps to reproduce the behavior:
1. Go to '...'
2. Click on '....'
3. See error

**Expected behavior**
What you expected to happen.

**Screenshots**
If applicable, add screenshots.

**Environment:**
- OS: [e.g. macOS 14.0]
- Rust version: [e.g. 1.75.0]
- Node version: [e.g. 20.0.0]
- Code Furnace version: [e.g. 0.1.0]

**Additional context**
Any other context about the problem.
```

## 🔄 Pull Request Process

### Before Submitting a PR
1. **Create an issue** first for significant changes
2. **Fork the repository** and create a feature branch
3. **Write tests** for new functionality
4. **Update documentation** as needed
5. **Ensure all tests pass** locally

### PR Guidelines
- Use descriptive commit messages following [Conventional Commits](https://conventionalcommits.org/)
- Keep PRs focused on a single feature or fix
- Include tests for new functionality
- Update relevant documentation
- Reference related issues in the PR description

### Commit Message Format
```
type(scope): description

[optional body]

[optional footer]
```

Examples:
```
feat(terminal): add multi-session support
fix(git): resolve merge conflict detection bug
docs(readme): update installation instructions
test(workspace): add background process tests
```

### PR Template
```markdown
## Description
Brief description of the changes.

## Type of Change
- [ ] Bug fix
- [ ] New feature  
- [ ] Breaking change
- [ ] Documentation update
- [ ] Refactoring

## Testing
- [ ] Unit tests pass
- [ ] Integration tests pass
- [ ] Manual testing completed

## Checklist
- [ ] Code follows style guidelines
- [ ] Self-review completed
- [ ] Documentation updated
- [ ] Tests added/updated
```

## 🧪 Testing Guidelines

### Rust Tests
```bash
# Run all tests
cargo test --all

# Run tests with coverage
cargo test --all --features test-coverage

# Run specific test module
cargo test --package code-furnace-workspace git_tests
```

### Frontend Tests
```bash
# Run React tests
npm test

# Run with coverage
npm run test:coverage

# Run specific test file
npm test -- TerminalSession.test.tsx
```

### Integration Tests
```bash
# Run full integration test suite
npm run test:integration
```

## 🏷️ Component Guidelines

### Creating New Components

#### Rust Components
1. **Create module structure**:
   ```
   crates/my-component/
   ├── Cargo.toml
   ├── src/
   │   ├── lib.rs        # Main module
   │   ├── types.rs      # Data structures
   │   ├── manager.rs    # Core logic
   │   └── tests.rs      # Unit tests
   ```

2. **Follow async patterns**:
   ```rust
   pub struct MyManager {
       data: Arc<RwLock<HashMap<Uuid, MyData>>>,
       event_bus: EventBus,
   }
   
   impl MyManager {
       pub async fn create_item(&self, name: String) -> Result<Uuid> {
           // Implementation
       }
   }
   ```

#### React Components
1. **Use functional components with hooks**
2. **Implement proper error boundaries**
3. **Follow accessibility guidelines**
4. **Use semantic HTML elements**

## 🔌 Plugin Development

### Creating a Plugin
1. **Set up WASM target**:
   ```bash
   rustup target add wasm32-wasi
   ```

2. **Create plugin structure**:
   ```rust
   use code_furnace_plugin_api::*;
   
   #[plugin_export]
   fn init() -> PluginResult<()> {
       register_command("my-command", my_command_handler)?;
       Ok(())
   }
   
   fn my_command_handler(args: &[String]) -> PluginResult<String> {
       // Implementation
   }
   ```

3. **Build and test**:
   ```bash
   cargo build --target wasm32-wasi --release
   ```

## 🚀 Release Process

### Version Management
- Follow [Semantic Versioning](https://semver.org/)
- Update version in `Cargo.toml` and `package.json`
- Create changelog entries for all changes

### Release Checklist
- [ ] All tests pass
- [ ] Documentation updated
- [ ] Version numbers bumped
- [ ] Changelog updated
- [ ] Git tag created
- [ ] Release notes prepared

## 🤝 Community Guidelines

### Code of Conduct
We follow the [Rust Code of Conduct](https://www.rust-lang.org/policies/code-of-conduct). Be respectful, inclusive, and constructive in all interactions.

### Getting Help
- **GitHub Discussions**: General questions and ideas
- **Discord**: Real-time chat and collaboration
- **Stack Overflow**: Technical questions (tag: `code-furnace`)

### Recognition
Contributors are recognized in:
- README.md acknowledgments
- Release notes
- Annual contributor highlights

## 📚 Additional Resources

### Learning Resources
- [Rust Book](https://doc.rust-lang.org/book/)
- [Tauri Documentation](https://tauri.app/v1/guides/)
- [React Documentation](https://react.dev/)
- [TypeScript Handbook](https://www.typescriptlang.org/docs/)

### Development Tools
- **VS Code Extensions**: Rust Analyzer, Tauri, ES7+ React
- **CLI Tools**: `cargo-watch`, `cargo-expand`, `cargo-audit`
- **Debugging**: Tauri DevTools, Chrome DevTools

---

Thank you for contributing to Code Furnace! Your efforts help make development better for everyone. 🔥