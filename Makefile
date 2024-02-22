RFLAGS="-C link-arg=-s"

build: build-exchange

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

unittest: build-exchange
ifdef TC
	RUSTFLAGS=$(RFLAGS) cargo test $(TC) -p ref-exchange --lib -- --nocapture
else
	RUSTFLAGS=$(RFLAGS) cargo test -p ref-exchange --lib -- --nocapture
endif

test: build-exchange mock-ft mock-rated mock-farming
ifdef TF
	RUSTFLAGS=$(RFLAGS) cargo test -p ref-exchange --test $(TF) -- --nocapture
else
	RUSTFLAGS=$(RFLAGS) cargo test -p ref-exchange --tests
endif

test-exchange: build-exchange mock-ft mock-rated mock-boost-farming mock-wnear
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

mock-rated: test-rated-token
	rustup target add wasm32-unknown-unknown
	RUSTFLAGS=$(RFLAGS) cargo build -p test-rated-token --target wasm32-unknown-unknown --release
	mkdir -p res
	cp target/wasm32-unknown-unknown/release/test_rated_token.wasm ./res/test_rated_token.wasm

mock-farming: mock-boost-farming
	rustup target add wasm32-unknown-unknown
	RUSTFLAGS=$(RFLAGS) cargo build -p mock-boost-farming --target wasm32-unknown-unknown --release
	mkdir -p res
	cp target/wasm32-unknown-unknown/release/mock_boost_farming.wasm ./res/mock_boost_farming.wasm

mock-wnear:
	rustup target add wasm32-unknown-unknown
	RUSTFLAGS=$(RFLAGS) cargo build -p mock-wnear --target wasm32-unknown-unknown --release
	mkdir -p res
	cp target/wasm32-unknown-unknown/release/mock_wnear.wasm ./res/mock_wnear.wasm

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
