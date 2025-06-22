# Concurrency Control Example

This directory demonstrates entry-level concurrency control with a simple, focused example.

## What is Entry Concurrency?

**Entry concurrency controls how many instances of the SAME entry can run simultaneously.**

- `concurrency: 1` (default) = Only one instance at a time
- `concurrency: 2` = Up to 2 instances of the same entry can run concurrently  
- `concurrency: 5` = Up to 5 instances of the same entry can run concurrently

**Important**: This does NOT create multiple agents. There is still only 1 agent, but it can execute the same entry multiple times in parallel.

## Files

### concurrency_demo.yaml
Minimal configuration with:
- **1 periodic entry** that runs every 3 seconds with `concurrency: 2`
- **Custom base port**: 8080 instead of default 9990
- **Rule**: Reacts to task completion messages

### heavy_task.sh
The demo script that:
- Takes 5-10 seconds to complete (simulates slow processing)
- Prints start/completion messages with unique task IDs
- Shows how multiple instances can overlap

## Usage

```bash
# Run the concurrency demo
cargo run -- --rules examples/concurrency/concurrency_demo.yaml

# Watch at http://localhost:8080 (custom port instead of default 9990)
```

## Expected Behavior

### Timeline Example

```
Time 0s:  ✅ Task 1 starts (1/2 slots used)
Time 3s:  ✅ Task 2 starts (2/2 slots used) 
Time 6s:  ⏳ Task 3 waits (slots full)
Time 8s:  ✅ Task 1 completes, Task 3 starts (2/2 slots used)
Time 9s:  ⏳ Task 4 waits (slots full)
Time 11s: ✅ Task 2 completes, Task 4 starts (2/2 slots used)
```

### What You'll See

1. **Every 3 seconds**: A new `demo_task` is triggered
2. **Maximum 2 running**: Even though tasks trigger every 3s but take 5-10s
3. **Queuing behavior**: 3rd task waits until one of the first 2 completes
4. **Task IDs**: Each task shows unique ID like `heavy-12345-1234567890`

## Configuration Details

```yaml
monitor:
  base_port: 8080    # Custom port

entries:
  - name: "demo_task"
    trigger: "periodic"
    interval: "3s"     # Triggers every 3 seconds  
    concurrency: 2     # Max 2 instances running simultaneously
    action: "send_keys"
    keys: ["bash examples/concurrency/heavy_task.sh", "\r"]
```

## Key Benefits

### System Protection
- **Resource management**: Prevents overwhelming the system with too many concurrent heavy tasks
- **Controlled execution**: Different limits for different task types based on their resource requirements

### Performance Optimization  
- **Higher throughput**: Lightweight tasks can run with higher concurrency
- **Better responsiveness**: System remains responsive even under load

### Safety
- **Critical sections**: Single concurrency for operations requiring exclusive access
- **Predictable behavior**: Controlled execution prevents race conditions

## Configuration Options

### Base Port Configuration
```yaml
monitor:
  base_port: 8080  # Any available port (default: 9990)
```

### Entry Concurrency
```yaml
entries:
  - name: "task_name"
    concurrency: 3   # Max simultaneous executions (default: 1)
    # ... other entry fields
```

### Backward Compatibility
- **Optional fields**: Both `monitor` section and `concurrency` field are optional
- **Default values**: Uses sensible defaults when fields are omitted
- **Existing configs**: All existing configurations continue to work unchanged

## Testing the Example

1. **Start the demo**:
   ```bash
   cargo run -- --rules examples/concurrency/concurrency_demo.yaml
   ```

2. **Monitor execution**:
   - Open http://localhost:8080 in your browser
   - Watch how different tasks respect their concurrency limits
   - Observe timing and execution patterns

3. **Expected behavior**:
   - Heavy tasks: Max 2 running simultaneously despite 3s intervals
   - Light tasks: Up to 5 running simultaneously with 2s intervals
   - Critical tasks: Always single execution with 10s intervals
   - Custom port: Web interface available at 8080 instead of 9990