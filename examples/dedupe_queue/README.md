# Source Processing with Deduplication Examples

This directory demonstrates source processing with automatic deduplication to prevent duplicate item processing using the new simplified configuration format.

## Files

### config.yaml
Comprehensive deduplication demonstration:
- **Periodic triggers with duplicates**: Script outputs include duplicate items
- **Source field**: Directly executes command and processes output
- **Built-in deduplication**: Optional `dedupe: true` flag prevents reprocessing
- **Unified placeholders**: Uses `${1}` syntax for variable substitution

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
# Run dedupe source processing example
cargo run -- --config examples/dedupe_queue/config.yaml
```

## How It Works

1. **Periodic Generation**: Every 8 seconds, `generate_items.sh` produces items (including duplicates)
2. **Automatic Deduplication**: `dedupe: true` flag filters out previously seen items
3. **Direct Processing**: Only new/unique items are processed immediately
4. **Memory Reset**: Deduplication memory is cleared when the process restarts

## Configuration Details

```yaml
agents:
  triggers:
    # Generate and process items with automatic deduplication
    - name: "process_unique_items"
      event: "timer:8s"
      source: "bash examples/dedupe_queue/generate_items.sh"
      dedupe: true  # Key feature for deduplication
      action: "send_keys"
      keys: ["echo 'Processing unique: ${1}'", "\r", "bash examples/dedupe_queue/process_item.sh ${1}", "\r"]
```

## Comparison: Normal vs Dedupe Processing

| Feature | Normal Processing | Dedupe Processing (`dedupe: true`) |
|---------|-------------------|-----------------------------------|
| Duplicate handling | Processes all items | Skips duplicate items |
| Memory usage | Lower | Higher (stores seen items) |
| Performance | Faster | Slightly slower (dedup check) |
| Configuration | Simple | One additional boolean flag |
| Use case | Simple workflows | Idempotent operations |

## Key Benefits

- **Simplified Configuration**: Single trigger with optional deduplication flag
- **Direct Processing**: No intermediate queue storage needed
- **Unified Syntax**: Consistent `${1}` placeholder usage
- **Automatic Filtering**: Built-in duplicate detection without separate queue concepts
- **Flexible**: Can easily enable/disable deduplication per trigger