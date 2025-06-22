#!/bin/bash
# Simple task for agent pool demonstration

TASK_NAME=${1:-Unknown}
TASK_ID="$TASK_NAME-$$-$(date +%s)"

echo "Starting task: $TASK_ID"

# Simple processing (2-3 seconds)
sleep 2

echo "Task $TASK_NAME completed"
echo "Task finished successfully"