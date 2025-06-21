#!/bin/bash

# Test script to verify mock.sh works with rule-agents

echo "Testing mock.sh directly..."
echo ""

# Test 1: Direct execution with option 1
echo "Test 1: Direct execution with option 1"
echo "1" | timeout 15s bash examples/mock.sh
echo ""

# Test 2: Direct execution with /exit
echo "Test 2: Direct execution with /exit"
echo "/exit" | timeout 10s bash examples/mock.sh
echo ""

echo "Tests complete!"