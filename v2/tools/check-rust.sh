#!/bin/sh
set -eu
unset SUPERCMD_EXTENSION_PATHS
cd "$(dirname "$0")/.."
python3 tools/prepare_harness.py
for chain in stable 1.85.0; do
  cargo +"$chain" fmt -- --check
  rustup run "$chain" rustfmt --edition 2021 --config skip_children=true --check adapter.rs harness/src/lib.rs harness/tests.rs harness/tauri-stub/src/lib.rs
  for manifest in Cargo.toml .work/harness/Cargo.toml; do
    cargo +"$chain" test --locked --offline --manifest-path "$manifest"
    cargo +"$chain" clippy --locked --offline --all-targets --manifest-path "$manifest" -- -D warnings
    cargo +"$chain" check --locked --offline --all-targets --target x86_64-pc-windows-gnu --manifest-path "$manifest"
    cargo +"$chain" clippy --locked --offline --all-targets --target x86_64-pc-windows-gnu --manifest-path "$manifest" -- -D warnings
  done
done
