#!/usr/bin/env bash

echo "# Git Summary"
echo

git status --short

echo
echo "--------------------------------"
echo

git diff --stat

echo
echo "--------------------------------"
echo

git diff --name-only
