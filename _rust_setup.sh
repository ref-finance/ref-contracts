#/bin/bash
VER=1.69.0
rustup toolchain install $VER
rustup default $VER
rustup target add wasm32-unknown-unknown
cargo build -p ref-exchange --target wasm32-unknown-unknown --release
cargo build -p ref_farming --target wasm32-unknown-unknown --release