#!/bin/bash

# Check if gh command is available
if ! command -v gh &> /dev/null; then
    echo "Error: gh command not found. Please install GitHub CLI." >&2
    exit 1
fi

# Check if authenticated
if ! gh auth status &> /dev/null; then
    echo "Error: Not authenticated with GitHub. Run 'gh auth login' first." >&2
    exit 1
fi

# Get all open issues
all_issues=$(gh issue list --json number --template '{{range .}}{{.number}}{{"\n"}}{{end}}')

# Get issues referenced in open PRs
referenced_issues=$(gh pr list --json title,body --template '{{range .}}{{.title}} {{.body}}{{"\n"}}{{end}}' | grep -o '#[0-9]\+' | sed 's/#//' | sort -u)

# Filter out referenced issues
if [ -n "$referenced_issues" ]; then
    echo "$all_issues" | grep -v -F "$referenced_issues"
else
    echo "$all_issues"
fi