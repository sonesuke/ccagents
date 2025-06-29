# Queue System Examples

This directory demonstrates the queue system for periodic task execution and automatic processing.

## Files

### config.yaml
Minimal queue system demonstration:
- **Periodic triggers**: Execute commands at regular intervals
- **Enqueue actions**: Add command output to named queues
- **Queue listeners**: Process items when they're added to queues
- **Variable expansion**: Use `<task>` placeholders in actions

### list_tasks.sh
Script that generates sample tasks:
```bash
#!/bin/bash
echo "task-001"
echo "task-002"
echo "task-003"
```

### process_task.sh
Script that simulates task processing:
```bash
#!/bin/bash
task_id=${1:-"unknown"}
echo "Processing task: $task_id"
echo "Status: completed"
echo "Result: success"
```

## Usage

```bash
# Run queue example
cargo run -- --config examples/simple_queue/config.yaml
```

## How It Works

1. **Periodic Generation**: Every 15 seconds, `list_tasks.sh` is executed
2. **Enqueue**: Command output is added line-by-line to the "tasks" queue
3. **Auto-Processing**: Each queued task triggers automatic processing
4. **Variable Replacement**: `<task>` is replaced with actual task IDs

## Configuration Details

```yaml
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

## Key Features

- **Automatic Workflow**: Tasks are generated and processed without manual intervention
- **Queue-Based**: Reliable task queuing with FIFO processing
- **Variable Substitution**: Dynamic command generation with placeholder replacement
- **Configurable Intervals**: Support for various time formats (`10s`, `5m`, `2h`)