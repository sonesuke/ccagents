# Mock Test Guide

This guide explains how to use the mock script and rules to test the agent automation system.

## Overview

The mock test scenario provides a concrete example of how `rule-agents` can automate interactive processes. It consists of:

- **`mock.sh`**: A bash script that simulates an interactive process with countdown and user prompts
- **`mock-rules.yaml`**: Rule definitions that automate interactions with the script

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

Run rule-agents with the mock rules:

```bash
rule-agents --rules examples/mock-rules.yaml
```

### Test Flow

1. **Start the test**:
   ```
   > entry
   ```
   This command triggers the mock script execution.

2. **Automatic flow**:
   - The script displays a 5-second countdown
   - When prompted "Do you want to proceed?", the agent automatically responds with "1"
   - The script processes for 3 seconds
   - Displays "MISSION COMPLETE" and exits

3. **Manual controls**:
   - `exit`: Sends `/exit` to the script to terminate it gracefully
   - `cancel`: Sends Ctrl+C to interrupt the script
   - `resume`: Legacy command (displays deprecation message)

## Testing Different Scenarios

### Test Automatic Response
```bash
# This tests the full automated flow
rule-agents test --rules examples/mock-rules.yaml --capture "entry"
```

### Test Exit Command
```bash
# This tests the exit command handling
rule-agents test --rules examples/mock-rules.yaml --capture "exit"
```

### Test Cancel Command
```bash
# This tests the cancel/interrupt handling
rule-agents test --rules examples/mock-rules.yaml --capture "cancel"
```

## Expected Behavior

### Successful Run
```
> entry
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
Enter your choice: 1
Processing request...
Processing... (1/3)
Processing... (2/3)
Processing... (3/3)

=== MISSION COMPLETE ===
The operation has been successfully completed!
```

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
    timeout 30s rule-agents test --rules examples/mock-rules.yaml --capture "entry"
```

## Troubleshooting

1. **Script not found**: Ensure you're running from the project root directory
2. **Permission denied**: Run `chmod +x examples/mock.sh`
3. **Timeout issues**: The full test should complete in about 10 seconds
4. **No auto-response**: Check that the capture pattern in `mock-rules.yaml` matches exactly

## Extending the Test

You can modify the mock scenario to test more complex interactions:

1. Add more prompts to `mock.sh`
2. Define corresponding rules in `mock-rules.yaml`
3. Test different response patterns and timing scenarios

This mock test provides a foundation for developing and testing more sophisticated automation scenarios with rule-agents.