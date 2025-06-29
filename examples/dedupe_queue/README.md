# Dedupe Queue Examples

This directory demonstrates queue processing with automatic deduplication to prevent duplicate item processing.

## Files

### dedupe_example.yaml
Comprehensive deduplication demonstration:
- **Periodic triggers with duplicates**: Script outputs include duplicate items
- **Enqueue dedupe actions**: Only unique items are added to queues
- **In-memory deduplication**: Prevents reprocessing of seen items
- **Automatic queue processing**: Unique items are processed automatically

### generate_items.sh
Script that intentionally generates duplicate items:
```bash
#!/bin/bash
echo "item-001"
echo "item-002"
echo "item-003"
echo "item-001"  # duplicate
echo "item-004"
echo "item-002"  # duplicate
echo "item-005"
```

### process_item.sh
Script that processes unique items:
```bash
#!/bin/bash
item_id=${1:-"unknown"}
echo "Processing unique item: $item_id"
echo "Status: processed"
echo "Result: success"
```

## Usage

```bash
# Run dedupe queue example
cargo run -- --config examples/dedupe_queue/config.yaml
```

## How It Works

1. **Periodic Generation**: Every 8 seconds, `generate_items.sh` produces items (including duplicates)
2. **Deduplication**: `enqueue_dedupe` automatically filters out previously seen items
3. **Unique Processing**: Only new/unique items trigger processing actions
4. **Memory Reset**: Deduplication memory is cleared when the process restarts

## Configuration Details

```yaml
entries:
  # Generate items with automatic deduplication
  - name: "item_generator"
    trigger: "periodic"
    interval: "8s"
    action: "enqueue_dedupe"  # Key difference from normal queue
    queue: "pending_items"
    command: "bash examples/dedupe_queue/generate_items.sh"

  # Process each unique item
  - name: "item_processor"
    trigger: "enqueue:pending_items"
    action: "send_keys"
    keys: ["bash examples/dedupe_queue/process_item.sh <task>", "\r"]
```

## Comparison: Normal vs Dedupe

| Feature | Normal Queue (`enqueue`) | Dedupe Queue (`enqueue_dedupe`) |
|---------|--------------------------|--------------------------------|
| Duplicate handling | Processes all items | Skips duplicate items |
| Memory usage | Lower | Higher (stores seen items) |
| Performance | Faster | Slightly slower (dedup check) |
| Use case | Simple workflows | Idempotent operations |

## Key Benefits

- **Idempotent Operations**: Ensures each unique item is processed only once
- **Automatic Filtering**: No manual duplicate detection needed
- **Configurable**: Can mix normal and dedupe queues in same configuration
- **Memory Efficient**: In-memory storage, no persistent state required