#!/usr/bin/env bash

echo
echo "======================================"
echo "TASK COMPLETE"
echo "======================================"

cargo check

echo

git status --short
