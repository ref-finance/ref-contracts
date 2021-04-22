#!/bin/bash
set -e

RUSTFLAGS='-C link-arg=-s' cargo build --target wasm32-unknown-unknown --release
cd ..
cp target/wasm32-unknown-unknown/release/ref_farming.wasm ./res/

