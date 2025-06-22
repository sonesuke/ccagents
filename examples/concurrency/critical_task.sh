#!/bin/bash
# Critical task simulation - must run alone (concurrency = 1)

TASK_ID="critical-$$-$(date +%s)"
echo "Starting critical task: $TASK_ID"

# Check if another critical task is running (simulation)
LOCK_FILE="/tmp/critical_task.lock"

if [ -f "$LOCK_FILE" ]; then
    echo "ERROR: Too many concurrent critical tasks detected!"
    echo "Critical task $TASK_ID aborted"
    exit 1
fi

# Create lock file
echo $TASK_ID > "$LOCK_FILE"

echo "Critical task $TASK_ID has exclusive access"

# Simulate critical processing (3-7 seconds)
SLEEP_TIME=$((3 + RANDOM % 5))
echo "Critical task $TASK_ID processing for $SLEEP_TIME seconds"

sleep $SLEEP_TIME

# Clean up lock file
rm -f "$LOCK_FILE"

echo "Task completed: $TASK_ID"
echo "Critical task finished safely"