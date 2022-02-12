NEAR_CONTRACT_BUILDER_IMAGE=nearprotocol/contract-builder

FARMING_DIR=ref-farming
FARMING_BUILDER_NAME=build_ref_farming
FARMING_RELEASE=ref_farming

EXCHANGE_DIR=ref-exchange
EXCHANGE_BUILDER_NAME=build_ref_exchange
EXCHANGE_RELEASE=ref_exchange

test-all: test-exchange test-farming

build-farming:
	$(call create_builder,${FARMING_BUILDER_NAME},${FARMING_DIR})
	$(call start_builder,${FARMING_BUILDER_NAME})
	$(call setup_builder,${FARMING_BUILDER_NAME})
	
test-farming: build-farming
	docker exec ${FARMING_BUILDER_NAME} cargo test 

build-exchange:
	$(call create_builder,${EXCHANGE_BUILDER_NAME},${EXCHANGE_DIR})
	$(call start_builder,${EXCHANGE_BUILDER_NAME})
	$(call setup_builder,${EXCHANGE_BUILDER_NAME})
	
test-exchange: build-exchange
	docker exec ${EXCHANGE_BUILDER_NAME} cargo test 

res:
	mkdir -p res

release-farming: res
	$(call release_wasm,${FARMING_RELEASE})

release-exchange: res
	$(call release_wasm,${EXCHANGE_RELEASE})

remove-builders:
	$(call remove_builder,${FARMING_BUILDER_NAME}) || \
	$(call remove_builder,${EXCHANGE_BUILDER_NAME})

define create_builder 
	docker ps -a | grep $(1) || docker create \
     --mount type=bind,source=${PWD},target=/host \
     --cap-add=SYS_PTRACE --security-opt seccomp=unconfined \
     --name=$(1) \
     -w /host/$(2) \
     -e RUSTFLAGS='-C link-arg=-s' \
     -it \
     ${NEAR_CONTRACT_BUILDER_IMAGE} \
     /bin/bash
endef

define start_builder
	docker ps | grep $(1) || docker start $(1) 
endef

define setup_builder
	docker exec $(1) /bin/bash rust_setup.sh 
endef

define remove_builder
	docker stop $(1) && docker rm $(1) 
endef

define release_wasm
	cp ${PWD}/target/wasm32-unknown-unknown/release/$(1).wasm ${PWD}/res/$(1)_release.wasm
endef
