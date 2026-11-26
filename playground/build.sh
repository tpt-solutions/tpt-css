#!/bin/sh
# Builds the playground module and copies it next to the page.
# Needs the wasm32-unknown-unknown target: rustup target add wasm32-unknown-unknown
set -e
cd "$(dirname "$0")"
cargo build --release --target wasm32-unknown-unknown
cp target/wasm32-unknown-unknown/release/tpt_css_playground.wasm playground.wasm
echo "built playground.wasm"
