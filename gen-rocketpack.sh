#!/usr/bin/env bash
set -euo pipefail

cd -- "$(dirname -- "$0")"
cargo run -p omnius-core-rocketpack-compiler -- compile ./
