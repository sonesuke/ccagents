#!/bin/bash
cd /Users/sonesuke/rule-agents
echo "=== Current working directory ==="
pwd
echo "=== Git status ==="
git status
echo "=== Git diff --name-only ==="
git diff --name-only
echo "=== Git log --oneline -5 ==="
git log --oneline -5