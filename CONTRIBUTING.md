# Contributing to ccauto

Thank you for your interest in contributing to ccauto! This document provides guidelines for developers working on this project.

## Development Guidelines

For detailed development rules and conventions, see [CLAUDE.md](CLAUDE.md).

## Building from Source

```bash
cargo build          # Debug build
cargo test           # Run tests (sets up git hooks)
cargo run -- --help  # Run with help flag
```

## Quality Checks

Pre-commit hooks are automatically set up by `cargo-husky`:

```bash
cargo test                     # All tests must pass
cargo clippy -- -D warnings    # No clippy warnings allowed
cargo fmt                      # Code must be properly formatted
```

**Important**: Never bypass pre-commit hooks with `--no-verify`.

## Test Coverage

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

## Working with Git Worktrees

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

## Debugging Web UI and PTY Issues

### Browser Console Debugging with Playwright

Use Playwright to check frontend JavaScript logs and errors:

1. **Access console logs**:
```bash
# After navigating to the Web UI
mcp__playwright__browser_console_messages
```

2. **Monitor real-time console output**:
   - Check for WebSocket connection status
   - Verify data transmission logs
   - Look for JavaScript errors

3. **Key console log patterns to look for**:
   - `Connecting to WebSocket: ws://localhost:9990/ws` - WebSocket initialization
   - `WebSocket connected` - Connection established
   - `Creating asciinema player` - Terminal player setup
   - `Asciinema player created successfully` - Player ready
   - `Sending command: <command>` - Command transmission
   - **Missing**: Data reception logs indicate server-side issues

4. **Identify problem areas**:
   - ✅ **Frontend working**: WebSocket connects, commands sent
   - ❌ **Backend issue**: No data reception logs = PTY output not captured/transmitted

5. **Example debugging session**:
```bash
# 1. Start server in background
nohup cargo run -- --config examples/web-ui-test/config.yaml --debug > debug.log 2>&1 &

# 2. Access with Playwright
mcp__playwright__browser_navigate http://localhost:9990

# 3. Wait for connection
mcp__playwright__browser_wait_for "Connected"

# 4. Send test command
mcp__playwright__browser_type "echo test" (submit)

# 5. Check console logs
mcp__playwright__browser_console_messages

# 6. Analyze: If no data reception logs → server-side PTY issue
```

### Debug Log Analysis

Key log patterns to look for:
- `🎯 RuleAgents started` - Application startup
- `🚀 Web server ready` - Server initialization
- `PTY process started successfully` - Terminal ready
- `WebSocket connection established` - Client connected
- `🔍 send_input called with` - Command execution
- `📤 Sending WebSocket data` - Output transmission

## Questions or Issues?

If you encounter problems with the development workflow:
1. Check this document first
2. Look at [CLAUDE.md](CLAUDE.md) for detailed guidelines
3. Look at recent successful PRs for examples
4. Ask in the issue tracker