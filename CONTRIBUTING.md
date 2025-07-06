# Contributing to ccauto

Welcome to the ccauto project! This document provides guidelines for developers contributing to the project.

## Development Setup

See [CLAUDE.md](CLAUDE.md) for detailed development guidelines including pre-commit hooks, code standards, and workflow requirements.

### Building from Source

```bash
cargo build          # Debug build
cargo test           # Run tests (sets up git hooks)
cargo run -- --help  # Run with help flag
```

### Quality Checks

Pre-commit hooks are automatically set up by `cargo-husky`:

```bash
cargo test                     # All tests must pass
cargo clippy -- -D warnings    # No clippy warnings allowed
cargo fmt                      # Code must be properly formatted
```

**Important**: Never bypass pre-commit hooks with `--no-verify`.

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

For feature development, use git worktrees:

```bash
git worktree add .worktree/issue-<number> -b issue-<number>
cd .worktree/issue-<number>
```

## Architecture

The system consists of several key components:

- **PTY Process**: Native pseudo-terminal implementation for full terminal emulation
- **Agent Pool**: Manages multiple terminal agents for parallel execution
- **Rule Engine**: Pattern matching and action execution system
- **Queue System**: Task queuing and processing with deduplication
- **Web Server**: Built-in HTTP server with WebSocket support for real-time updates

## Development Guidelines

This project follows the guidelines outlined in [CLAUDE.md](CLAUDE.md), including:

- Pre-commit hooks for code quality
- Rust coding conventions
- Testing requirements
- CI/CD integration
- Debugging procedures for web UI and PTY issues

## Getting Help

If you encounter problems with the development workflow:
1. Check [CLAUDE.md](CLAUDE.md) first for detailed guidelines
2. Look at recent successful PRs for examples
3. Ask in the issue tracker