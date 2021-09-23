use near_sdk::json_types::U128;
use near_sdk_sim::{call, to_yocto, ContractAccount, UserAccount};

use ref_exchange::{ContractContract as Exchange, SwapAction};
use test_token::ContractContract as TestToken;

near_sdk_sim::lazy_static_include::lazy_static_include_bytes! {
    EXCHANGE_WASM_BYTES => "../res/ref_exchange_release.wasm",
}

use crate::common::utils::*;
pub mod common;

/// prepare user with three kinds of token deposited and excact storage
fn prepare_old_user() -> (
    UserAccount,
    UserAccount,
    ContractAccount<Exchange>,
    ContractAccount<TestToken>,
    ContractAccount<TestToken>,
    ContractAccount<TestToken>,
    UserAccount,
) {
    let (root, owner, pool, token1, token2, token3) = setup_old_pool_with_liquidity();
    let user = root.create_user("new_user".to_string(), to_yocto("100"));
    call!(
        user,
        token1.mint(to_va(user.account_id.clone()), U128(to_yocto("10")))
    )
    .assert_success();
    call!(
        user,
        token2.mint(to_va(user.account_id.clone()), U128(to_yocto("10")))
    )
    .assert_success();
    call!(
        user,
        token3.mint(to_va(user.account_id.clone()), U128(to_yocto("10")))
    )
    .assert_success();

    call!(user, pool.storage_deposit(None, None), deposit = to_yocto("1"))
    .assert_success();

    call!(
        user,
        token1.ft_transfer_call(to_va(swap()), to_yocto("5").into(), None, "".to_string()),
        deposit = 1
    )
    .assert_success();

    call!(
        user,
        token2.ft_transfer_call(to_va(swap()), to_yocto("5").into(), None, "".to_string()),
        deposit = 1
    )
    .assert_success();

    call!(
        user,
        token3.ft_transfer_call(to_va(swap()), to_yocto("5").into(), None, "".to_string()),
        deposit = 1
    )
    .assert_success();

    call!(
        user,
        pool.storage_withdraw(None),
        deposit = 1
    )
    .assert_success();

    assert_eq!(get_version(&pool), String::from("1.0.2"));
    assert_eq!(
        get_deposits(&pool, user.valid_account_id())
            .get(&token1.account_id()).unwrap().0, 
        to_yocto("5")
    );
    (root, owner, pool, token1, token2, token3, user)
}

#[test]
fn account_upgrade_enough_storage() {
    let (_, owner, pool, token1, token2, token3, user) = prepare_old_user();

    // Upgrade to the new version.
    owner.call(
        pool.user_account.account_id.clone(),
        "upgrade",
        &EXCHANGE_WASM_BYTES,
        near_sdk_sim::DEFAULT_GAS,
        0,
    )
    .assert_success();

    assert_eq!(get_version(&pool), String::from("1.2.0"));

    let ss = get_storage_state(&pool, user.valid_account_id()).unwrap();
    assert_eq!(ss.deposit.0, to_yocto("0.00350"));
    assert_eq!(ss.usage.0, to_yocto("0.00354"));
    let sb = get_storage_balance(&pool, user.valid_account_id()).unwrap();
    assert_eq!(sb.total.0, to_yocto("0.00350"));
    assert_eq!(sb.available.0, 0);

    call!(
        user,
        pool.storage_deposit(None, None),
        deposit = to_yocto("0.00068")
    )
    .assert_success();
    // deposit would OK
    let out_come = call!(
        user,
        token1.ft_transfer_call(to_va(swap()), to_yocto("5").into(), None, "".to_string()),
        deposit = 1
    );
    out_come.assert_success();
    // println!("{:#?}", out_come.promise_results());
    assert_eq!(get_error_count(&out_come), 0);

    let ss = get_storage_state(&pool, user.valid_account_id()).unwrap();
    assert_eq!(ss.deposit.0, to_yocto("0.00418"));
    assert_eq!(ss.usage.0, to_yocto("0.00418"));
    let sb = get_storage_balance(&pool, user.valid_account_id()).unwrap();
    assert_eq!(sb.total.0, to_yocto("0.00418"));
    assert_eq!(sb.available.0, 0);

    assert_eq!(get_deposits(&pool, 
        user.valid_account_id())
        .get(&token1.account_id()).unwrap().0, to_yocto("10"));

    call!(
        user,
        pool.storage_deposit(None, None),
        deposit = to_yocto("0.00064")
    )
    .assert_success();
    // swap would OK
    let out_come = call!(
        user,
        pool.swap(
            vec![SwapAction {
                pool_id: 0,
                token_in: token1.account_id(),
                amount_in: Some(U128(to_yocto("1"))),
                token_out: token2.account_id(),
                min_amount_out: U128(1)
            }],
            None
        ),
        deposit = 1
    );
    out_come.assert_success();
    assert_eq!(get_error_count(&out_come), 0);

    let ss = get_storage_state(&pool, user.valid_account_id()).unwrap();
    assert_eq!(ss.deposit.0, to_yocto("0.00482"));
    assert_eq!(ss.usage.0, to_yocto("0.00482"));
    let sb = get_storage_balance(&pool, user.valid_account_id()).unwrap();
    assert_eq!(sb.total.0, to_yocto("0.00482"));
    assert_eq!(sb.available.0, 0);

    assert_eq!(get_deposits(&pool, 
        user.valid_account_id())
        .get(&token1.account_id()).unwrap().0, to_yocto("9"));
    assert!(get_deposits(&pool, 
        user.valid_account_id())
        .get(&token2.account_id()).unwrap().0 > to_yocto("6.8"));

    call!(
        user,
        pool.storage_deposit(None, None),
        deposit = to_yocto("0.00064")
    )
    .assert_success();
    // withdraw would OK
    let out_come = call!(
        user,
        pool.withdraw(token3.valid_account_id(), to_yocto("1").into(), None),
        deposit = 1
    );
    out_come.assert_success();
    assert_eq!(get_error_count(&out_come), 0);

    let ss = get_storage_state(&pool, user.valid_account_id()).unwrap();
    assert_eq!(ss.deposit.0, to_yocto("0.00546"));
    assert_eq!(ss.usage.0, to_yocto("0.00546"));
    let sb = get_storage_balance(&pool, user.valid_account_id()).unwrap();
    assert_eq!(sb.total.0, to_yocto("0.00546"));
    assert_eq!(sb.available.0, 0);

    assert_eq!(get_deposits(&pool, 
        user.valid_account_id())
        .get(&token3.account_id()).unwrap().0, to_yocto("4"));
}

