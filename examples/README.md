# RuleAgents Examples

This directory contains example configurations and scripts demonstrating various RuleAgents features.

## Available Examples

### [Basic Examples](./basic/)
Basic terminal automation and rule-based processing.
- **mock.sh**: Original mock script for testing
- **config.yaml**: Simple rule-based automation

```bash
# Run basic examples
cargo run -- --rules examples/basic/config.yaml
```

### [Simple Queue System](./simple_queue/)
Periodic task execution and queue-based processing.
- **simple_queue.yaml**: Basic queue operations
- Periodic triggers and automatic task processing

```bash
# Run queue examples
cargo run -- --rules examples/simple_queue/simple_queue.yaml
```

### [Dedupe Queue System](./dedupe_queue/)
Queue processing with automatic deduplication.
- **dedupe_example.yaml**: Prevents duplicate item processing
- In-memory deduplication with automatic filtering

```bash
# Run dedupe queue examples
cargo run -- --rules examples/dedupe_queue/dedupe_example.yaml
```

### [Agent Pool System](./agent_pool/)
Multiple agents running in parallel for improved performance.
- **concurrency_demo.yaml**: 2 agents running tasks in parallel
- Round-robin task distribution across agents

```bash
# Run agent pool examples
cargo run -- --rules examples/agent_pool/concurrency_demo.yaml
```

## Quick Start

```bash
# Default basic example
cargo run

# With specific config
cargo run -- --rules examples/[folder]/[config].yaml
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