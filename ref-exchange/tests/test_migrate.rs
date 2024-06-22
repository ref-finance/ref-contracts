use std::convert::TryFrom;

use near_sdk::json_types::{ValidAccountId};
use near_sdk_sim::{deploy, init_simulator, to_yocto};

use ref_exchange::{ContractContract as Exchange, RunningState};

use crate::common::utils::*;
pub mod common;

near_sdk_sim::lazy_static_include::lazy_static_include_bytes! {
    PREV_EXCHANGE_WASM_BYTES => "../releases/ref_exchange_release_v192.wasm",
    EXCHANGE_WASM_BYTES => "../res/ref_exchange.wasm",
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
        init_method: new(ValidAccountId::try_from(root.account_id.clone()).unwrap(),
                ValidAccountId::try_from("boost_farm".to_string()).unwrap(),
                ValidAccountId::try_from("burrowland".to_string()).unwrap(), 4, 1)
    );
    let metadata = get_metadata(&pool);
    assert_eq!(metadata.version, "1.9.2".to_string());
    
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
    assert!(format!("{:?}", result).contains("E100"));

    root.call(
        pool.user_account.account_id.clone(),
        "upgrade",
        &EXCHANGE_WASM_BYTES,
        near_sdk_sim::DEFAULT_GAS,
        0,
    )
    .assert_success();
    let metadata = get_metadata(&pool);
    // println!("{:#?}", metadata);
    assert_eq!(metadata.version, "1.9.3".to_string());
    assert_eq!(metadata.admin_fee_bps, 5);
    assert_eq!(metadata.boost_farm_id, "boost_farm".to_string());
    assert_eq!(metadata.burrowland_id, "burrowland".to_string());
    assert_eq!(metadata.state, RunningState::Running);

    // Upgrade to the same code with insurfficient gas.
    let result = root
        .call(
            pool.user_account.account_id.clone(),
            "upgrade",
            &EXCHANGE_WASM_BYTES,
            70_000_000_000_000_u64,
            0,
        )
        .status();
    assert!(format!("{:?}", result).contains("Not enough gas to complete state migration"));

    // Upgrade to the same code migration is skipped.
    root.call(
        pool.user_account.account_id.clone(),
        "upgrade",
        &EXCHANGE_WASM_BYTES,
        100_000_000_000_000_u64,
        0,
    )
    .assert_success();
}