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
entries:
  - name: "start_mock"
    trigger: "on_start"
    action: "send_keys"
    keys: ["bash examples/basic/mock.sh", "\r"]

rules:
  - pattern: "Do you want to proceed"
    action: "send_keys"
    keys: ["1", "\r"]
```

### Claude Command Monitoring
Automatically captures and streams `claude` command output to the web UI:
```bash
$ claude "explain how grep works"
# Output automatically appears in web UI
```

### Agent Pool Configuration
```yaml
web_ui:
  enabled: true
  base_port: 9990

agents:
  concurrency: 2       # Number of parallel agents
  cols: 80
  rows: 24
```

## Core Concepts

**Entries vs Rules:**
- **Entries**: External triggers (startup, periodic, queue events)
- **Rules**: Automatic responses to terminal output patterns

**Trigger Types:**
- `on_start`: Executes at startup
- `periodic`: Executes at intervals ("15s", "5m", "2h")
- `enqueue:queue_name`: Executes when items added to queue

**Actions:**
- `send_keys`: Send keyboard input
- `workflow`: Execute workflow sequence
- `enqueue`: Add output to queue
- `enqueue_dedupe`: Add output to queue with deduplication

## Web Interface

Real-time terminal monitoring at http://localhost:9990:

- Live terminal view with asciinema player
- Real-time Claude command output streaming
- Multi-agent support with separate tabs
- Professional terminal playback controls

## Development

For developers interested in contributing to ccauto, see [CONTRIBUTING.md](CONTRIBUTING.md) for detailed guidelines on:
- Building from source
- Code quality standards
- Testing requirements
- Git workflow with worktrees
- Architecture overview

Detailed development guidelines are also available in [CLAUDE.md](CLAUDE.md).

## Examples

Explore the `examples/` directory for working configurations:

| Example | Description |
|---------|-------------|
| `examples/basic/` | Basic automation with on_start triggers and pattern-based rules |
| `examples/simple_queue/` | Queue system with periodic task generation and processing |
| `examples/web-ui-test/` | Web UI functionality and Claude command monitoring |
| `examples/agent_pool/` | Multiple agents running in parallel with round-robin distribution |

```bash
# Run any example
ccauto --config examples/<example-name>/config.yaml
```

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for detailed guidelines on how to contribute to this project.