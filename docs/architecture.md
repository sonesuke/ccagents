# RuleAgents Architecture

## Overview

RuleAgents is a terminal automation tool that provides YAML-driven control of interactive terminal sessions. It integrates with HyperTerminal (HT) to monitor terminal output and automatically respond based on configured rules.

## Core Architecture

### Single Agent Mode (Default)
```
┌─────────────────┐     ┌──────────────────┐     ┌────────────────┐
│   RuleAgents    │────▶│   HT Process     │────▶│  Web Terminal  │
│   (Controller)  │     │  (Terminal Emu)  │     │ localhost:9990 │
└────────┬────────┘     └─────────┬────────┘     └────────────────┘
         │                        │
         │ Monitors               │ Sends
         │ Output                 │ Keystrokes
         ▼                        ▼
┌─────────────────┐     ┌──────────────────┐
│  Terminal       │     │    Terminal      │
│  Output Buffer  │     │    Session       │
└─────────────────┘     └──────────────────┘
```

### Agent Pool Mode (agent_pool_size > 1)
```
                            ┌──────────────────┐     ┌────────────────┐
                       ┌───▶│   HT Process 1   │────▶│  Web Terminal  │
                       │    │  (Terminal Emu)  │     │ localhost:9990 │
┌─────────────────┐    │    └──────────────────┘     └────────────────┘
│   RuleAgents    │────┤    
│  (Controller)   │    │    ┌──────────────────┐     ┌────────────────┐
│   Agent Pool    │    ├───▶│   HT Process 2   │────▶│  Web Terminal  │
│                 │    │    │  (Terminal Emu)  │     │ localhost:9991 │
└─────────────────┘    │    └──────────────────┘     └────────────────┘
                       │    
                       │    ┌──────────────────┐     ┌────────────────┐
                       └───▶│   HT Process N   │────▶│  Web Terminal  │
                            │  (Terminal Emu)  │     │ localhost:999X │
                            └──────────────────┘     └────────────────┘
```

## Key Components

### 1. Agent Module (`agent/`)
Manages the HT process and terminal interactions:
- **Agent Pool**: Manages multiple parallel agents for improved throughput
- **HT Process Management**: Spawns and controls HyperTerminal subprocesses
- **Terminal Monitor**: Detects changes in terminal output using differential buffer analysis
- **Key Sender**: Sends keyboard input to the terminal session
- **State Detection**: Monitors if scripts are running or idle
- **Round-Robin Distribution**: Automatically distributes tasks across available agents

### 2. Ruler Module (`ruler/`)
Processes configuration and decides actions:
- **Config Loader**: Parses YAML configuration files
- **Pattern Matcher**: Evaluates regex patterns against terminal output
- **Action Decider**: Determines which action to execute based on matches
- **Priority System**: Rules are evaluated in order (first match wins)

### 3. Queue Module (`queue/`)
Manages task queues and periodic execution:
- **Queue Manager**: FIFO queues with event notifications
- **Queue Executor**: Command execution and result enqueueing
- **Periodic Triggers**: Timer-based task scheduling with configurable intervals
- **Variable Expansion**: Dynamic task substitution using `<task>` placeholders
- **Deduplication**: In-memory duplicate filtering for idempotent operations

### 4. Workflow Module (`workflow/`)
Manages complex multi-step operations:
- **Session Management**: Saves and restores terminal sessions
- **Action Execution**: Runs configured workflows
- **Recovery**: Handles interruptions and resumption
- **Hot Reload**: Supports dynamic workflow updates

### 5. HT Integration
RuleAgents depends on HyperTerminal (HT) for:
- Terminal emulation
- Web-based terminal access
- Terminal output capture
- Keyboard input injection

## Configuration Structure

### Monitor Configuration
```yaml
monitor:
  base_port: 9990        # First agent port (default: 9990)
  agent_pool_size: 1     # Number of parallel agents (default: 1)
```

### Entries - Event Triggers
```yaml
entries:
  - name: "descriptive_name"
    trigger: "on_start"        # Executes when RuleAgents starts
    action: "send_keys"        # Action type
    keys: ["command", "\r"]    # Keys to send (\r = Enter)
    
  - name: "periodic_task"
    trigger: "periodic"        # Executes at regular intervals
    interval: "15s"            # Interval (supports s, m, h)
    action: "enqueue"          # Add command output to queue
    queue: "tasks"             # Queue name
    command: "echo 'task'"     # Command to execute
    
  - name: "queue_processor"
    trigger: "enqueue:tasks"   # Executes when items added to queue
    action: "send_keys"
    keys: ["process <task>", "\r"]  # <task> expands to queue item
```

