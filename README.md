# ccauto

A command-line tool for YAML-driven agent auto-control system with automatic terminal interaction and Claude AI integration.

## Features

- **Claude AI Integration**: Automatically captures and monitors output from `claude` commands
- **Terminal Automation**: Pattern-based automatic responses to terminal prompts
- **Agent Pool System**: Multiple parallel terminal agents for improved performance
- **Queue System**: Advanced task processing with FIFO queues and deduplication
- **Web UI**: Real-time terminal monitoring via built-in web interface
- **PTY Support**: Native pseudo-terminal support for full terminal emulation

## Installation

```bash
# Clone the repository
git clone https://github.com/sonesuke/ccauto.git
cd ccauto

# Build and install
cargo build --release
cargo install --path .

# Or install directly from git
cargo install --git https://github.com/sonesuke/ccauto.git
```

## Quick Start

```bash
# Run with default configuration
ccauto

# Run with specific config file
ccauto --config custom-config.yaml

# Enable debug logging
ccauto --debug

# View terminal automation at http://localhost:9990
```

## Configuration

Create a YAML file with entries and rules. See `examples/` directory for reference configurations:

### Basic Configuration
```yaml
# External triggers - initiated by system events
entries:
  - name: "start_mock"
    trigger: "on_start"           # Automatic startup trigger
    action: "send_keys"
    keys: ["bash examples/basic/mock.sh", "\r"]

# Automatic detection rules - triggered by terminal state changes
# Higher priority = earlier in the list (line order matters)
rules:
  - pattern: "Do you want to proceed"    # Highest priority
    action: "send_keys"
    keys: ["1", "\r"]
    
  - pattern: "^exit$"                    # Lower priority
    action: "send_keys"
    keys: ["/exit", "\r"]
```

### Claude Command Monitoring
When you run a `claude` command in the terminal, ccauto automatically:
1. Detects the command execution
2. Spawns a separate process to capture stdout/stderr
3. Streams the output to the web UI in real-time

Example:
```bash
# In the terminal managed by ccauto
$ claude "explain how grep works"
# Output is automatically captured and displayed in the web UI
```

### Agent Pool Configuration
```yaml
# Web UI configuration
web_ui:
  enabled: true
  host: "localhost"
  base_port: 9990      # First agent port (default: 9990)

# Agent configuration
agents:
  concurrency: 2       # Number of parallel agents (default: 1)
  cols: 80             # Terminal width
  rows: 24             # Terminal height
```

## Core Concepts

### Entries vs Rules

The system distinguishes between two types of automation:

- **Entries**: External triggers initiated by system events (e.g., startup, periodic intervals, queue events)
- **Rules**: Automatic detection triggered by terminal state changes (e.g., prompts, output patterns)

### Trigger Types

**Entry Triggers:**
- `on_start`: Executes when ccauto starts
- `periodic`: Executes at regular intervals (e.g., "15s", "5m", "2h")
- `enqueue:queue_name`: Executes when items are added to specified queue

### Action Types

- `send_keys`: Send keyboard input to terminal
- `workflow`: Execute named workflow sequence
- `enqueue`: Add command output to named queue
- `enqueue_dedupe`: Add command output to queue with duplicate filtering

## Web Interface

The built-in web interface provides real-time terminal monitoring:

- **Live Terminal View**: Watch agent terminal sessions via web browser
- **Claude Output Streaming**: Real-time display of Claude command outputs
- **Asciinema Player**: Professional terminal playback with controls
- **Multi-Agent Support**: Separate tabs for each agent in the pool

### Access URLs

- **Single Agent**: http://localhost:9990
- **Agent Pool**: Multiple ports (e.g., http://localhost:9990, http://localhost:9991, etc.)

## Development

See [CLAUDE.md](CLAUDE.md) for detailed development guidelines.

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

### Working with Git Worktrees

For feature development, use git worktrees:

```bash
git worktree add .worktree/issue-<number> -b issue-<number>
cd .worktree/issue-<number>
```

## Examples

Multiple example configurations demonstrate different features:

### 1. Basic Automation (`examples/basic/`)
```bash
ccauto --config examples/basic/config.yaml
```
- Demonstrates on_start triggers and pattern-based rules
- Automatically executes mock.sh and responds to prompts

### 2. Queue System (`examples/simple_queue/`)
```bash
ccauto --config examples/simple_queue/config.yaml
```
- Shows periodic task generation and automatic processing
- Demonstrates queue-based workflows with `<task>` variable expansion

### 3. Web UI Test (`examples/web-ui-test/`)
```bash
ccauto --config examples/web-ui-test/config.yaml
```
- Tests the web UI functionality
- Demonstrates Claude command monitoring

### 4. Agent Pool (`examples/agent_pool/`)
```bash
ccauto --config examples/agent_pool/config.yaml
```
- Multiple agents running in parallel
- Round-robin task distribution
- Multiple web interface tabs

## Architecture

The system consists of several key components:

- **PTY Process**: Native pseudo-terminal implementation for full terminal emulation
- **Agent Pool**: Manages multiple terminal agents for parallel execution
- **Rule Engine**: Pattern matching and action execution system
- **Queue System**: Task queuing and processing with deduplication
- **Web Server**: Built-in HTTP server with WebSocket support for real-time updates

## License

[Add your license information here]

## Contributing

[Add contribution guidelines here]