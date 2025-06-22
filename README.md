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

### Queue System Configuration
```yaml
# Queue-based task processing with periodic triggers
entries:
  # Generate tasks every 15 seconds
  - name: "generate_tasks"
    trigger: "periodic"
    interval: "15s"
    action: "enqueue"
    queue: "tasks"
    command: "bash examples/simple_queue/list_tasks.sh"

  # Process each task automatically
  - name: "process_tasks"
    trigger: "enqueue:tasks"
    action: "send_keys"
    keys: ["bash examples/simple_queue/process_task.sh <task>", "\r"]
```

### Concurrency Control Configuration
```yaml
# Monitor section with custom base port
monitor:
  base_port: 8080  # Custom port (default: 9990)

# Entries with concurrency limits
entries:
  - name: "heavy_task"
    trigger: "periodic"
    interval: "3s"
    action: "send_keys"
    keys: ["bash heavy_task.sh", "\r"]
    concurrency: 2  # Max 2 simultaneous executions

  - name: "light_task"
    trigger: "periodic"
    interval: "2s"
    action: "send_keys"
    keys: ["bash light_task.sh", "\r"]
    concurrency: 5  # Max 5 simultaneous executions
    
  - name: "critical_task"
    trigger: "periodic"
    interval: "10s"
    action: "send_keys"
    keys: ["bash critical_task.sh", "\r"]
    concurrency: 1  # Single execution only (default)
```

## Core Concepts

### Entries vs Rules

The system distinguishes between two types of automation:

- **Entries**: External triggers initiated by system events (e.g., startup, periodic intervals, queue events)
- **Rules**: Automatic detection triggered by terminal state changes (e.g., prompts, output patterns)

### Trigger Types

**Entry Triggers:**
- `on_start`: Executes when RuleAgents starts
- `periodic`: Executes at regular intervals (e.g., "15s", "5m", "2h")
- `enqueue:queue_name`: Executes when items are added to specified queue

### Action Types

- `send_keys`: Send keyboard input to terminal
- `workflow`: Execute named workflow sequence
- `enqueue`: Add command output to named queue
- `enqueue_dedupe`: Add command output to queue with duplicate filtering

### Configuration Structure

- **entries**: Define external triggers with `trigger`, `action`, and parameters
- **rules**: Define automatic responses with `pattern`, `action`, and `keys`
- **Priority**: Rules are processed in order (first rule = highest priority)
- **Variable Expansion**: Use `<task>` in actions to reference queue items

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

### Queue System

Advanced task processing with automatic queue management:

- **Periodic Triggers**: Execute commands at configurable intervals
- **Queue Processing**: FIFO queues with event-driven item processing
- **Variable Expansion**: Dynamic task substitution using `<task>` placeholders
- **Duplicate Filtering**: Built-in deduplication with `enqueue_dedupe` action
- **Multi-Queue Support**: Handle multiple named queues simultaneously

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
# Start automation with basic example
./target/release/rule-agents --rules examples/basic/config.yaml

# Start with queue system example
./target/release/rule-agents --rules examples/simple_queue/simple_queue.yaml

# Start with dedupe queue example
./target/release/rule-agents --rules examples/dedupe_queue/dedupe_example.yaml

# Start with concurrency control example
./target/release/rule-agents --rules examples/concurrency/concurrency_demo.yaml

# Test rule matching
./target/release/rule-agents test --rules examples/basic/config.yaml --capture "Do you want to proceed"

# View loaded configuration
./target/release/rule-agents show --rules examples/basic/config.yaml
```

## Examples

Multiple example configurations are provided to demonstrate different features:

### 1. Basic Automation (`examples/basic/`)
```bash
./target/release/rule-agents --rules examples/basic/config.yaml
```
- Demonstrates on_start triggers and pattern-based rules
- Automatically executes mock.sh and responds to prompts
- Good starting point for understanding core concepts

### 2. Queue System (`examples/simple_queue/`)
```bash
./target/release/rule-agents --rules examples/simple_queue/simple_queue.yaml
```
- Shows periodic task generation and automatic processing
- Demonstrates queue-based workflows with `<task>` variable expansion
- Useful for batch processing scenarios

### 3. Dedupe Queue (`examples/dedupe_queue/`)
```bash
./target/release/rule-agents --rules examples/dedupe_queue/dedupe_example.yaml
```
- Demonstrates automatic duplicate detection and filtering
- Prevents reprocessing of identical items
- Ideal for idempotent operations

### 4. Concurrency Control (`examples/concurrency/`)
```bash
./target/release/rule-agents --rules examples/concurrency/concurrency_demo.yaml
```
- Demonstrates configurable base port and entry-level concurrency limits
- Shows different concurrency settings for different task types
- Custom web interface port (8080) instead of default 9990
- Prevents system overload with controlled task execution

**Web Interface**: Examples 1-3 provide monitoring at http://localhost:9990, Example 4 uses http://localhost:8080

See [docs/tutorial.md](docs/tutorial.md) for a comprehensive tutorial on configuring and using RuleAgents.