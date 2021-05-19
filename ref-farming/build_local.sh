#!/bin/bash
set -e
rustup target add wasm32-unknown-unknown
RUSTFLAGS='-C link-arg=-s' cargo build --target wasm32-unknown-unknown --release
# RUSTFLAGS='-C link-arg=-s' cargo +stable build --target wasm32-unknown-unknown --release
cd ..
cp target/wasm32-unknown-unknown/release/ref_farming.wasm ./res/ref_farming_local.wasm
