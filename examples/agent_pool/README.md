# Agent Pool Example

This directory demonstrates agent pool functionality with a simple example.

## What is Agent Pool?

**Agent pool creates multiple terminal agents that can run tasks simultaneously.**

- `agent_pool_size: 1` (default) = Single agent like before
- `agent_pool_size: 2` = 2 agents on ports 9990, 9991

**Key benefit**: Tasks can run in parallel without waiting for each other.

## Files

### concurrency_demo.yaml
Simple configuration with:
- **Agent pool**: 2 agents on ports 9990, 9991
- **2 tasks**: Task A (every 3s) and Task B (every 4s)
- **Rule**: Reacts to task completion messages

### simple_task.sh
Basic demo script that takes 2 seconds and shows which task is running.

## Usage

```bash
# Run the agent pool demo
cargo run -- --rules examples/agent_pool/concurrency_demo.yaml

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
monitor:
  agent_pool_size: 2  # 2 agents in parallel

entries:
  - name: "task_a"
    trigger: "periodic"
    interval: "3s"
    
  - name: "task_b"  
    trigger: "periodic"
    interval: "4s"
```

Both tasks run independently on separate agents without blocking each other.