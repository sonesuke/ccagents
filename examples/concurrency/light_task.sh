#!/bin/bash
# Light task simulation - quick execution, can handle higher concurrency

TASK_ID="light-$$-$(date +%s)"
echo "Starting light task: $TASK_ID"

# Simulate light processing (1-3 seconds)
SLEEP_TIME=$((1 + RANDOM % 3))
echo "Light task $TASK_ID processing for $SLEEP_TIME seconds"

sleep $SLEEP_TIME

echo "Task completed: $TASK_ID"
echo "Light task finished quickly"