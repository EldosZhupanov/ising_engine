#!/usr/bin/env bash

echo "# Project Index"

echo
echo "## Cargo targets"
cargo metadata --no-deps --format-version=1 \
| jq -r '.packages[].targets[].name'

echo
echo "## Rust files"
fd . src -e rs

echo
echo "## Main structs"
rg "^pub struct|^struct" src

echo
echo "## Traits"
rg "^pub trait|^trait" src

echo
echo "## Enums"
rg "^pub enum|^enum" src

echo
echo "## Functions"
rg "^pub fn|^fn" src
