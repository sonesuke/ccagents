# Terminal Output Monitoring

The Terminal Output Monitoring system provides real-time detection of agent state transitions based on HT (terminal) process output analysis. This enables precise agent state management based on actual terminal behavior rather than assumptions.

## Overview

The system monitors HT process terminal output to determine agent states:

- **Idle**: Shell prompt is displayed (e.g., `user@hostname:/path$ `, `# `, `> `)
- **Wait**: Command is running but terminal output has no changes over time
- **Active**: Command is running and terminal output is continuously changing

## Core Components

### TerminalOutputMonitor

The main monitoring component that:
- Subscribes to HT terminal output events
- Analyzes terminal snapshots for state detection
- Emits state transition events
- Handles timeout scenarios for stuck commands

### AgentState

Represents the current state of the agent:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentState {
    Idle,   // Shell prompt detected
    Wait,   // No output changes
    Active, // Output changing
}
```

### StateTransition

Captures state change events with context:

```rust
#[derive(Debug, Clone)]
pub struct StateTransition {
    pub from: AgentState,
    pub to: AgentState,
    pub timestamp: Instant,
    pub snapshot: TerminalSnapshot,
}
```

## Configuration

### MonitorConfig

Configurable monitoring parameters:

```rust
pub struct MonitorConfig {
    /// Interval between terminal snapshots for change detection
    pub change_detection_interval: Duration,
    /// How long to wait without output changes before transitioning to Wait state
    pub wait_timeout: Duration,
    /// Maximum time to stay in Wait state before considering command stuck
    pub stuck_timeout: Duration,
    /// Shell prompt patterns for Idle state detection
    pub prompt_patterns: Vec<String>,
    /// Number of snapshots to keep in history for comparison
    pub snapshot_history_size: usize,
}
```

Default configuration:
- **change_detection_interval**: 500ms
- **wait_timeout**: 2 seconds
- **stuck_timeout**: 30 seconds
- **prompt_patterns**: Common shell prompts (bash, root, etc.)
- **snapshot_history_size**: 10 snapshots

## Usage

### Basic Usage

```rust
use rule_agents::{
    HtClient, HtProcess, HtProcessConfig, TerminalOutputMonitor, MonitorConfig
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create and start HT client
    let ht_config = HtProcessConfig::default();
    let ht_process = HtProcess::new(ht_config);
    let ht_client = Arc::new(HtClient::new(ht_process));
    ht_client.start().await?;

    // Create monitor
    let mut monitor = TerminalOutputMonitor::new(ht_client.clone()).await?;
    
    // Start monitoring and get state receiver
    let mut state_receiver = monitor.start_monitoring();
    
    // Run monitoring in background task
    tokio::spawn(async move {
        monitor.run_monitoring_loop().await
    });
    
    // Handle state transitions
    while let Some(transition) = state_receiver.recv().await {
        println!("State: {} -> {}", 
            format_state(&transition.from), 
            format_state(&transition.to)
        );
    }
    
    Ok(())
}
```

### Custom Configuration

```rust
let monitor_config = MonitorConfig {
    change_detection_interval: Duration::from_millis(250),
    wait_timeout: Duration::from_secs(1),
    stuck_timeout: Duration::from_secs(15),
    prompt_patterns: vec![
        r".*@.*:\S*\$ $".to_string(),    // bash prompt
        r".*# $".to_string(),            // root prompt  
        r"\$ $".to_string(),             // simple prompt
    ],
    snapshot_history_size: 5,
};

let mut monitor = TerminalOutputMonitor::with_config(
    ht_client.clone(), 
    monitor_config
).await?;
```

## State Detection Logic

### Idle State Detection

Uses regex patterns to identify shell prompts:
- `user@hostname:.*\$ $` (bash prompt)
- `.*# $` (root prompt)
- `.*> $` (other shell prompts)
- `\$ $` (simple dollar prompt)

When any pattern matches the last line of terminal output → **Idle state**

### Active/Wait State Detection

Compares terminal snapshots at regular intervals:
- **Output changing** between intervals → **Active state**
- **Output unchanged** for `wait_timeout` duration → **Wait state**
- **Wait state** exceeding `stuck_timeout` → Warning logged

### Change Detection

Compares both content and cursor position:
```rust
fn has_output_changed(&self, current_snapshot: &TerminalSnapshot) -> bool {
    if let Some(previous_snapshot) = self.previous_snapshots.back() {
        let content_changed = current_snapshot.content != previous_snapshot.content;
        let cursor_changed = current_snapshot.cursor_x != previous_snapshot.cursor_x
            || current_snapshot.cursor_y != previous_snapshot.cursor_y;
        
        content_changed || cursor_changed
    } else {
        true // No previous snapshot - consider as change
    }
}
```

