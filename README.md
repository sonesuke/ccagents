# ccauto

[![codecov](https://codecov.io/github/sonesuke/ccauto/graph/badge.svg?token=4YGL6PWD3C)](https://codecov.io/github/sonesuke/ccauto)

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


## Examples

Multiple example configurations demonstrate different features:

| Example | Command | Description |
|---------|---------|-------------|
| Basic Automation | `ccauto --config examples/basic/config.yaml` | On_start triggers and pattern-based rules |
| Queue System | `ccauto --config examples/simple_queue/config.yaml` | Periodic task generation and queue workflows |
| Web UI Test | `ccauto --config examples/web-ui-test/config.yaml` | Web UI functionality and Claude monitoring |
| Agent Pool | `ccauto --config examples/agent_pool/config.yaml` | Multiple parallel agents with round-robin distribution |

## Contributing

Interested in contributing to ccauto? We welcome contributions from developers of all skill levels!

For detailed development guidelines, build instructions, testing procedures, and code standards, please see [CONTRIBUTING.md](CONTRIBUTING.md).

For project-specific development rules and conventions, see [CLAUDE.md](CLAUDE.md).

## License

[Add your license information here]