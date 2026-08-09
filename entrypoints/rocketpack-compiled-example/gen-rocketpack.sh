#!/usr/bin/env bash
set -euo pipefail

cargo run --manifest-path ../rocketpack-compiler/Cargo.toml -- compile ./showcase
cargo run --manifest-path ../rocketpack-compiler/Cargo.toml -- compile ./provider
cargo run --manifest-path ../rocketpack-compiler/Cargo.toml -- compile ./consumer
cargo fmt --manifest-path showcase/rust/Cargo.toml --all
