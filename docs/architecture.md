# RuleAgents Architecture

## Overview

RuleAgents is a terminal automation tool that provides YAML-driven control of interactive terminal sessions. It integrates with HyperTerminal (HT) to monitor terminal output and automatically respond based on configured rules.

## Core Architecture

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

## Key Components

### 1. Agent Module (`agent/`)
Manages the HT process and terminal interactions:
- **HT Process Management**: Spawns and controls HyperTerminal subprocess
- **Terminal Monitor**: Detects changes in terminal output using differential buffer analysis
- **Key Sender**: Sends keyboard input to the terminal session
- **State Detection**: Monitors if scripts are running or idle

### 2. Ruler Module (`ruler/`)
Processes configuration and decides actions:
- **Config Loader**: Parses YAML configuration files
- **Pattern Matcher**: Evaluates regex patterns against terminal output
- **Action Decider**: Determines which action to execute based on matches
- **Priority System**: Rules are evaluated in order (first match wins)

### 3. Workflow Module (`workflow/`)
Manages complex multi-step operations:
- **Session Management**: Saves and restores terminal sessions
- **Action Execution**: Runs configured workflows
- **Recovery**: Handles interruptions and resumption
- **Hot Reload**: Supports dynamic workflow updates

### 4. HT Integration
RuleAgents depends on HyperTerminal (HT) for:
- Terminal emulation
- Web-based terminal access
- Terminal output capture
- Keyboard input injection

## Configuration Structure

### Entries - Event Triggers
```yaml
entries:
  - name: "descriptive_name"
    trigger: "on_start"        # Executes when RuleAgents starts
    action: "send_keys"        # Action type
    keys: ["command", "\r"]    # Keys to send (\r = Enter)
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
   - Start HT process on available port (9990+)
   - Initialize terminal monitor

2. **Startup Phase**
   - Execute `on_start` entries
   - Begin monitoring terminal output
   - Web terminal becomes accessible

3. **Runtime Loop**
   - Monitor terminal buffer for changes
   - Detect new terminal output
   - Match output against configured rules
   - Execute actions for matching patterns
   - Track script state (running/idle)

4. **Action Execution**
   - `send_keys`: Inject keyboard input
   - `workflow`: Execute named workflow sequence

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

### Web Interface
- Real-time terminal viewing at http://localhost:9990
- Multiple concurrent viewers supported
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

## Future Considerations

- Plugin system for custom actions
- Enhanced pattern matching capabilities
- Terminal session recording/playback
- Multi-terminal orchestration
- REST API for external control