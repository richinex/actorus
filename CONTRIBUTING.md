# Contributing to Actorus

Thank you for your interest in contributing to Actorus. This document provides guidelines and information for contributors.

## Code of Conduct

- Be respectful and constructive
- Welcome newcomers and help them learn
- Focus on what is best for the community
- Show empathy towards other contributors

## Getting Started

### Prerequisites

- Rust 1.70 or later
- OpenAI API key for testing
- Familiarity with async Rust and Tokio
- Understanding of actor pattern (helpful but not required)

### Development Setup

1. Clone the repository:
```bash
git clone https://github.com/yourusername/actorus.git
cd actorus
```

2. Set up environment variables:
```bash
cp .env.example .env
# Edit .env and add your OPENAI_API_KEY
```

3. Build the project:
```bash
cargo build
```

4. Run tests:
```bash
cargo test
```

5. Run examples:
```bash
cargo run --example simple_usage
```

## Project Structure

```
actorus/
├── src/
│   ├── actors/          # Actor system implementation
│   ├── api/             # Public API interface
│   ├── core/            # Core functionality (LLM, MCP)
│   ├── tools/           # Tool system
│   ├── storage/         # Storage backends
│   └── utils/           # Utilities
├── examples/            # Example programs
├── actorus_macros/   # Procedural macros
└── tests/              # Integration tests
```

## Development Guidelines

### Information Hiding Principle

This project follows the information hiding principle from Parnas (1972). When adding new features:

1. **Hide design decisions** that are likely to change
2. **Expose stable interfaces** that won't change
3. **Modules should encapsulate** implementation details

**Good Example**:
```rust
// Public interface - stable
pub async fn discover_mcp_tools(
    server_command: &str,
    server_args: Vec<&str>
) -> Result<Vec<Arc<dyn Tool>>>;

// Implementation details - hidden
struct MCPToolWrapper {
    tool_name: String,
    description: String,
    input_schema: Value,
    // These can change without affecting users
}
```

**Bad Example**:
```rust
// Exposing implementation details
pub struct MCPToolWrapper {
    pub tool_name: String,
    pub description: String,
    pub input_schema: Value,
}
// Users depend on internal structure - hard to change
```

### Code Style

- Follow Rust standard naming conventions
- Use `rustfmt` for formatting: `cargo fmt`
- Run `clippy` for linting: `cargo clippy`
- Write documentation for public APIs
- Add examples for new features

### Actor Pattern Guidelines

When working with actors:

1. **Isolated State**: Actors should not share state
2. **Message Passing**: All communication via messages
3. **No Direct Access**: Never access actor internals directly
4. **Async Operations**: Use async/await for all actor operations

**Example**:
```rust
// Good - message passing
router.send_message(RoutingMessage::GetState(response_tx)).await?;

// Bad - direct access
let state = actor.internal_state; // Don't do this
```

### Testing Guidelines

1. **Unit Tests**: Test individual functions and modules
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tool_discovery() {
        // Test implementation
    }
}
```

2. **Integration Tests**: Test complete workflows
```rust
// tests/integration_test.rs
#[tokio::test]
async fn test_agent_pipeline() {
    // Test complete pipeline
}
```

3. **Example Programs**: Create runnable examples
```rust
// examples/new_feature.rs
#[tokio::main]
async fn main() -> Result<()> {
    // Demonstrate the feature
}
```

## Making Changes

### Branching Strategy

- `main` - stable, production-ready code
- `develop` - integration branch for features
- `feature/*` - new features
- `fix/*` - bug fixes
- `docs/*` - documentation updates

### Pull Request Process

1. **Create a feature branch**:
```bash
git checkout -b feature/your-feature-name
```

2. **Make your changes**:
   - Write code following guidelines
   - Add tests for new functionality
   - Update documentation
   - Add examples if appropriate

3. **Test thoroughly**:
```bash
cargo test
cargo clippy
cargo fmt --check
```

4. **Commit with clear messages**:
```bash
git commit -m "Add dynamic MCP tool discovery

- Implement discover_mcp_tools() function
- Add MCPToolWrapper for automatic conversion
- Include mcp_discover_tools example
- Update documentation"
```

5. **Push and create PR**:
```bash
git push origin feature/your-feature-name
```

6. **PR Description Template**:
```markdown
## Description
Brief description of changes

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Documentation update

## Testing
- [ ] Unit tests pass
- [ ] Integration tests pass
- [ ] Manual testing completed

## Checklist
- [ ] Code follows style guidelines
- [ ] Self-review completed
- [ ] Comments added where needed
- [ ] Documentation updated
- [ ] No new warnings
- [ ] Tests added
```

## Areas for Contribution

### High Priority

1. **MCP Server Integration**
   - Add more MCP server examples
   - Improve error handling
   - Add server discovery mechanisms

2. **Tool System**
   - More built-in tools
   - Tool composition patterns
   - Tool validation improvements

3. **Validation Framework**
   - Additional validation rules
   - Custom validators
   - Validation reporting

### Medium Priority

4. **Agent System**
   - Agent templates
   - Agent lifecycle hooks
   - Agent collaboration patterns

5. **Performance**
   - Benchmarking suite
   - Performance optimizations
   - Memory usage profiling

6. **Documentation**
   - More examples
   - Tutorial series
   - Architecture deep-dives

### Good First Issues

- Add new tool examples
- Improve error messages
- Write documentation
- Add unit tests
- Create example programs

## Documentation

### Code Documentation

Use Rust doc comments for public APIs:

```rust
/// Discovers all tools from an MCP server and creates tool wrappers.
///
/// This is the main entry point for integrating MCP servers into your agent system.
///
/// # Arguments
///
/// * `server_command` - Command to launch the MCP server
/// * `server_args` - Arguments to pass to the server
///
/// # Returns
///
/// A vector of wrapped tools ready for use in agents
///
/// # Example
///
/// ```no_run
/// let tools = discover_mcp_tools(
///     "npx",
///     vec!["-y", "@modelcontextprotocol/server-brave-search"]
/// ).await?;
/// ```
pub async fn discover_mcp_tools(
    server_command: &str,
    server_args: Vec<&str>,
) -> Result<Vec<Arc<dyn Tool>>> {
    // Implementation
}
```

### README and Guides

- Keep README.md up to date
- Add examples to EXAMPLES.md
- Document architecture in ARCHITECTURE.md
- Update ACTOR_AGENTS.md for actor changes

## Release Process

1. Update version in Cargo.toml
2. Update CHANGELOG.md
3. Tag release: `git tag v0.x.0`
4. Push tag: `git push origin v0.x.0`
5. Create GitHub release
6. Publish to crates.io (maintainers only)

## Getting Help

- Open an issue for bugs or questions
- Discussion forum for general questions
- Check existing issues and PRs first

## License

By contributing, you agree that your contributions will be licensed under the MIT License.

## Recognition

Contributors will be added to:
- AUTHORS file
- Release notes
- Project acknowledgments

Thank you for contributing to Actorus!
