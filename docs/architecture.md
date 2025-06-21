# RuleAgents Architecture

## Overview

RuleAgents is a command-line tool that provides a YAML-driven agent auto-control system. It runs as a daemon process that monitors events, evaluates conditions, and executes actions according to rules defined in YAML configuration files.

## Usage

```bash
# Run with default rules.yaml
rule-agents

# Run with specific rules file
rule-agents --rules /path/to/my-rules.yaml
```

## Core Components

### 1. Rule Engine
The heart of the system that:
- Loads and parses YAML rule files
- Compiles rules into an efficient runtime representation
- Manages rule execution based on triggers
- Handles condition evaluation
- Executes configured actions

### 2. Agent Manager
Manages multiple agents that:
- Execute commands and tasks
- Monitor file system changes
- Collect system metrics
- Handle inter-agent communication

### 3. Event System
- File system events (using `notify` crate)
- Timer-based events
- Custom event sources

### 4. Metrics & Monitoring
- Prometheus metrics export
- HTTP server for metrics endpoint (using `warp`)
- Rule execution statistics

## Rule Structure

Rules are defined in YAML format with the following structure:

```yaml
rules:
  - name: "rule-name"
    description: "What this rule does"
    trigger:
      type: "trigger_type"
      # trigger-specific configuration
    conditions:  # optional
      - type: "condition_type"
        # condition-specific configuration
    actions:
      - type: "action_type"
        # action-specific configuration
```

### Supported Triggers
- `file_change`: Monitors file system changes
- `interval`: Executes at regular intervals

### Supported Conditions
- `pattern_match`: Regex pattern matching

### Supported Actions
- `log`: Log messages
- `notify`: Send notifications
- `command`: Execute shell commands
- `collect_metrics`: Gather system metrics

## Data Flow

1. **Startup**: Parse CLI args → Load YAML rules → Compile rules → Start engine
2. **Runtime**: Event occurs → Match triggers → Evaluate conditions → Execute actions  
3. **Shutdown**: Receive Ctrl+C signal → Stop engine → Clean up resources

## Future Enhancements

- Rule hot-reloading (SIGHUP support)
- Configuration file validation
- Systemd service integration
- Metrics endpoint (HTTP server)
- Plugin system for custom actions
- Interactive mode for rule testing