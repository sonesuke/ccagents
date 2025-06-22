# RuleAgents

A command-line tool for YAML-driven agent auto-control system with automatic terminal interaction.

## Prerequisites

### HT (Headless Terminal)

RuleAgents requires HT (Headless Terminal) to be installed on your system. HT provides the terminal emulation and web interface that RuleAgents uses for automation.

#### Install HT

```bash
# Clone and install HT
git clone https://github.com/andyk/ht.git
cd ht
cargo install --path .
```

Alternatively, if you have cargo installed:
```bash
cargo install ht
```

Verify HT is installed and accessible:
```bash
ht --version
```

## Installation

```bash
cargo build --release
```

## Quick Start

```bash
# Run with default configuration (starts automatically)
./target/release/rule-agents

# Run with specific config file
./target/release/rule-agents --rules custom-config.yaml

# View terminal automation at http://localhost:9990
```

## Configuration

Create a YAML file with entries and rules. See `config.yaml` for reference:

```yaml
# External triggers - initiated by system events
entries:
  - name: "start_mock"
    trigger: "on_start"           # Automatic startup trigger
    action: "send_keys"
    keys: ["bash examples/mock.sh", "\r"]

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

## Core Concepts

### Entries vs Rules

The system distinguishes between two types of automation:

- **Entries**: External triggers initiated by system events (e.g., startup, user commands)
- **Rules**: Automatic detection triggered by terminal state changes (e.g., prompts, output patterns)

### Configuration Structure

- **entries**: Define external triggers with `trigger`, `action`, and `keys`
- **rules**: Define automatic responses with `pattern`, `action`, and `keys`
- **Priority**: Rules are processed in order (first rule = highest priority)

## Development

### Building from Source

```bash
cargo build          # Debug build
cargo test           # Run tests (also sets up git hooks)
cargo run -- --help  # Run with help flag
```

### Quality Checks

Git hooks are automatically set up by `cargo-husky` when you first run `cargo test`:

```bash
cargo check                    # Check compilation
cargo test                     # Run tests
cargo clippy -- -D warnings    # Lint checks
cargo fmt                      # Auto-format code
```

### Architecture

See [docs/architecture.md](docs/architecture.md) for system design details.

## Features

### Terminal Output Monitoring

Real-time agent state detection based on HT (terminal) process output analysis:

- **State Detection**: Automatically detects Idle, Wait, and Active agent states
- **Shell Prompt Recognition**: Built-in regex patterns for various shell prompts
- **Output Change Monitoring**: Compares terminal snapshots to detect activity
- **Timeout Handling**: Detects stuck commands and provides warnings
- **Event-Driven Architecture**: Efficient monitoring with minimal CPU overhead

### Web Interface

The HT terminal process automatically starts with a web interface for real-time monitoring:

- **Live Terminal View**: Watch agent terminal sessions via web browser
- **Automatic Execution**: View real-time automation triggered by entries and rules
- **Multi-user Support**: Multiple people can observe sessions simultaneously
- **Debug Support**: Real-time visibility into agent behavior

#### Access URLs

After starting rule-agents, the terminal web interface is available at:
- **Local**: http://localhost:9990
- **Network**: http://[machine-ip]:9990

The web interface URL is automatically displayed when the HT process starts.

## Available Commands

```bash
# Start automation with default config.yaml
./target/release/rule-agents

# Start with custom config file
./target/release/rule-agents --rules custom-config.yaml

# Test rule matching
./target/release/rule-agents test --rules config.yaml --capture "Do you want to proceed"

# View loaded configuration
./target/release/rule-agents show --rules config.yaml
```

## Mock Test Example

A complete test scenario is provided to demonstrate the system:

1. **Run the test**:
   ```bash
   chmod +x examples/mock.sh
   ./target/release/rule-agents --rules config.yaml
   ```

2. **Automatic execution**:
   - Opens http://localhost:9990 to view the terminal
   - Automatically starts `examples/mock.sh` via `on_start` trigger
   - Automatically responds to prompts using defined rules
   - Completes the full workflow without manual intervention

See [docs/mock-test-guide.md](docs/mock-test-guide.md) for a comprehensive tutorial on configuring and using RuleAgents.