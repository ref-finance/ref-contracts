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
fn account_upgrade_enough_storage() {
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

    assert_eq!(get_version(&pool), String::from("1.0.1"));
    assert_eq!(get_deposits(&pool, 
        new_user.valid_account_id())
        .get(&token1.account_id()).unwrap().0, to_yocto("5"));

    // Upgrade to the new version.
    owner.call(
        pool.user_account.account_id.clone(),
        "upgrade",
        &EXCHANGE_WASM_BYTES,
        near_sdk_sim::DEFAULT_GAS,
        0,
    )
    .assert_success();

    assert_eq!(get_version(&pool), String::from("1.1.0"));
    
    // println!("after upgrade, version: {}", get_version(&pool));
    // println!("after upgrade, num_of_pools: {}", get_num_of_pools(&pool));
    // println!("after upgrade, pool0 shares: {}", get_pool(&pool, 0).shares_total_supply.0);

    let out_come = call!(
        new_user,
        token1.ft_transfer_call(to_va(swap()), to_yocto("5").into(), None, "".to_string()),
        deposit = 1
    );
    out_come.assert_success();
    assert_eq!(get_error_count(&out_come), 0);

    assert_eq!(get_deposits(&pool, 
        new_user.valid_account_id())
        .get(&token1.account_id()).unwrap().0, to_yocto("10"));

    // println!("{:#?}", out_come.promise_results());
    // assert_eq!(balance_of(&token1, &new_user.account_id()), to_yocto("0"));

    // let sb = get_storage_balance(&pool, new_user.valid_account_id()).unwrap();
    // println!("after upgrade, t:{} a:{}", sb.total.0, sb.available.0);

}

#[test]
fn account_upgrade_not_enough_storage() {
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
        deposit = to_yocto("0.00182")
    )
    .assert_success();
    // 0.00098 Near
    let sb = get_storage_balance(&pool, new_user.valid_account_id()).unwrap();
    println!("Old min, t:{} a:{}", sb.total.0, sb.available.0);

    call!(
        new_user,
        token1.ft_transfer_call(to_va(swap()), to_yocto("5").into(), None, "".to_string()),
        deposit = 1
    )
    .assert_success();

    // 0.00182 Near
    let sb = get_storage_balance(&pool, new_user.valid_account_id()).unwrap();
    println!("Old with one token, t:{} a:{}", sb.total.0, sb.available.0);

    // Upgrade to the new version.
    owner.call(
        pool.user_account.account_id.clone(),
        "upgrade",
        &EXCHANGE_WASM_BYTES,
        near_sdk_sim::DEFAULT_GAS,
        0,
    )
    .assert_success();

    assert_eq!(get_version(&pool), String::from("1.1.0"));
    let ss = get_storage_state(&pool, new_user.valid_account_id()).unwrap();
    println!("New with one token, d:{} u:{}", ss.deposit.0, ss.usage.0);

    // deposit would fail
    let out_come = call!(
        new_user,
        token1.ft_transfer_call(to_va(swap()), to_yocto("5").into(), None, "".to_string()),
        deposit = 1
    );
    out_come.assert_success();
    let ex_status = format!("{:?}", out_come.promise_errors()[0].as_ref().unwrap().status());
    assert!(ex_status.contains("E11: insufficient $NEAR storage deposit"));

    assert_eq!(get_deposits(&pool, 
        new_user.valid_account_id())
        .get(&token1.account_id()).unwrap().0, to_yocto("5"));
    
    // TODO: instant swap would ?


}

#[test]
fn account_upgrade_view_before_modify() {
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

    // Upgrade to the new version.
    owner.call(
        pool.user_account.account_id.clone(),
        "upgrade",
        &EXCHANGE_WASM_BYTES,
        near_sdk_sim::DEFAULT_GAS,
        0,
    )
    .assert_success();

    assert_eq!(get_deposits(&pool, 
        new_user.valid_account_id())
        .get(&token1.account_id()).unwrap().0, to_yocto("5"));
    
    let sb = get_storage_balance(&pool, new_user.valid_account_id()).unwrap();
    println!("after upgrade, t:{} a:{}", sb.total.0, sb.available.0);

    // let vr = view!(pool.storage_balance_of(new_user.valid_account_id()));
    // println!("{}", vr.unwrap_err());
    // assert!(format!("{}", vr.unwrap_err()).contains("ProhibitedInView { method_name: \"storage_write\" }"));
}