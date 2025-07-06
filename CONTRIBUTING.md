# Contributing to ccauto

Thank you for your interest in contributing to ccauto! This document provides guidelines for developers working on the project.

## Development Environment Setup

### Prerequisites

- Rust 1.70+ (latest stable recommended)
- Git
- A terminal emulator

### Building from Source

```bash
# Clone the repository
git clone https://github.com/sonesuke/ccauto.git
cd ccauto

# Build the project
cargo build          # Debug build
cargo build --release # Release build

# Run tests (sets up git hooks)
cargo test

# Run with help flag
cargo run -- --help
```

## Development Guidelines

For comprehensive development guidelines, see [CLAUDE.md](CLAUDE.md), which contains:
- Pre-commit hooks and quality standards
- Git workflow with worktrees
- Code standards and conventions
- Testing requirements
- CI/CD integration

### Quality Checks

Pre-commit hooks are automatically set up by `cargo-husky` when you run `cargo test`:

```bash
cargo test                     # All tests must pass
cargo clippy -- -D warnings    # No clippy warnings allowed
cargo fmt                      # Code must be properly formatted
```

**Important**: Never bypass pre-commit hooks with `--no-verify`. The hooks exist to maintain code quality and prevent CI failures.

### Test Coverage

The project uses `cargo-llvm-cov` for comprehensive test coverage measurement:

```bash
# Generate coverage report in terminal
cargo llvm-cov

# Generate HTML coverage report
cargo llvm-cov --html --output-dir target/coverage/html

# Generate and open HTML report in browser
cargo llvm-cov --open

# Generate LCOV format for CI/CD integration
cargo llvm-cov --lcov --output-path target/lcov.info

# Using Makefile shortcuts
make coverage        # Generate LCOV report
```

**Coverage Targets:**
- **Line Coverage**: Minimum 75% (currently 23.34%)
- **Function Coverage**: Minimum 70% (currently 25.47%)
- **Region Coverage**: Target 70% (currently 15.13%)

Coverage reports are automatically generated in CI/CD and uploaded to [Codecov](https://codecov.io).

### Working with Git Worktrees

For feature development, use git worktrees to keep your main working directory clean:

```bash
git worktree add .worktree/issue-<number> -b issue-<number>
cd .worktree/issue-<number>
# Work on your feature
# When done:
cd ../..
git worktree remove .worktree/issue-<number>
```

## Architecture

The system consists of several key components:

- **PTY Process**: Native pseudo-terminal implementation for full terminal emulation
- **Agent Pool**: Manages multiple terminal agents for parallel execution
- **Rule Engine**: Pattern matching and action execution system
- **Queue System**: Task queuing and processing with deduplication
- **Web Server**: Built-in HTTP server with WebSocket support for real-time updates

## Code Standards

### Rust Conventions
- Follow standard Rust naming conventions (snake_case for functions/variables, CamelCase for types)
- Use `anyhow::Result` for error handling in application code
- Use `thiserror` for library errors
- Prefer `tokio` for async runtime

### Project Structure
- Keep modules focused and single-purpose
- Integration tests go in `tests/`
- Unit tests go in the same file as the code being tested
- Examples go in `examples/`

### Dependencies
- Minimize external dependencies
- Pin minor versions in Cargo.toml (e.g., "1.0" not "1")
- Document why each dependency is needed

## Testing

### Test Requirements
- All new features must have tests
- All bug fixes must include a regression test
- Integration tests should test actual command-line behavior
- Use `tempfile` for tests that need filesystem access

### Running Tests
```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_name
```

## Debugging

### Web UI and PTY Issues

For detailed debugging instructions, see [CLAUDE.md](CLAUDE.md#debugging-web-ui-and-pty-issues), which includes:
- Browser console debugging with Playwright
- Log analysis patterns
- Common troubleshooting steps

## Pull Request Guidelines

1. **Branch Naming**: Use descriptive branch names, preferably `issue-<number>` format
2. **Commit Messages**: Use clear, descriptive commit messages
3. **Testing**: Ensure all tests pass before submitting
4. **Code Quality**: Run `cargo fmt` and `cargo clippy` before committing
5. **Documentation**: Update documentation if your changes affect user-facing features

## Getting Help

If you encounter problems with the development workflow:
1. Check [CLAUDE.md](CLAUDE.md) for detailed guidelines
2. Look at recent successful PRs for examples
3. Ask in the issue tracker

## License

By contributing to ccauto, you agree that your contributions will be licensed under the same license as the project.