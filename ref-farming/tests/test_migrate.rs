use std::convert::TryFrom;

use near_sdk::json_types::ValidAccountId;
use near_sdk_sim::{deploy, init_simulator, to_yocto};

use ref_farming::ContractContract as Farming;

near_sdk_sim::lazy_static_include::lazy_static_include_bytes! {
    PREV_FARMING_WASM_BYTES => "../res/ref_farming_local.wasm",
    FARMING_WASM_BYTES => "../res/ref_farming_release.wasm",
}


#[test]
fn test_upgrade() {
    let root = init_simulator(None);
    let test_user = root.create_user("test".to_string(), to_yocto("100"));
    let farming = deploy!(
        contract: Farming,
        contract_id: "farming".to_string(),
        bytes: &PREV_FARMING_WASM_BYTES,
        signer_account: root,
        init_method: new(ValidAccountId::try_from(root.account_id.clone()).unwrap())
    );

    // Failed upgrade with no permissions.
    let result = test_user
        .call(
            farming.user_account.account_id.clone(),
            "upgrade",
            &PREV_FARMING_WASM_BYTES,
            near_sdk_sim::DEFAULT_GAS,
            0,
        )
        .status();
    assert!(format!("{:?}", result).contains("ERR_NOT_ALLOWED"));

    // Upgrade with calling migration. Should fail as currently migration not implemented
    root.call(
        farming.user_account.account_id.clone(),
        "upgrade",
        &FARMING_WASM_BYTES,
        near_sdk_sim::DEFAULT_GAS,
        0,
    )
    .assert_success();

    // Upgrade to the same code without migration is successful.
    root.call(
        farming.user_account.account_id.clone(),
        "upgrade",
        &FARMING_WASM_BYTES,
        near_sdk_sim::DEFAULT_GAS,
        0,
    )
    .assert_success();
}
