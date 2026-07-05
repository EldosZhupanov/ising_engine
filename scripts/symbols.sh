#!/usr/bin/env bash

echo "# Public Structs"
rg "^pub struct" src

echo
echo "# Public Enums"
rg "^pub enum" src

echo
echo "# Public Traits"
rg "^pub trait" src

echo
echo "# Public Functions"
rg "^pub fn" src
