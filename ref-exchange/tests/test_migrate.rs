use std::convert::TryFrom;

use near_sdk::borsh::{self, BorshSerialize};
use near_sdk::json_types::ValidAccountId;
use near_sdk_sim::{deploy, init_simulator, to_yocto};

use ref_exchange::ContractContract as Exchange;

near_sdk_sim::lazy_static_include::lazy_static_include_bytes! {
    PREV_EXCHANGE_WASM_BYTES => "../res/ref_exchange_local.wasm",
    EXCHANGE_WASM_BYTES => "../res/ref_exchange_local.wasm",
}

#[derive(BorshSerialize)]
struct UpgradeArgs {
    code: Vec<u8>,
    migrate: bool,
}

#[test]
fn test_upgrade() {
    let root = init_simulator(None);
    let test_user = root.create_user("test".to_string(), to_yocto("100"));
    let pool = deploy!(
        contract: Exchange,
        contract_id: "swap".to_string(),
        bytes: &PREV_EXCHANGE_WASM_BYTES,
        signer_account: root,
        init_method: new(ValidAccountId::try_from(root.account_id.clone()).unwrap(), 4, 1)
    );
    // Failed upgrade with no permissions.
    let result = test_user
        .call(
            pool.user_account.account_id.clone(),
            "upgrade",
            &EXCHANGE_WASM_BYTES,
            near_sdk_sim::DEFAULT_GAS,
            0,
        )
        .status();
    assert!(format!("{:?}", result).contains("ERR_NOT_ALLOWED"));

    root.call(
        pool.user_account.account_id.clone(),
        "upgrade",
        &EXCHANGE_WASM_BYTES,
        near_sdk_sim::DEFAULT_GAS,
        0,
    )
    .assert_success();

    // Upgrade to the same code migration is skipped.
    root.call(
        pool.user_account.account_id.clone(),
        "upgrade",
        &EXCHANGE_WASM_BYTES,
        near_sdk_sim::DEFAULT_GAS,
        0,
    )
    .assert_success();
}
