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
# Clone and build
git clone https://github.com/sonesuke/ccauto.git
cd ccauto
cargo build --release
cargo install --path .

# Or install directly from git
cargo install --git https://github.com/sonesuke/ccauto.git
```

## Quick Start

```bash
ccauto                                    # Default config
ccauto --config custom-config.yaml       # Custom config
ccauto --debug                           # Debug logging
# View at http://localhost:9990
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
Automatically captures and streams `claude` command outputs to the web UI in real-time.

```bash
$ claude "explain how grep works"
# Output streamed to web UI automatically
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

**Entries vs Rules:**
- **Entries**: External triggers (startup, periodic, queue events)
- **Rules**: Automatic responses to terminal state changes

**Trigger Types:** `on_start`, `periodic`, `enqueue:queue_name`

**Action Types:** `send_keys`, `workflow`, `enqueue`, `enqueue_dedupe`

## Web Interface

Real-time terminal monitoring with live view, Claude output streaming, and asciinema player.

**Access URLs:**
- Single Agent: http://localhost:9990
- Agent Pool: Multiple ports (9990, 9991, etc.)


## Examples

Example configurations in `examples/` directory:

- **Basic Automation**: `examples/basic/` - Startup triggers and pattern-based rules
- **Queue System**: `examples/simple_queue/` - Periodic tasks and queue workflows
- **Web UI Test**: `examples/web-ui-test/` - Web interface and Claude monitoring
- **Agent Pool**: `examples/agent_pool/` - Multiple parallel agents

```bash
ccauto --config examples/basic/config.yaml
```


## License

[Add your license information here]

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for developer guidelines, including building from source, testing, workflows, and architecture details.