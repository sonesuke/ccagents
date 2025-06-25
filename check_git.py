#!/usr/bin/env python3
import subprocess
import os
import sys

def run_git_command(cmd):
    try:
        os.chdir('/Users/sonesuke/rule-agents')
        result = subprocess.run(cmd, shell=True, capture_output=True, text=True)
        return result.returncode, result.stdout, result.stderr
    except Exception as e:
        return -1, "", str(e)

# Check git status
print("=== Git Status ===")
code, stdout, stderr = run_git_command("git status --porcelain")
if code == 0:
    if stdout.strip():
        print("Modified files:")
        print(stdout)
    else:
        print("No changes detected")
else:
    print(f"Error: {stderr}")

# Check current branch
print("\n=== Current Branch ===")
code, stdout, stderr = run_git_command("git branch --show-current")
if code == 0:
    print(f"Current branch: {stdout.strip()}")
else:
    print(f"Error: {stderr}")

# Check for modified files
print("\n=== Modified Files ===")
code, stdout, stderr = run_git_command("git diff --name-only")
if code == 0:
    if stdout.strip():
        print("Modified files:")
        for file in stdout.strip().split('\n'):
            print(f"  - {file}")
    else:
        print("No modified files")
else:
    print(f"Error: {stderr}")