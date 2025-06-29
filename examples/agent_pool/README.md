# Agent Pool Example

This directory demonstrates agent pool functionality with a simple example.

## What is Agent Pool?

**Agent pool creates multiple terminal agents that can run tasks simultaneously.**

- `agents.concurrency: 1` (default) = Single agent like before
- `agents.concurrency: 2` = 2 agents on ports 9990, 9991

**Key benefit**: Tasks can run in parallel without waiting for each other.

## Files

### config.yaml
Simple configuration with:
- **Agent pool**: 2 agents on ports 9990, 9991
- **2 tasks**: Task A (every 3s) and Task B (every 4s)
- **Rule**: Reacts to task completion messages

### simple_task.sh
Basic demo script that takes 2 seconds and shows which task is running.

## Usage

```bash
# Run the agent pool demo
cargo run -- --config examples/agent_pool/config.yaml

# Watch terminals at:
# - http://localhost:9990 (Agent 1)
# - http://localhost:9991 (Agent 2)  
```

## Expected Behavior

You'll see:
1. **2 browser tabs**: Each agent shows in its own tab
2. **Parallel execution**: Task A and Task B run simultaneously on different agents
3. **Round-robin distribution**: Tasks automatically distributed across agents

### Timeline Example

```
Time 0s:  ✅ task_a → Agent 1 (9990)
Time 0s:  ✅ task_b → Agent 2 (9991)  
Time 3s:  ✅ task_a → Agent 1 (9990) [previous finished]
Time 4s:  ✅ task_b → Agent 2 (9991) [previous finished]
```

## Configuration

```yaml
web_ui:
  enabled: true
  host: "localhost"
  base_port: 9990

agents:
  concurrency: 2  # 2 agents in parallel
  cols: 80
  rows: 24

entries:
  - name: "task_a"
    trigger: "periodic"
    interval: "3s"
    
  - name: "task_b"  
    trigger: "periodic"
    interval: "4s"
```

Both tasks run independently on separate agents without blocking each other.