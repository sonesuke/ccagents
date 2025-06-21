# RuleAgents

A command-line tool for YAML-driven agent auto-control system

## Installation

```bash
cargo build --release
```

## Usage

```bash
# Run with default rules.yaml file
./target/release/rule-agents

# Run with specific rules file
./target/release/rule-agents --rules examples/basic-rules.yaml

# Show help
./target/release/rule-agents --help
```

## Configuration

Create a YAML file with your rules. See `examples/basic-rules.yaml` for reference:

```yaml
rules:
  - priority: 10
    pattern: "issue\\s+(\\d+)"
    command: "entry"
    args: []
  - priority: 20
    pattern: "cancel"
    command: "cancel"
    args: []
  - priority: 30
    pattern: "resume"
    command: "resume"
    args: []
```

### Rule Structure

- **priority**: Lower numbers = higher priority (rules are sorted by priority)
- **pattern**: Regular expression to match against input
- **command**: Command to execute when pattern matches (`entry`, `cancel`, `resume`)
- **args**: Optional arguments for the command (defaults to empty array)

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
- **Shell Prompt Recognition**: Configurable regex patterns for various shell prompts
- **Output Change Monitoring**: Compares terminal snapshots to detect activity
- **Timeout Handling**: Detects stuck commands and provides warnings
- **Event-Driven Architecture**: Efficient monitoring with minimal CPU overhead

See [docs/terminal_output_monitoring.md](docs/terminal_output_monitoring.md) for detailed documentation.

### Web Interface

The HT terminal process automatically starts with a web interface for real-time monitoring:

- **Live Terminal View**: Watch agent terminal sessions via web browser
- **Remote Access**: Monitor from any device on the network
- **Multi-user Support**: Multiple people can observe sessions simultaneously
- **Debug Support**: Real-time visibility into agent behavior

#### Access URLs

After starting rule-agents, the terminal web interface is available at:
- **Local**: http://localhost:9999
- **Network**: http://[machine-ip]:9999

The web interface URL is automatically displayed when the HT process starts.

#### Command Line Interface

Access the web interface by running the rule-agents binary:

```bash
# Start with default configuration
./target/release/rule-agents

# Start with custom rules and settings
./target/release/rule-agents daemon --rules examples/basic-rules.yaml --interval 5

# Test rule matching
./target/release/rule-agents test --rules examples/basic-rules.yaml --capture "issue 123"

# View loaded rules
./target/release/rule-agents show --rules examples/basic-rules.yaml
```