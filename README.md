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

#### Quick Example

```rust
use rule_agents::{HtClient, HtProcess, HtProcessConfig, TerminalOutputMonitor};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup HT client
    let ht_process = HtProcess::new(HtProcessConfig::default());
    let ht_client = Arc::new(HtClient::new(ht_process));
    ht_client.start().await?;

    // Create and start monitor
    let mut monitor = TerminalOutputMonitor::new(ht_client.clone()).await?;
    let mut state_receiver = monitor.start_monitoring();
    
    // Handle state transitions
    while let Some(transition) = state_receiver.recv().await {
        println!("Agent state: {} -> {}", 
            transition.from, transition.to);
    }
    
    Ok(())
}
```