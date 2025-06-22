#!/bin/bash
# Heavy task simulation - takes time and should have limited concurrency

TASK_ID="heavy-$$-$(date +%s)"
echo "Starting heavy task: $TASK_ID"

# Simulate heavy processing (5-10 seconds)
SLEEP_TIME=$((5 + RANDOM % 6))
echo "Heavy task $TASK_ID will take $SLEEP_TIME seconds"

sleep $SLEEP_TIME

echo "Task completed: $TASK_ID"
echo "Heavy task finished successfully"