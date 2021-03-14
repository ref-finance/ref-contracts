use std::convert::TryFrom;

use near_sdk::borsh::{self, BorshSerialize};
use near_sdk::json_types::ValidAccountId;
use near_sdk::PendingContractTx;
use near_sdk_sim::{deploy, init_simulator, to_yocto};

use ref_exchange::ContractContract as Exchange;

near_sdk_sim::lazy_static_include::lazy_static_include_bytes! {
    EXCHANGE_WASM_BYTES => "../res/ref_exchange.wasm",
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
        bytes: &EXCHANGE_WASM_BYTES,
        signer_account: root,
        init_method: new(ValidAccountId::try_from(root.account_id.clone()).unwrap(), 4, 1)
    );
    let args = UpgradeArgs {
        code: EXCHANGE_WASM_BYTES.to_vec(),
        migrate: false,
    };
    // Successful upgrade without migration.
    root.call(
        PendingContractTx {
            receiver_id: pool.user_account.account_id.clone(),
            method: "upgrade".to_string(),
            args: args.try_to_vec().unwrap(),
            is_view: false,
        },
        to_yocto("0"),
        near_sdk_sim::DEFAULT_GAS,
    )
    .assert_success();
    // Failed upgrade with no permissions.
    let result = test_user
        .call(
            PendingContractTx {
                receiver_id: pool.user_account.account_id.clone(),
                method: "upgrade".to_string(),
                args: args.try_to_vec().unwrap(),
                is_view: false,
            },
            to_yocto("0"),
            near_sdk_sim::DEFAULT_GAS,
        )
        .status();
    assert!(format!("{:?}", result).contains("ERR_NOT_ALLOWED"));
    // Upgrade with calling migration. Should fail as currently migration not implemented
    // TODO: when migration will be added, change this test.
    let args = UpgradeArgs {
        code: EXCHANGE_WASM_BYTES.to_vec(),
        migrate: true,
    };
    let result = root.call(
        PendingContractTx {
            receiver_id: pool.user_account.account_id.clone(),
            method: "upgrade".to_string(),
            args: args.try_to_vec().unwrap(),
            is_view: false,
        },
        to_yocto("0"),
        near_sdk_sim::DEFAULT_GAS,
    );
    assert!(format!("{:?}", result).contains("not implemented"));
}
