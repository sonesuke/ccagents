# RuleAgents Tutorial

This tutorial walks you through configuring and using RuleAgents with a practical example. You'll learn how to create automation rules and understand the core concepts through hands-on experience.

## Overview

RuleAgents automates terminal interactions using YAML configuration files. This tutorial uses a mock interactive script to demonstrate:
- How to write configuration rules
- Different types of automation triggers
- Pattern matching and automated responses
- Real-time monitoring via web interface

## Core Concepts

### Configuration Structure

A RuleAgents configuration file (`config.yaml`) contains two main sections:

#### 1. Entries - Event-Driven Triggers
```yaml
entries:
  - name: "descriptive_name"
    trigger: "event_type"    # When to activate
    action: "action_type"    # What to do
    keys: ["command", "\r"]  # Keys to send (\r = Enter)
```

**Available triggers:**
- `on_start`: Executes automatically when RuleAgents starts
- More triggers can be added as needed

#### 2. Rules - Pattern-Based Automation
```yaml
rules:
  - pattern: "text to match"   # Regex pattern to detect
    action: "send_keys"        # Action to perform
    keys: ["response", "\r"]   # Keys to send
```

**Key points:**
- Rules are evaluated in order (first rule = highest priority)
- Patterns support regular expressions
- Rules trigger automatically when patterns match terminal output

## Tutorial: Automating an Interactive Script

### Step 1: Understanding the Example

Our example uses a mock script that simulates an interactive process:
- Shows a countdown timer
- Asks for user input
- Processes the response
- Completes the operation

### Step 2: Configuration File Breakdown

Let's examine `config.yaml`:

```yaml
# Entry: Automatic startup
entries:
  - name: "start_mock"
    trigger: "on_start"           # Runs when RuleAgents starts
    action: "send_keys"
    keys: ["bash examples/mock.sh", "\r"]  # Executes the mock script

# Rules: Automated responses
rules:
  - pattern: "Do you want to proceed"    # Detects the prompt
    action: "send_keys"
    keys: ["1", "\r"]                    # Automatically selects option 1
    
  - pattern: "^exit$"                    # Detects "exit" command
    action: "send_keys"
    keys: ["/exit", "\r"]                # Sends special exit command
```

### Step 3: Running the Tutorial

1. **Start RuleAgents:**
   ```bash
   ./target/release/rule-agents --rules config.yaml
   ```

2. **Watch the automation:**
   - Open http://localhost:9990 in your browser
   - Observe the automated execution:
     - Script starts automatically (via `on_start` trigger)
     - Countdown runs
     - When prompted, option "1" is selected automatically
     - Process completes without manual intervention

### Step 4: Testing Your Configuration

RuleAgents provides a test command to verify rule matching:

```bash
# Test if a pattern matches correctly
./target/release/rule-agents test --rules config.yaml --capture "Do you want to proceed"

# Output shows which rule would trigger:
# Match found: rule at index 0
# Pattern: "Do you want to proceed"
# Action: send_keys ["1", "\r"]
```

## Creating Your Own Automation

### Example 1: Automating Git Operations
```yaml
entries:
  - name: "git_status"
    trigger: "on_start"
    action: "send_keys"
    keys: ["git status", "\r"]

rules:
  - pattern: "nothing to commit"
    action: "send_keys"
    keys: ["echo 'Repository is clean!'", "\r"]
```

### Example 2: Handling Yes/No Prompts
```yaml
rules:
  - pattern: "Are you sure.*\\[y/N\\]"  # Regex pattern
    action: "send_keys"
    keys: ["y", "\r"]
    
  - pattern: "Continue\\? \\(yes/no\\)"
    action: "send_keys"
    keys: ["yes", "\r"]
```

### Example 3: Multi-Step Workflows
```yaml
entries:
  - name: "deploy_workflow"
    trigger: "on_start"
    action: "send_keys"
    keys: ["./deploy.sh", "\r"]

rules:
  # Step 1: Environment selection
  - pattern: "Select environment.*production"
    action: "send_keys"
    keys: ["2", "\r"]  # Select production
    
  # Step 2: Confirmation
  - pattern: "Deploy to production\\?"
    action: "send_keys"
    keys: ["yes", "\r"]
    
  # Step 3: Authentication
  - pattern: "Enter deployment token:"
    action: "send_keys"
    keys: ["${DEPLOY_TOKEN}", "\r"]
```

## Best Practices

### 1. Pattern Design
- **Be specific**: Use precise patterns to avoid false matches
- **Use anchors**: `^` for start, `$` for end of line
- **Test patterns**: Use the test command before deployment

### 2. Rule Priority
- Order matters: Place more specific rules first
- General catch-all rules should go last
- Document why certain rules have higher priority

### 3. Safety Considerations
```yaml
rules:
  # Dangerous - too broad
  - pattern: "yes"
    action: "send_keys"
    keys: ["y", "\r"]
    
  # Better - more specific
  - pattern: "Remove all files\\? \\[yes/no\\]"
    action: "send_keys"
    keys: ["no", "\r"]  # Safe default
```

## Advanced Features

### Regular Expression Patterns
```yaml
rules:
  # Match version numbers
  - pattern: "Version: (\\d+\\.\\d+\\.\\d+)"
    action: "send_keys"
    keys: ["echo 'Detected version match'", "\r"]
    
  # Match file paths
  - pattern: "File not found: (.+\\.txt)$"
    action: "send_keys"
    keys: ["touch $1", "\r"]  # Create the missing file
```

### Combining Rules
```yaml
rules:
  # Handle different prompt formats
  - pattern: "(Continue|Proceed|Go ahead)\\?"
    action: "send_keys"
    keys: ["y", "\r"]
    
  # Multi-line pattern (careful with these)
  - pattern: "Summary:.*Total: \\d+ files"
    action: "send_keys"
    keys: ["echo 'Processing complete'", "\r"]
```

## Debugging Tips

1. **Enable verbose logging:**
   ```bash
   RUST_LOG=debug ./target/release/rule-agents --rules config.yaml
   ```

2. **Test patterns individually:**
   ```bash
   ./target/release/rule-agents test --rules config.yaml --capture "your text here"
   ```

3. **View the configuration:**
   ```bash
   ./target/release/rule-agents show --rules config.yaml
   ```

## Common Issues and Solutions

### Pattern Not Matching
- Check for extra whitespace in patterns
- Verify regex escape characters
- Use the test command to debug

### Wrong Rule Triggering
- Review rule order (priority)
- Make patterns more specific
- Consider using anchors (^$)

### Automation Too Fast
- Some applications need delays
- Consider breaking complex commands into steps
- Monitor via web interface at http://localhost:9990

## Next Steps

1. Start with simple patterns and gradually add complexity
2. Test each rule thoroughly before adding more
3. Build a library of common patterns for your workflows
4. Share useful configurations with your team

Remember: RuleAgents is powerful but requires careful configuration. Always test in a safe environment before using in production workflows!