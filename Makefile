RFLAGS="-C link-arg=-s"

build: build-exchange build-farm

build-exchange: ref-exchange
	rustup target add wasm32-unknown-unknown
	RUSTFLAGS=$(RFLAGS) cargo build -p ref-exchange --target wasm32-unknown-unknown --release
	mkdir -p res
	cp target/wasm32-unknown-unknown/release/ref_exchange.wasm ./res/ref_exchange.wasm

build-farm: ref-farming
	rustup target add wasm32-unknown-unknown
	RUSTFLAGS=$(RFLAGS) cargo build -p ref_farming --target wasm32-unknown-unknown --release
	mkdir -p res
	cp target/wasm32-unknown-unknown/release/ref_farming.wasm ./res/ref_farming.wasm

test: test-exchange test-farm

test-exchange: build-exchange mock-ft
	RUSTFLAGS=$(RFLAGS) cargo test -p ref-exchange 

test-farm: build-farm mock-ft
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
