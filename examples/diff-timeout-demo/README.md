# Diff Timeout Demo

This example demonstrates the new `diff_timeout` rule type that triggers actions when diff detection (pattern matching) has been inactive for a specified time period.

## How It Works

The `diff_timeout` rule type monitors the time since the last successful pattern match and triggers specified actions when timeout periods are reached. This is useful for:

- Detecting when monitoring has gone quiet
- Implementing watchdog behavior
- Triggering periodic actions during inactivity
- Detecting stuck or unresponsive processes

## Configuration Format

```yaml
rules:
  # Traditional pattern matching
  - when: "pattern_regex"
    action: "send_keys"
    keys: ["response"]

  # New timeout-based rules
  - diff_timeout: "30s"    # Trigger after 30 seconds of no pattern matches
    action: "send_keys"
    keys: ["timeout response"]

  - diff_timeout: "5m"     # Trigger after 5 minutes of no pattern matches
    action: "workflow"
    workflow: "restart_monitoring"
```

## Supported Time Formats

- `30s` - 30 seconds
- `5m` - 5 minutes  
- `2h` - 2 hours

## Demo Usage

1. **Start the demo:**
   ```bash
   cargo run -- --config examples/diff-timeout-demo/config.yaml
   ```

2. **Test pattern matching (resets timers):**
   - Type `hello` - triggers immediate response and resets all timeout timers
   - Type `test` - triggers immediate response and resets all timeout timers
   - Type `reset` - explicitly resets timeout timers

3. **Test timeout behavior:**
   - Wait 30 seconds without typing matching patterns - see short timeout trigger
   - Wait 1 minute - see medium timeout trigger
   - Wait 2 minutes - see long timeout trigger
   - Wait 5 minutes - see workflow timeout trigger

4. **Exit:**
   - Type `exit` or `quit` to cleanly exit

## Key Features

### Timer Reset Behavior
- Any successful pattern match resets ALL timeout timers
- This ensures timeouts only trigger during true inactivity periods
- Multiple timeouts can trigger if enough time passes without activity

### Priority System
- Pattern matching rules are checked first
- Timeout rules are checked periodically (every 50ms by default)
- Multiple timeout rules can trigger simultaneously if their durations have elapsed

### Action Types
- `send_keys` - Send keyboard input to the terminal
- `workflow` - Execute custom workflow (placeholder for future implementation)

## Implementation Details

The timeout functionality is implemented through:

1. **Rule Compilation:** `diff_timeout` field is parsed into `Duration` objects
2. **State Tracking:** `TimeoutState` tracks last activity time and timer states  
3. **Decision Engine:** Enhanced to handle both pattern matching and timeout checks
4. **Monitoring Loop:** Periodically checks timeout conditions every 50ms

This provides efficient, responsive timeout detection without impacting normal pattern matching performance.