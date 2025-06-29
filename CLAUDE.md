# Development Guidelines for RuleAgents

This document contains important development rules and conventions for the RuleAgents project.

## Pre-commit Hooks

This project uses cargo-husky to enforce code quality standards. The following checks run automatically on every commit:

1. `cargo test` - All tests must pass
2. `cargo clippy -- -D warnings` - No clippy warnings allowed
3. `cargo fmt -- --check` - Code must be properly formatted

### IMPORTANT: Never bypass pre-commit hooks

**DO NOT use `git commit --no-verify` or `git commit -n`**

The pre-commit hooks exist to maintain code quality and prevent CI failures. If you encounter issues:
- Fix the actual problems instead of bypassing the hooks
- If tests fail, fix the tests or the code
- If clippy warns, address the warnings
- If formatting is wrong, run `cargo fmt` to fix it

## Development Workflow

### 1. Always use git worktrees for feature development
```bash
git worktree add .worktree/issue-<number> -b issue-<number>
cd .worktree/issue-<number>
```

### 2. Before creating a PR
- Ensure all tests pass: `cargo test`
- Check for clippy warnings: `cargo clippy -- -D warnings`
- Format your code: `cargo fmt`
- Run the full CI check locally: `cargo test && cargo clippy -- -D warnings && cargo fmt -- --check`

**Note**: Pre-commit hooks use `cargo fmt -- --check` (non-modifying) to prevent commit-time file modifications. Always run `cargo fmt` manually before committing to ensure proper formatting.

### 3. Commit message format
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

### Running the Application with Debug Output

1. **Start with debug logging** (foreground):
```bash
cargo run -- --config examples/claude/config.yaml --debug
```

2. **Start in background** (for automated testing/screenshots):
```bash
# Start in background
cargo run -- --config examples/claude/config.yaml --debug &

# Or use nohup to persist after terminal closes
nohup cargo run -- --config examples/claude/config.yaml --debug > debug.log 2>&1 &
```

3. **Stop background process**:
```bash
# Find and kill the process
pkill -f "rule-agents"

# Or find PID and kill specifically
ps aux | grep rule-agents
kill <PID>
```

4. **Check the debug log file**:
```bash
tail -f pattern_match_debug.log
```

5. **Monitor process status**:
```bash
ps aux | grep rule-agents
```

### Web UI Testing

1. **Create test configuration** (use `examples/web-ui-test/config.yaml` for simple testing):
```yaml
entries:
  - name: "test-echo"
    trigger: "on_start"
    action: "send_keys"
    keys: ["echo 'Web UI Test - Hello World!'", "\r"]
```

2. **Access Web UI**:
   - URL: http://localhost:9990
   - Check browser console for WebSocket errors
   - Look for "Connected" status indicator

3. **Verify WebSocket connection**:
   - Look for log messages: `🔌 WebSocket connection established`
   - Check for data flow: `📤 Sending WebSocket data: X bytes`

### Automated Web UI Testing

For automated testing and screenshots, start the server in background:

1. **Start server in background**:
```bash
cargo run -- --config examples/web-ui-test/config.yaml --debug &
```

2. **Wait for server to be ready** (look for "🚀 Web server ready"):
```bash
# Wait a few seconds for startup
sleep 3
```

3. **Test with browser automation** (Playwright/Selenium):
```bash
# Server should be accessible at http://localhost:9990
curl -I http://localhost:9990  # Quick health check
```

4. **Take screenshots or perform automated testing**

5. **Clean up**:
```bash
pkill -f "rule-agents"
```

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

### Common Debugging Steps

1. **PTY Output Issues**:
   - Check for `🔍 PTY content` debug messages
   - Verify shell is starting: `Starting PTY process with config`
   - Look for `PTY process started successfully`

2. **WebSocket Problems**:
   - Verify server binding: `✅ Web server successfully bound`
   - Check connection logs: `WebSocket connection established`
   - Monitor data transmission: `📤 Sending WebSocket data`

3. **Command Execution**:
   - Look for `🔍 send_input called with` messages
   - Check if commands are being processed
   - Verify output is being captured

### Troubleshooting Checklist

**Backend (Server-side)**:
- [ ] Application compiles without errors
- [ ] Web server starts on port 9990
- [ ] PTY process initializes successfully
- [ ] WebSocket connections are established
- [ ] Commands are sent to PTY
- [ ] Output is captured and transmitted
- [ ] Look for `📤 Sending WebSocket data: X bytes` logs

**Frontend (Browser-side)**:
- [ ] Web UI loads without JavaScript errors
- [ ] WebSocket connection established (`WebSocket connected`)
- [ ] Asciinema player created successfully
- [ ] Commands sent (`Sending command: <cmd>`)
- [ ] Data reception logs present (if missing → server issue)
- [ ] Terminal content updates in UI

**Problem Isolation**:
1. **Frontend + WebSocket OK, No terminal output** → PTY output capture issue
2. **Commands not sent** → Frontend JavaScript issue  
3. **WebSocket connection fails** → Network/server binding issue
4. **No WebSocket data reception logs** → Server-side output processing issue

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
2. Look at recent successful PRs for examples
3. Ask in the issue tracker