## Event Subscription Requirements

**⚠️ Critical Setup**: The HT client must call `subscribe_to_events()` during initialization before using `take_snapshot()`. The `TerminalOutputMonitor` automatically handles this:

```rust
// Automatic event subscription in TerminalOutputMonitor::new()
let events = vec!["terminalOutput".to_string()];
let event_receiver = ht_client.subscribe_to_events(events).await?;
```

Without proper event subscription, snapshot requests will not return results.

## State Transition Scenarios

### Common Transition Patterns

1. **Command Execution**:
   ```
   Idle → Active → Idle
   ```
   Quick command that completes immediately.

2. **Long-Running Command**:
   ```
   Idle → Active → Wait → Active → Idle
   ```
   Command with periods of inactivity.

3. **Interactive Command**:
   ```
   Idle → Active → Wait → Active → Wait → Idle
   ```
   Command requiring user input or processing in batches.

4. **Stuck Command**:
   ```
   Idle → Active → Wait (exceeds stuck_timeout)
   ```
   Command that hangs or waits indefinitely.

## Error Handling

### MonitorError Types

```rust
#[derive(Debug, Error)]
pub enum MonitorError {
    #[error("HT client error: {0}")]
    HtClientError(#[from] HtClientError),
    #[error("Event subscription failed: {0}")]
    SubscriptionError(String),
    #[error("Regex error: {0}")]
    RegexError(#[from] regex::Error),
    #[error("Monitor timeout: {0}")]
    Timeout(String),
    #[error("Monitor not running")]
    NotRunning,
}
```

### Timeout Handling

- **Snapshot Timeout**: 5 seconds timeout for terminal snapshots
- **Wait State Timeout**: Configurable via `wait_timeout` 
- **Stuck Command Timeout**: Configurable via `stuck_timeout`

## Performance Considerations

### Snapshot Comparison Efficiency

- Maintains circular buffer of snapshots (configurable size)
- Compares only content and cursor position (not full terminal state)
- Uses string comparison for content changes

### Memory Usage

- Snapshot history limited by `snapshot_history_size` (default: 10)
- Each snapshot contains terminal content as string
- Automatic cleanup of old snapshots

### CPU Usage

- Configurable monitoring interval (default: 500ms)
- Regex pattern matching only on last line of output
- Event-driven architecture minimizes polling

## Integration Examples

### With Rule Engine

```rust
// React to state transitions in rule evaluation
match transition.to {
    AgentState::Idle => {
        // Command completed - evaluate completion rules
        rule_engine.evaluate_completion_rules(&transition.snapshot).await?;
    }
    AgentState::Wait => {
        // Command waiting - check if intervention needed
        rule_engine.evaluate_wait_rules(&transition.snapshot).await?;
    }
    AgentState::Active => {
        // Command running - monitor progress
        rule_engine.evaluate_progress_rules(&transition.snapshot).await?;
    }
}
```

### With Session Management

```rust
// Update session state based on agent state
session_manager.update_agent_state(transition.to.clone()).await?;

// Save state transitions for session persistence
session_manager.record_state_transition(transition).await?;
```

## Testing

### Unit Tests

The module includes comprehensive unit tests:
- Shell prompt pattern detection
- Output change detection 
- State transition logic
- Configuration handling
- Serialization/deserialization

### Integration Testing

Run integration tests with a real HT process:
```bash
cargo test terminal_output_monitor --features integration-tests
```

### Example Usage

See `examples/terminal_monitor_example.rs` for a complete working example demonstrating:
- HT client setup and event subscription
- Monitor configuration and initialization
- State transition handling
- Command execution scenarios

## Troubleshooting

### Common Issues

1. **No State Transitions**:
   - Ensure HT client is properly started
   - Verify event subscription is working
   - Check terminal output is being generated

2. **Incorrect Idle Detection**:
   - Review prompt patterns in configuration
   - Test regex patterns against actual shell prompts
   - Add custom patterns for non-standard shells

3. **Performance Issues**:
   - Increase `change_detection_interval` for less frequent checks
   - Reduce `snapshot_history_size` to use less memory
   - Monitor CPU usage and adjust accordingly

### Debug Logging

Enable debug logging to troubleshoot issues:
```rust
tracing_subscriber::init();
```

Key log messages:
- Event subscription and reception
- State transitions with context
- Prompt pattern matches
- Timeout warnings for stuck commands

## Dependencies

- **HT Backend**: Requires completion of HT integration (#22)
- **Event System**: Proper HT client event subscription
- **Regex**: For shell prompt pattern matching
- **Tokio**: For async runtime and channels
- **Tracing**: For logging and debugging