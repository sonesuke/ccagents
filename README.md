# RuleAgents

A command-line tool for YAML-driven agent auto-control system

## Installation

```bash
cargo build --release
```

## Usage

```bash
# Run with default rules.yaml file
./target/release/rule-agents

# Run with specific rules file
./target/release/rule-agents --rules examples/basic-rules.yaml

# Show help
./target/release/rule-agents --help
```

## Configuration

Create a YAML file with your rules. See `examples/basic-rules.yaml` for reference:

```yaml
rules:
  - priority: 10
    pattern: "issue\\s+(\\d+)"
    command: "solve-issue"
    args: []
  - priority: 20
    pattern: "cancel"
    command: "cancel"
    args: []
  - priority: 30
    pattern: "resume"
    command: "resume"
    args: []
```

### Rule Structure

- **priority**: Lower numbers = higher priority (rules are sorted by priority)
- **pattern**: Regular expression to match against input
- **command**: Command to execute when pattern matches (`solve-issue`, `cancel`, `resume`)
- **args**: Optional arguments for the command (defaults to empty array)

## Development

### Building from Source

```bash
cargo build          # Debug build
cargo test           # Run tests (also sets up git hooks)
cargo run -- --help  # Run with help flag
```

### Quality Checks

Git hooks are automatically set up by `cargo-husky` when you first run `cargo test`:

```bash
cargo check                    # Check compilation
cargo test                     # Run tests
cargo clippy -- -D warnings    # Lint checks
cargo fmt                      # Auto-format code
```

### Architecture

See [docs/architecture.md](docs/architecture.md) for system design details.