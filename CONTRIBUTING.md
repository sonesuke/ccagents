# Contributing to ccauto

Thank you for your interest in contributing to ccauto! This document provides guidelines for developers who want to contribute to the project.

## Development Setup

### Building from Source

```bash
# Clone the repository
git clone https://github.com/sonesuke/ccauto.git
cd ccauto

# Debug build
cargo build

# Release build
cargo build --release

# Run tests (also sets up git hooks)
cargo test

# Run with help flag
cargo run -- --help
```

### Prerequisites

- Rust 1.70+ (stable toolchain)
- Git
- Basic understanding of Rust and async programming

## Code Quality Standards

### Pre-commit Hooks

This project uses `cargo-husky` to enforce code quality standards. The following checks run automatically on every commit:

1. `cargo test` - All tests must pass
2. `cargo clippy -- -D warnings` - No clippy warnings allowed
3. `cargo fmt -- --check` - Code must be properly formatted

**IMPORTANT: Never bypass pre-commit hooks with `--no-verify`**

If you encounter issues:
- Fix the actual problems instead of bypassing the hooks
- If tests fail, fix the tests or the code
- If clippy warns, address the warnings
- If formatting is wrong, run `cargo fmt` to fix it

### Quality Checks

Before submitting your changes, run these commands:

```bash
# Check all tests pass
cargo test

# Check for clippy warnings
cargo clippy -- -D warnings

# Format your code
cargo fmt

# Run the full CI check locally
cargo test && cargo clippy -- -D warnings && cargo fmt -- --check
```

**Note**: Pre-commit hooks use `cargo fmt -- --check` (non-modifying) to prevent commit-time file modifications. Always run `cargo fmt` manually before committing.

## Test Coverage

The project uses `cargo-llvm-cov` for comprehensive test coverage measurement:

### Coverage Tools Setup

```bash
# Install cargo-llvm-cov (if not already installed)
cargo install cargo-llvm-cov

# Ensure llvm-tools-preview component is installed
rustup component add llvm-tools-preview
```

### Local Coverage Commands

```bash
# Generate coverage report in terminal
cargo llvm-cov

# Generate HTML coverage report
cargo llvm-cov --html --output-dir target/coverage/html

# Generate and open HTML report in browser
cargo llvm-cov --open

# Generate LCOV format for CI/CD integration
cargo llvm-cov --lcov --output-path target/lcov.info

# Generate summary only
cargo llvm-cov --summary-only

# Using Makefile shortcuts
make coverage        # Generate LCOV report
```

### Coverage Requirements

- **Line Coverage**: Minimum 75% target (current: ~23%)
- **Function Coverage**: Minimum 70% target (current: ~25%)
- **Region Coverage**: Target 70% (current: ~15%)

### Coverage Integration

- GitHub Actions automatically generates coverage reports
- LCOV reports are uploaded to Codecov for detailed analysis
- PR comments show coverage changes and impact
- Coverage thresholds are enforced via codecov.yml configuration

### Files Excluded from Coverage

- `tests/` directory (test files themselves)
- `examples/` directory
- `target/` directory
- Generated code and build artifacts

## Development Workflow

### 1. Git Worktrees

For feature development, use git worktrees to keep the main working directory clean:

```bash
git worktree add .worktree/issue-<number> -b issue-<number>
cd .worktree/issue-<number>
```

### 2. Commit Guidelines

- Use clear, descriptive commit messages
- Reference issue numbers when applicable: "Fix #123: Description"
- Keep the first line under 72 characters
- Use imperative mood: "Fix bug" not "Fixed bug"

### 3. Pull Request Process

Before creating a PR:
- Ensure all tests pass: `cargo test`
- Check for clippy warnings: `cargo clippy -- -D warnings`
- Format your code: `cargo fmt`
- Run the full CI check locally

## Code Standards

### Rust Conventions

- Follow standard Rust naming conventions (snake_case for functions/variables, CamelCase for types)
- Use `anyhow::Result` for error handling in application code
- Use `thiserror` for library errors
- Prefer `tokio` for async runtime
- Write comprehensive documentation for public APIs

### Project Structure

- Keep modules focused and single-purpose
- Integration tests go in `tests/`
- Unit tests go in the same file as the code being tested
- Examples go in `examples/`
- Documentation goes in `docs/`

### Dependencies

- Minimize external dependencies
- Pin minor versions in Cargo.toml (e.g., "1.0" not "1")
- Document why each dependency is needed
- Prefer dependencies that are actively maintained

## Testing

### Test Requirements

- All new features must have tests
- All bug fixes must include a regression test
- Integration tests should test actual command-line behavior
- Use `tempfile` for tests that need filesystem access
- Tests should be deterministic and not depend on external services

### Running Tests

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_name

# Run tests in a specific module
cargo test module_name
```

### Test Organization

- **Unit tests**: Test individual functions and modules
- **Integration tests**: Test end-to-end behavior in `tests/`
- **Example tests**: Ensure examples work correctly

## Architecture Overview

The system consists of several key components:

- **PTY Process**: Native pseudo-terminal implementation for full terminal emulation
- **Agent Pool**: Manages multiple terminal agents for parallel execution
- **Rule Engine**: Pattern matching and action execution system
- **Queue System**: Task queuing and processing with deduplication
- **Web Server**: Built-in HTTP server with WebSocket support for real-time updates

For detailed architecture documentation, see `docs/architecture.md`.

## Debugging

### Web UI and PTY Issues

Use Playwright for debugging frontend issues:

```bash
# After navigating to the Web UI
mcp__playwright__browser_console_messages
```

Key console log patterns to look for:
- `Connecting to WebSocket: ws://localhost:9990/ws` - WebSocket initialization
- `WebSocket connected` - Connection established
- `Creating asciinema player` - Terminal player setup
- `Asciinema player created successfully` - Player ready
- `Sending command: <command>` - Command transmission

### Debug Log Analysis

Key log patterns to look for:
- `🎯 RuleAgents started` - Application startup
- `🚀 Web server ready` - Server initialization
- `PTY process started successfully` - Terminal ready
- `WebSocket connection established` - Client connected
- `🔍 send_input called with` - Command execution
- `📤 Sending WebSocket data` - Output transmission

## CI/CD

GitHub Actions runs the same checks as pre-commit hooks. To avoid CI failures:

1. Never bypass pre-commit hooks
2. Always run the full test suite before pushing
3. Check the CI status before merging PRs
4. Address any failing checks promptly

## Common Issues and Solutions

### "Clippy warnings in CI but not locally"
- Ensure you're using the same Rust version as CI (stable)
- Update your toolchain: `rustup update stable`

### "Format check fails in CI"
- Run `cargo fmt` locally before committing
- Ensure your rustfmt version matches CI

### "Tests pass locally but fail in CI"
- Check for race conditions in async tests
- Ensure tests don't depend on local environment
- Use proper test isolation with `tempfile`

## Documentation

For detailed development guidelines and troubleshooting, see [CLAUDE.md](CLAUDE.md).

## Questions or Issues?

If you encounter problems with the development workflow:
1. Check this document first
2. Review [CLAUDE.md](CLAUDE.md) for detailed guidelines
3. Look at recent successful PRs for examples
4. Ask in the issue tracker

We appreciate your contributions to ccauto!