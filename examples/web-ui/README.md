# Web UI Configuration Example

This example demonstrates how to configure and use the web-based terminal interface.

## Features Demonstrated

- **Web UI Configuration**: Enable and configure the web terminal interface
- **Multi-Agent Setup**: Configure multiple agents with sequential ports
- **Basic Automation**: Simple startup automation examples
- **Queue Integration**: Basic queue-based pattern matching

## Configuration

- `config.yaml`: Complete web UI configuration with multiple agents

## Usage

```bash
# Run with web UI enabled (multiple agents)
cargo run -- --config examples/web-ui/config.yaml

# Access web interfaces
# Agent 1: http://localhost:9990
# Agent 2: http://localhost:9991
```

## Key Configuration Options

```yaml
monitor:
  base_port: 9990          # Starting port for web interfaces
  agent_pool_size: 2       # Number of agent instances
  web_ui:
    enabled: true          # Enable web terminal interface
    host: "localhost"      # Bind address (use "0.0.0.0" for external access)
```

## Web Interface Features

- Real-time terminal display with AVT color support
- Configurable terminal dimensions via YAML
- WebSocket-based communication
- Input capability through web interface
- Clean, modern terminal UI