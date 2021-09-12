use std::convert::TryFrom;

use near_sdk::json_types::{U128, ValidAccountId};
use near_sdk_sim::{call, deploy, init_simulator, to_yocto};

use ref_exchange::ContractContract as Exchange;

near_sdk_sim::lazy_static_include::lazy_static_include_bytes! {
    PREV_EXCHANGE_WASM_BYTES => "../res/ref_exchange_101.wasm",
    EXCHANGE_WASM_BYTES => "../res/ref_exchange_release.wasm",
}

use crate::common::utils::*;
pub mod common;

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


#[test]
fn test_account_upgrade() {
    let (root, owner, pool, token1, _, _) = setup_old_pool_with_liquidity();

    let new_user = root.create_user("new_user".to_string(), to_yocto("100"));
    call!(
        new_user,
        token1.mint(to_va(new_user.account_id.clone()), U128(to_yocto("10")))
    )
    .assert_success();

    call!(
        new_user,
        pool.storage_deposit(None, None),
        deposit = to_yocto("1")
    )
    .assert_success();
    call!(
        new_user,
        token1.ft_transfer_call(to_va(swap()), to_yocto("5").into(), None, "".to_string()),
        deposit = 1
    )
    .assert_success();

    println!("before upgrade, version: {}", get_version(&pool));
    println!("before upgrade, num_of_pools: {}", get_num_of_pools(&pool));
    println!("before upgrade, pool0 shares: {}", get_pool(&pool, 0).shares_total_supply.0);
    let sb = get_storage_balance(&pool, new_user.valid_account_id()).unwrap();
    println!("before upgrade, t:{} a:{}", sb.total.0, sb.available.0);

    // Upgrade to the new version.
    owner.call(
        pool.user_account.account_id.clone(),
        "upgrade",
        &EXCHANGE_WASM_BYTES,
        near_sdk_sim::DEFAULT_GAS,
        0,
    )
    .assert_success();

    println!("after upgrade, version: {}", get_version(&pool));
    println!("after upgrade, num_of_pools: {}", get_num_of_pools(&pool));
    println!("after upgrade, pool0 shares: {}", get_pool(&pool, 0).shares_total_supply.0);

    let out_come = call!(
        new_user,
        token1.ft_transfer_call(to_va(swap()), to_yocto("5").into(), None, "".to_string()),
        deposit = 1
    );
    out_come.assert_success();
    // println!("{:#?}", out_come.promise_results());

    let ex_status = format!("{:?}", out_come.promise_errors()[0].as_ref().unwrap().status());
    println!("{}", ex_status);

    // let sb = get_storage_balance(&pool, new_user.valid_account_id()).unwrap();
    // println!("before upgrade, t:{} a:{}", sb.total.0, sb.available.0);

}