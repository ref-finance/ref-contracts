use std::convert::TryFrom;

use near_sdk::serde_json::{json};
use near_sdk::json_types::ValidAccountId;
use near_sdk_sim::{deploy, init_simulator, to_yocto, view};

use ref_farming::ContractContract as Farming;

near_sdk_sim::lazy_static_include::lazy_static_include_bytes! {
    PREV_FARMING_WASM_BYTES => "../res/ref_farming_v0104.wasm",
    FARMING_WASM_BYTES => "../res/ref_farming.wasm",
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

    root.call(
        farming.user_account.account_id.clone(),
        "upgrade",
        &FARMING_WASM_BYTES,
        near_sdk_sim::DEFAULT_GAS,
        0,
    )
    .assert_success();

    let metadata = view!(farming.get_metadata()).unwrap_json_value();
    assert_eq!("1.1.0".to_string(), *metadata.get("version").unwrap());
    assert_eq!("Running".to_string(), *metadata.get("state").unwrap());

    root.call(
        farming.user_account.account_id.clone(),
        "pause_contract",
        &json!({})
        .to_string()
        .into_bytes(),
        near_sdk_sim::DEFAULT_GAS,
        1,
    )
    .assert_success(); 

    let metadata = view!(farming.get_metadata()).unwrap_json_value();
    assert_eq!("Paused".to_string(), *metadata.get("state").unwrap());

    let out_come = root.call(
        farming.user_account.account_id.clone(),
        "claim_reward_by_farm",
        &json!({
            "farm_id": "farm_id",
        })
        .to_string()
        .into_bytes(),
        near_sdk_sim::DEFAULT_GAS,
        0,
    ); 
    let ex_status = format!("{:?}", out_come.promise_errors()[0].as_ref().unwrap().status());
    assert!(ex_status.contains("E600: contract paused"));

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
