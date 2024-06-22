#/bin/bash
VER=1.69.0
rustup toolchain install $VER
rustup default $VER
rustup target add wasm32-unknown-unknown
cargo build -p ref-exchange --target wasm32-unknown-unknown --release
cargo build -p ref_farming --target wasm32-unknown-unknown --release
cargo install wasm-opt --locked --version 0.116.0
wasm-opt -Oz -o target/wasm32-unknown-unknown/release/ref_exchange_by_wasm_opt.wasm  target/wasm32-unknown-unknown/release/ref_exchange.wasm