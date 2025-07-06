# Simplified Source Processing Examples

This directory demonstrates the simplified source processing system for periodic task execution and automatic processing.

## Files

### config.yaml
Minimal source processing demonstration:
- **Periodic triggers**: Execute source commands at regular intervals
- **Direct processing**: Process each line from source command output immediately
- **Variable expansion**: Use `${1}` placeholders in actions
- **Unified syntax**: No need for separate queue concepts

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
# Run source processing example
cargo run -- --config examples/simple_queue/config.yaml
```

## How It Works

1. **Periodic Generation**: Every 15 seconds, `list_tasks.sh` is executed
2. **Direct Processing**: Each line from command output is processed immediately
3. **Variable Replacement**: `${1}` is replaced with the actual line content

## Configuration Details

```yaml
agents:
  triggers:
    # Generate and process tasks every 15 seconds
    - name: "process_tasks"
      event: "timer:15s"
      source: "bash examples/simple_queue/list_tasks.sh"
      action: "send_keys"
      keys: ["bash examples/simple_queue/process_task.sh ${1}", "\r"]
```

## Key Features

- **Simplified Configuration**: Single trigger handles both generation and processing
- **Direct Processing**: No intermediate queue storage needed
- **Unified Placeholders**: Consistent `${1}` syntax throughout
- **Clearer Flow**: Direct relationship between source and action