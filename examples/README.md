# ccauto Examples

This directory contains example configurations and scripts demonstrating various ccauto features.

## Available Examples

### [Basic Examples](./basic/)
Basic terminal automation and rule-based processing.
- **mock.sh**: Original mock script for testing
- **config.yaml**: Simple rule-based automation

```bash
# Run basic examples
cargo run -- --config examples/basic/config.yaml
```

### [Simple Queue System](./simple_queue/)
Periodic task execution and queue-based processing.
- **config.yaml**: Basic queue operations
- Periodic triggers and automatic task processing

```bash
# Run queue examples
cargo run -- --config examples/simple_queue/config.yaml
```

### [Dedupe Queue System](./dedupe_queue/)
Queue processing with automatic deduplication.
- **config.yaml**: Prevents duplicate item processing
- In-memory deduplication with automatic filtering

```bash
# Run dedupe queue examples
cargo run -- --config examples/dedupe_queue/config.yaml
```

### [Agent Pool System](./agent_pool/)
Multiple agents running in parallel for improved performance.
- **config.yaml**: 2 agents running tasks in parallel
- Round-robin task distribution across agents

```bash
# Run agent pool examples
cargo run -- --config examples/agent_pool/config.yaml
```

### [Web UI Configuration](./web-ui/)
Web-based terminal interface configuration examples.
- **config.yaml**: Multi-agent web UI setup
- Real-time terminal display with AVT color support
- Configurable terminal dimensions

```bash
# Run with web UI enabled
cargo run -- --config examples/web-ui/config.yaml
# Access: http://localhost:9990, http://localhost:9991
```

## Quick Start

```bash
# Default basic example
cargo run

# With specific config
cargo run -- --config examples/[folder]/config.yaml
```

## Core Features Demonstrated

- **Agent Pool**: Multiple agents running tasks in parallel
- **Periodic Triggers**: Execute commands at intervals (`10s`, `5m`, `2h`)
- **Queue Processing**: Automatic item processing when enqueued
- **Deduplication**: Prevent duplicate processing with `enqueue_dedupe`
- **Variable Expansion**: Use `<task>` placeholders in actions
- **Rule Matching**: Pattern-based automation with regex support

## Web Interface Access

- **Basic, Simple Queue, Dedupe Queue**: http://localhost:9990
- **Agent Pool**: Multiple tabs starting from http://localhost:9990

Each agent in a pool gets its own web interface URL for independent monitoring.