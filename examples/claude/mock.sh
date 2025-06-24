#!/bin/bash

# Mock script for testing agent automation

# Function to handle exit
handle_exit() {
    echo "Exiting mock script..."
    exit 0
}

# Function to show countdown
countdown() {
    local seconds=$1
    echo "Starting countdown..."
    for ((i=$seconds; i>=1; i--)); do
        echo "Countdown: $i seconds remaining..."
        sleep 1
    done
    echo "Countdown complete!"
}

# Main script
echo "=== Mock Test Script ==="
echo "This script simulates an interactive process for testing rule-agents."
echo ""

# 5-second countdown
countdown 5

# Interactive prompt
while true; do
    echo ""
    echo "Do you want to proceed?"
    echo "1) Continue with mission"
    echo "2) Cancel operation"
    echo "(Type '/exit' to exit the script)"
    echo -n "Enter your choice: "
    
    read -r choice
    
    case "$choice" in
        1)
            echo "Processing request..."
            for ((i=1; i<=3; i++)); do
                echo "Processing... ($i/3)"
                sleep 1
            done
            echo ""
            echo "=== MISSION COMPLETE ==="
            echo "The operation has been successfully completed!"
            exit 0
            ;;
        2)
            echo "Operation cancelled by user."
            exit 0
            ;;
        /exit)
            handle_exit
            ;;
        *)
            echo "Invalid input. Please enter 1, 2, or /exit"
            echo ""
            ;;
    esac
done