#[test]
fn account_upgrade_not_enough_storage() {

    let (_, owner, pool, token1, token2, _, user) = prepare_old_user();

    // Upgrade to the new version.
    owner.call(
        pool.user_account.account_id.clone(),
        "upgrade",
        &EXCHANGE_WASM_BYTES,
        near_sdk_sim::DEFAULT_GAS,
        0,
    )
    .assert_success();
    assert_eq!(get_version(&pool), String::from("1.2.0"));

    // deposit would fail
    let out_come = call!(
        user,
        token1.ft_transfer_call(to_va(swap()), to_yocto("5").into(), None, "".to_string()),
        deposit = 1
    );
    out_come.assert_success();
    let ex_status = format!("{:?}", out_come.promise_errors()[0].as_ref().unwrap().status());
    assert!(ex_status.contains("E11: insufficient $NEAR storage deposit"));

    assert_eq!(get_deposits(&pool, 
        user.valid_account_id())
        .get(&token1.account_id()).unwrap().0, to_yocto("5"));
    
    // withdraw would fail
    let out_come = call!(
        user,
        pool.withdraw(token2.valid_account_id(), to_yocto("1").into(), None),
        deposit = 1
    );
    assert!(!out_come.is_ok());
    let ex_status = format!("{:?}", out_come.promise_errors()[0].as_ref().unwrap().status());
    assert!(ex_status.contains("E11: insufficient $NEAR storage deposit"));

    assert_eq!(get_deposits(&pool, 
        user.valid_account_id())
        .get(&token2.account_id()).unwrap().0, to_yocto("5"));

    // swap would fail
    let out_come = call!(
        user,
        pool.swap(
            vec![SwapAction {
                pool_id: 0,
                token_in: token1.account_id(),
                amount_in: Some(U128(to_yocto("1"))),
                token_out: token2.account_id(),
                min_amount_out: U128(1)
            }],
            None
        ),
        deposit = 1
    );
    assert!(!out_come.is_ok());
    let ex_status = format!("{:?}", out_come.promise_errors()[0].as_ref().unwrap().status());
    assert!(ex_status.contains("E11: insufficient $NEAR storage deposit"));

    assert_eq!(get_deposits(&pool, 
        user.valid_account_id())
        .get(&token1.account_id()).unwrap().0, to_yocto("5"));
    
    assert_eq!(get_deposits(&pool, 
        user.valid_account_id())
        .get(&token2.account_id()).unwrap().0, to_yocto("5"));
}

#[test]
fn account_upgrade_view_before_modify() {
    let (_, owner, pool, token1, _, _, user) = prepare_old_user();
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
        user.valid_account_id())
        .get(&token1.account_id()).unwrap().0, to_yocto("5"));
} 
