# Basic Examples

This directory contains basic examples demonstrating fundamental RuleAgents features.

## Files

### config.yaml
Simple rule-based automation configuration showing:
- **On-start triggers**: Execute actions when the program starts
- **Pattern matching**: Respond to specific text patterns in terminal output
- **Send keys**: Automated keyboard input to terminal

### mock.sh
Mock script that simulates an interactive process:
- Displays a countdown
- Prompts for user input
- Processes the response

## Usage

```bash
# Run basic example (default)
cargo run

# Run with explicit config
cargo run -- --rules examples/basic/config.yaml
```

## How It Works

1. **Startup**: Automatically executes `mock.sh`
2. **Monitoring**: Watches terminal output for patterns
3. **Response**: When "Do you want to proceed" appears, automatically sends "1"
4. **Completion**: When "exit" appears, sends "/exit"

## Configuration Details

```yaml
entries:
  - name: "start_mock"
    trigger: "on_start"
    action: "send_keys"
    keys: ["bash examples/basic/mock.sh", "\r"]

rules:
  - pattern: "Do you want to proceed"
    action: "send_keys"
    keys: ["1", "\r"]
    
  - pattern: "^exit$"
    action: "send_keys"
    keys: ["/exit", "\r"]
```

This demonstrates the basic workflow of automated terminal interaction with pattern-based responses.