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

Rules are defined in YAML format with the following simplified structure:

```yaml
rules:
  - priority: 10
    pattern: "issue\\s+(\\d+)"
    command: "entry"
    args: []
  - priority: 20
    pattern: "cancel"
    command: "cancel"
    args: []
```

### Rule Fields
- **priority**: Numeric priority (lower = higher priority, rules sorted ascending)  
- **pattern**: Regular expression pattern to match against input
- **command**: Command type to execute when pattern matches
- **args**: Optional command arguments (defaults to empty array)

### Supported Commands
- `entry`: Handle GitHub issue resolution workflow
- `cancel`: Cancel current operation
- `resume`: Resume interrupted operation

## Data Flow

1. **Startup**: Parse CLI args → Load YAML rules → Compile regex patterns → Sort by priority
2. **Runtime**: Input received → Match patterns (priority order) → Execute command
3. **Shutdown**: Receive Ctrl+C signal → Clean up resources

## Future Enhancements

- Rule hot-reloading (SIGHUP support)
- Configuration file validation
- Systemd service integration
- Metrics endpoint (HTTP server)
- Plugin system for custom actions
- Interactive mode for rule testing