# Mock Test Guide

This guide explains how to use the mock script and rules to test the agent automation system.

## Overview

The mock test scenario provides a concrete example of how `rule-agents` can automate interactive processes. It consists of:

- **`mock.sh`**: A bash script that simulates an interactive process with countdown and user prompts
- **`config.yaml`**: Entry and rule definitions that control automation behavior

## Concepts

### Entries vs Rules

The system distinguishes between two types of automation:

- **Entries**: External triggers initiated by system events (e.g., startup, user commands)
- **Rules**: Automatic detection triggered by terminal state changes (e.g., prompts, output patterns)

## Setup

1. Ensure the mock script is executable:
   ```bash
   chmod +x examples/mock.sh
   ```

2. Test the script manually (optional):
   ```bash
   bash examples/mock.sh
   ```

## Running with Rule Agents

### Basic Usage

Run rule-agents with the config:

```bash
rule-agents --rules config.yaml
```

### Test Flow

1. **Automatic startup**:
   - The `on_start` entry trigger automatically starts the mock script
   - No manual command input required

2. **Automatic flow**:
   - The script displays a 5-second countdown
   - When prompted "Do you want to proceed?", the agent automatically responds with "1"
   - The script processes for 3 seconds
   - Displays "MISSION COMPLETE" and exits

3. **Terminal access**:
   - Open http://localhost:9990 in your browser to view the automated terminal
   - The automation runs automatically without user intervention

## Configuration Structure

### config.yaml Format

```yaml
# External triggers - initiated by system events
entries:
  - name: "start_mock"
    trigger: "on_start"    # Automatic startup trigger
    action: "send_keys"
    keys: ["bash examples/mock.sh", "\r"]

# Automatic detection rules - triggered by terminal state changes
# Higher priority = earlier in the list (line order matters)
rules:
  - pattern: "Do you want to proceed"  # Highest priority
    action: "send_keys"
    keys: ["1", "\r"]
    
  - pattern: "^exit$"                  # Lower priority
    action: "send_keys"
    keys: ["/exit", "\r"]
```

### Testing Different Scenarios

#### Test Automatic Response
```bash
# This tests the full automated flow
rule-agents test --rules config.yaml --capture "Do you want to proceed"
```

#### Test Exit Command Detection
```bash
# This tests the exit command handling
rule-agents test --rules config.yaml --capture "exit"
```

## Expected Behavior

### Successful Run
```
=== Mock Test Script ===
This script simulates an interactive process for testing rule-agents.

Starting countdown...
Countdown: 5 seconds remaining...
Countdown: 4 seconds remaining...
Countdown: 3 seconds remaining...
Countdown: 2 seconds remaining...
Countdown: 1 seconds remaining...
Countdown complete!

Do you want to proceed?
1) Continue with mission
2) Cancel operation
(Type '/exit' to exit the script)
Enter your choice: 1                    # ← Automatically selected by rules
Processing request...
Processing... (1/3)
Processing... (2/3)
Processing... (3/3)

=== MISSION COMPLETE ===
The operation has been successfully completed!
```

Note: The script starts automatically via the `on_start` trigger, and "1" is selected automatically when the "Do you want to proceed?" prompt appears.

## Manual Script Testing

You can also test the script independently without rule-agents:

```bash
# Run the script
bash examples/mock.sh

# When prompted, you can:
# - Enter "1" to continue
# - Enter "2" to cancel
# - Type "/exit" to exit
# - Any other input shows help
```

## CI Integration

This test scenario can be integrated into CI pipelines:

```yaml
# Example GitHub Actions step
- name: Test Mock Scenario
  run: |
    chmod +x examples/mock.sh
    timeout 30s rule-agents --rules config.yaml
```

## Troubleshooting

1. **Script not found**: Ensure you're running from the project root directory
2. **Permission denied**: Run `chmod +x examples/mock.sh`
3. **Timeout issues**: The full test should complete in about 10 seconds
4. **No auto-response**: Check that the pattern in `config.yaml` matches exactly
5. **Script doesn't start**: Verify the `on_start` trigger is properly configured

## Key Features

- **Automatic startup**: No manual intervention required
- **Priority-based rules**: Line order determines rule priority
- **Clear separation**: Entries for triggers, rules for detection
- **Web interface**: View automation in real-time at http://localhost:9990

## Extending the Test

You can modify the mock scenario to test more complex interactions:

1. Add more prompts to `mock.sh`
2. Define corresponding rules in `config.yaml`
3. Test different trigger types and response patterns
4. Experiment with rule priorities and pattern matching

This mock test provides a foundation for developing and testing more sophisticated automation scenarios with rule-agents.