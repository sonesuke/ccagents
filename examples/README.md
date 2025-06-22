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

## Quick Start

```bash
# Default basic example
cargo run

# With specific config
cargo run -- --rules examples/[folder]/[config].yaml
```

## Core Features Demonstrated

- **Periodic Triggers**: Execute commands at intervals (`10s`, `5m`, `2h`)
- **Queue Processing**: Automatic item processing when enqueued
- **Deduplication**: Prevent duplicate processing with `enqueue_dedupe`
- **Variable Expansion**: Use `<task>` placeholders in actions
- **Rule Matching**: Pattern-based automation with regex support