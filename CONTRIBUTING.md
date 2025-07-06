# Contributing to ccauto

Thank you for your interest in contributing to ccauto! This guide provides detailed information for developers who want to build, test, and contribute to the project.

## Development Setup

### Building from Source

```bash
# Clone the repository
git clone https://github.com/sonesuke/ccauto.git
cd ccauto

# Build the project
cargo build          # Debug build
cargo build --release  # Release build

# Run tests (sets up git hooks)
cargo test           

# Run with help flag
cargo run -- --help  
```

### Prerequisites

- Rust 1.70 or later
- Git
- For coverage reporting: `cargo-llvm-cov` and `llvm-tools-preview`

```bash
# Install coverage tools
cargo install cargo-llvm-cov
rustup component add llvm-tools-preview
```

## Code Quality Standards

This project uses `cargo-husky` to enforce code quality standards. The following checks run automatically on every commit:

1. `cargo test` - All tests must pass
2. `cargo clippy -- -D warnings` - No clippy warnings allowed
3. `cargo fmt -- --check` - Code must be properly formatted

### Quality Check Commands

```bash
cargo test                     # All tests must pass
cargo clippy -- -D warnings    # No clippy warnings allowed
cargo fmt                      # Code must be properly formatted
```

### Important: Never Bypass Pre-commit Hooks

**DO NOT use `git commit --no-verify` or `git commit -n`**

The pre-commit hooks exist to maintain code quality and prevent CI failures. If you encounter issues:
- Fix the actual problems instead of bypassing the hooks
- If tests fail, fix the tests or the code
- If clippy warns, address the warnings
- If formatting is wrong, run `cargo fmt` to fix it

## Test Coverage

The project uses `cargo-llvm-cov` for comprehensive test coverage measurement:

### Coverage Commands

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

### Coverage Targets

- **Line Coverage**: Minimum 75% (currently 23.34%)
- **Function Coverage**: Minimum 70% (currently 25.47%)
- **Region Coverage**: Target 70% (currently 15.13%)

Coverage reports are automatically generated in CI/CD and uploaded to [Codecov](https://codecov.io).

## Development Workflow

### Git Worktree Workflow

For feature development, use git worktrees to keep your main working directory clean:

```bash
# Create a new worktree for issue development
git worktree add .worktree/issue-<number> -b issue-<number>
cd .worktree/issue-<number>

# Work on your feature...

# When done, clean up
cd ../..
git worktree remove .worktree/issue-<number>
```

### Before Creating a Pull Request

1. Ensure all tests pass: `cargo test`
2. Check for clippy warnings: `cargo clippy -- -D warnings`
3. Format your code: `cargo fmt`
4. Run the full CI check locally: `cargo test && cargo clippy -- -D warnings && cargo fmt -- --check`

**Note**: Pre-commit hooks use `cargo fmt -- --check` (non-modifying) to prevent commit-time file modifications. Always run `cargo fmt` manually before committing to ensure proper formatting.

### Commit Message Format

- Use clear, descriptive commit messages
- Reference issue numbers when applicable: "Fix #123: Description"
- Keep the first line under 72 characters

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

## Architecture

The system consists of several key components:

- **PTY Process**: Native pseudo-terminal implementation for full terminal emulation
- **Agent Pool**: Manages multiple terminal agents for parallel execution
- **Rule Engine**: Pattern matching and action execution system
- **Queue System**: Task queuing and processing with deduplication
- **Web Server**: Built-in HTTP server with WebSocket support for real-time updates

## CI/CD

GitHub Actions runs the same checks as pre-commit hooks. To avoid CI failures:

1. Never bypass pre-commit hooks
2. Always run the full test suite before pushing
3. Check the CI status before merging PRs

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

## Detailed Development Guidelines

For comprehensive development guidelines, coding standards, and project-specific rules, see [CLAUDE.md](CLAUDE.md).

## Questions or Issues?

If you encounter problems with the development workflow:

1. Check this document first
2. Review [CLAUDE.md](CLAUDE.md) for detailed guidelines
3. Look at recent successful PRs for examples
4. Ask in the issue tracker

Thank you for contributing to ccauto!