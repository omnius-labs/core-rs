#!/usr/bin/env bash
set -euo pipefail

cargo run --manifest-path ../rocketpack-compiler/Cargo.toml -- compile ./
