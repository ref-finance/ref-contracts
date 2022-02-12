#/bin/bash
VER=2020-10-08
rustup toolchain install stable-$VER
rustup default stable-$VER
rustup target add wasm32-unknown-unknown
cargo build --target wasm32-unknown-unknown --release
