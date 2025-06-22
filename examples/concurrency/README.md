# Concurrency Control Examples

This directory demonstrates the new concurrency control features for entries, including configurable base ports and entry-level concurrency limits.

## Files

### concurrency_demo.yaml
Configuration demonstrating:
- **Custom base port**: Monitor section with configurable port (8080 instead of default 9990)
- **Entry concurrency limits**: Different concurrency settings for different task types
- **Periodic triggers**: Multiple tasks running at different intervals
- **Concurrency behavior**: How limits prevent system overload

### Scripts

#### heavy_task.sh
Simulates resource-intensive tasks:
- Takes 5-10 seconds to complete
- Should have limited concurrency (2) to prevent system overload
- Demonstrates why concurrency limits are needed for heavy operations

#### light_task.sh  
Simulates lightweight tasks:
- Takes 1-3 seconds to complete
- Can handle higher concurrency (5) since it's fast
- Shows how different task types need different limits

#### critical_task.sh
Simulates critical operations:
- Must run alone (concurrency = 1)
- Uses file locking to demonstrate single-instance behavior
- Shows error handling when concurrency limits are exceeded

## Usage

```bash
# Run concurrency control demo
cargo run -- --rules examples/concurrency/concurrency_demo.yaml

# Access web interface at custom port
# http://localhost:8080 (instead of default 9990)
```

## How It Works

### 1. Monitor Configuration
```yaml
monitor:
  base_port: 8080  # Custom base port
```

### 2. Entry Concurrency Settings
```yaml
entries:
  - name: "heavy_task"
    concurrency: 2    # Max 2 simultaneous executions
    
  - name: "light_task"  
    concurrency: 5    # Max 5 simultaneous executions
    
  - name: "critical_task"
    concurrency: 1    # Single execution only (default)
```

### 3. Observed Behavior

**Heavy Tasks (concurrency: 2)**:
- Even with 3-second intervals, max 2 run simultaneously
- Additional tasks wait for permits to become available
- Prevents system overload from too many heavy operations

**Light Tasks (concurrency: 5)**:
- With 2-second intervals, up to 5 can run concurrently
- Higher throughput for lightweight operations
- Efficient resource utilization

**Critical Tasks (concurrency: 1)**:
- Only one instance runs at a time
- New attempts wait for the current one to complete
- Ensures exclusive access to shared resources

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