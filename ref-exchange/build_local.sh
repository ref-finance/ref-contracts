#!/bin/bash
set -e

RUSTFLAGS='-C link-arg=-s' cargo +stable build --target wasm32-unknown-unknown --release
cd ..
cp target/wasm32-unknown-unknown/release/ref_exchange.wasm ./res/ref_exchange_local.wasm
