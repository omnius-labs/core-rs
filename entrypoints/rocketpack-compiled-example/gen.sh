#!/usr/bin/env bash
set -euo pipefail

cargo run --manifest-path ../rocketpack-compiler/Cargo.toml -- compile ./
cargo run --manifest-path ../rocketpack-compiler/Cargo.toml -- compile ./external/provider
cargo run --manifest-path ../rocketpack-compiler/Cargo.toml -- compile ./external/consumer
cargo fmt --manifest-path rust/Cargo.toml --all
