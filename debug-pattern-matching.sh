#!/bin/bash

echo "=== Pattern Matching Debug Guide ==="
echo ""
echo "The issue you're experiencing is likely due to the pattern 'こんにちは|Hello' being too broad."
echo "It's matching 'Hello' somewhere in the terminal output before you expect it to."
echo ""
echo "To debug this issue:"
echo ""
echo "1. Run RuleAgents with debug logging enabled:"
echo "   cargo run -- --debug -c test-config.yaml"
echo ""
echo "2. Look for these log messages:"
echo "   🔎 New content to check: <content>"
echo "   ✅ Pattern matched! Pattern: <pattern>, Capture: <content>"
echo ""
echo "3. Common solutions:"
echo "   a) Use more specific patterns:"
echo "      - Instead of: 'Hello'"
echo "      - Use: '^Hello$' (exact line match)"
echo "      - Or: 'Hello, what.*' (match specific phrase)"
echo ""
echo "   b) Use word boundaries:"
echo "      - Pattern: '\\bHello\\b' (matches whole word only)"
echo ""
echo "   c) Make patterns context-specific:"
echo "      - Pattern: 'Assistant:.*Hello' (matches Hello only after Assistant:)"
echo ""
echo "Example of a more robust config:"
echo ""
cat << 'EOF'
rules:
  - pattern: "Do you want to proceed\\?"  # Escape the question mark
    action: "send_keys"
    keys: ["1", "\r"]
    
  - pattern: "^(こんにちは|Hello)\\s*$"  # Match only at line start/end with optional whitespace
    action: "send_keys"
    keys: ["q", "\r"]
    
  # Or even more specific:
  - pattern: "Assistant:.*こんにちは"   # Match only when Assistant says こんにちは
    action: "send_keys"
    keys: ["q", "\r"]
EOF