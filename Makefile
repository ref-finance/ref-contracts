RFLAGS="-C link-arg=-s"

build: ref-exchange 
	rustup target add wasm32-unknown-unknown
	RUSTFLAGS=$(RFLAGS) cargo build -p ref-exchange --target wasm32-unknown-unknown --release
	RUSTFLAGS=$(RFLAGS) cargo build -p ref_farming --target wasm32-unknown-unknown --release
	mkdir -p res
	cp target/wasm32-unknown-unknown/release/ref_exchange.wasm ./res/ref_exchange.wasm
	cp target/wasm32-unknown-unknown/release/ref_farming.wasm ./res/ref_farming.wasm

test: build mock-ft
	RUSTFLAGS=$(RFLAGS) cargo test -p ref-exchange 

test-farm: build mock-ft
	RUSTFLAGS=$(RFLAGS) cargo test -p ref_farming 

test-release: mock-ft
	mkdir -p res
	cp ./releases/ref_exchange_release.wasm ./res/ref_exchange.wasm
	RUSTFLAGS=$(RFLAGS) cargo test -p ref-exchange 

mock-ft: test-token
	rustup target add wasm32-unknown-unknown
	RUSTFLAGS=$(RFLAGS) cargo build -p test-token --target wasm32-unknown-unknown --release
	mkdir -p res
	cp target/wasm32-unknown-unknown/release/test_token.wasm ./res/test_token.wasm

release:
	$(call docker_build,_rust_setup.sh)
	mkdir -p res
	cp target/wasm32-unknown-unknown/release/ref_exchange.wasm res/ref_exchange_release.wasm
	cp target/wasm32-unknown-unknown/release/ref_farming.wasm res/ref_farming_release.wasm

clean:
	cargo clean
	rm -rf res/

define docker_build
	docker build -t my-contract-builder .
	docker run \
		--mount type=bind,source=${PWD},target=/host \
		--cap-add=SYS_PTRACE --security-opt seccomp=unconfined \
		-w /host \
		-e RUSTFLAGS=$(RFLAGS) \
		-i -t my-contract-builder \
		/bin/bash $(1)
endef
