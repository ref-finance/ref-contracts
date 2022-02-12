NEAR_BUILDER_IMAGE=nearprotocol/contract-builder

FARMING_DIR=ref-farming
FARMING_BUILDER_NAME=build_ref_farming
FARMING_RUSTUP_SETUP="rustup toolchain install stable-2020-10-08;rustup default stable-2020-10-08;rustup target add wasm32-unknown-unknown; cargo build --target wasm32-unknown-unknown --release"
FARMING_RELEASE=ref_farming

EXCHANGE_DIR=ref-exchange
EXCHANGE_BUILDER_NAME=build_ref_exchange
EXCHANGE_RUSTUP_SETUP="rustup toolchain install stable-2021-11-01; rustup default stable-2021-11-01; rustup target add wasm32-unknown-unknown; cargo build --target wasm32-unknown-unknown --release"
EXCHANGE_RELEASE=ref_exchange

define create_builder 
	docker ps -a | grep $(1) || docker create \
     --mount type=bind,source=${PWD},target=/host \
     --cap-add=SYS_PTRACE --security-opt seccomp=unconfined \
     --name=$(1) \
     -w /host/$(2) \
     -e RUSTFLAGS='-C link-arg=-s' \
     -it \
     ${NEAR_BUILDER_IMAGE} \
     /bin/bash
endef

define start_builder
	docker ps | grep $(1) || docker start $(1) 
endef

define setup_builder
	docker exec $(1) /bin/bash -c $(2)
endef

define remove_builder
	docker stop $(1) && docker rm $(1) 
endef

define release_wasm
	cp ${PWD}/target/wasm32-unknown-unknown/release/$(1).wasm ${PWD}/res/$(1)_release.wasm
endef

res:
	mkdir -p res

build-farming:
	$(call create_builder,${FARMING_BUILDER_NAME},${FARMING_DIR} )
	$(call start_builder,${FARMING_BUILDER_NAME}) 
	$(call setup_builder,${FARMING_BUILDER_NAME},${FARMING_RUSTUP_SETUP}) 
	
test-farming: build-farming
	docker exec ${FARMING_BUILDER_NAME} cargo test 

release-farming: res
	$(call release_wasm,${FARMING_RELEASE})


build-exchange:
	$(call create_builder,${EXCHANGE_BUILDER_NAME},${EXCHANGE_DIR} )
	$(call start_builder,${EXCHANGE_BUILDER_NAME}) 
	$(call setup_builder,${EXCHANGE_BUILDER_NAME},${EXCHANGE_RUSTUP_SETUP}) 
	
test-exchange: build-exchange
	docker exec ${EXCHANGE_BUILDER_NAME} cargo test 

release-exchange: res
	$(call release_wasm,${EXCHANGE_RELEASE})

remove-builders:
	$(call remove_builder,${FARMING_BUILDER_NAME})
	$(call remove_builder,${EXCHANGE_BUILDER_NAME})
