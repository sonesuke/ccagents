#!/bin/bash

# Generate some items including duplicates to demonstrate deduplication
echo "item-001"
echo "item-002"
echo "item-003"
echo "item-001"  # duplicate
echo "item-004"
echo "item-002"  # duplicate
echo "item-005"