### Rules - Pattern Matching
```yaml
rules:
  - pattern: "regex_pattern"   # Regex to match terminal output
    action: "send_keys"        # Action to perform
    keys: ["response", "\r"]   # Keys to send
```

## Operation Flow

1. **Initialization**
   - Parse command-line arguments
   - Load YAML configuration
   - Create agent pool with specified size (default: 1)
   - Start HT processes on sequential ports (9990, 9991, ...)
   - Initialize terminal monitors for each agent

2. **Startup Phase**
   - Execute `on_start` entries using round-robin agent selection
   - Begin monitoring terminal output across all agents
   - Web terminals become accessible (one URL per agent)

3. **Runtime Loop**
   - Monitor terminal buffers for changes across all agents
   - Detect new terminal output from multiple agents
   - Match output against configured rules
   - Execute actions for matching patterns using available agents
   - Track script state (running/idle) per agent

4. **Action Execution**
   - `send_keys`: Inject keyboard input
   - `workflow`: Execute named workflow sequence
   - `enqueue`: Add command output to named queue
   - `enqueue_dedupe`: Add command output with duplicate filtering

5. **Queue Processing**
   - Periodic tasks execute commands at specified intervals using round-robin agent selection
   - Command output is processed line-by-line and added to queues
   - Queue listeners trigger automatically when items are added
   - Variable expansion replaces `<task>` with actual queue items
   - Each queue event is processed by the next available agent in the pool

## Terminal Output Detection

RuleAgents uses a sophisticated algorithm to detect new content in HT's fixed-width terminal buffer:

1. **Differential Detection**: Compares snapshots to find new lines
2. **Buffer Wrapping**: Handles terminal scrolling and line wrapping
3. **State Tracking**: Prevents duplicate triggers when scripts are idle
4. **Content Hashing**: Uses MD5 to identify unique output chunks

## Key Features

### Terminal Automation
- Pattern-based response to prompts
- Automated script execution
- Multi-step workflow support
- State-aware triggering

### Queue System
- Periodic task scheduling with flexible intervals
- FIFO queue processing with event notifications
- Automatic duplicate detection and filtering
- Dynamic variable expansion in commands

### Web Interface
- Real-time terminal viewing starting at http://localhost:9990
- Multiple terminals with agent pool (9990, 9991, 9992, ...)
- Multiple concurrent viewers supported per agent
- No installation required for viewers

### Reliability
- Session persistence and recovery
- Graceful shutdown handling
- Error recovery mechanisms
- Timeout protection

## Testing Support

RuleAgents includes special testing features:
- `test` command for rule validation
- `show` command for configuration inspection
- Mock backend support for unit tests
- Integration test framework

## Dependencies

- **HyperTerminal (HT)**: Required for terminal emulation
- **Tokio**: Async runtime
- **Regex**: Pattern matching
- **Serde/YAML**: Configuration parsing
- **Clap**: Command-line interface

## Limitations

- Requires HT to be installed separately
- Terminal detection relies on text patterns
- No built-in scheduling or cron-like features
- Limited to terminal-based automation

## Agent Pool Benefits

### Performance Improvements
- **Parallel Task Execution**: Multiple tasks run simultaneously without blocking
- **Better Resource Utilization**: Each agent uses independent terminal processes
- **Improved Throughput**: System can handle more concurrent operations

### Scalability
- **Configurable Pool Size**: Easily adjust number of agents based on workload
- **Round-Robin Distribution**: Automatic load balancing across agents
- **Independent Operation**: Agents don't interfere with each other

### Monitoring and Debugging
- **Per-Agent Web Interface**: Monitor each agent's terminal independently
- **Distributed Execution**: View how tasks are distributed across agents
- **Isolated Sessions**: Debugging issues doesn't affect other agents

## Future Considerations

- Plugin system for custom actions
- Enhanced pattern matching capabilities
- Terminal session recording/playback
- Advanced agent scheduling strategies
- REST API for external control