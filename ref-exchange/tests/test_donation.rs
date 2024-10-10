use std::collections::HashMap;
use near_sdk::json_types::U128;
use near_sdk::AccountId;
use near_sdk_sim::{call, to_yocto, view};

use crate::common::utils::*;
pub mod common;

#[test]
fn donation_share() {
    let (root, _owner, pool, _token1, _token2, _token3) = setup_pool_with_liquidity();
    assert_eq!(mft_balance_of(&pool, ":0", &root.account_id()), to_yocto("1"));
    assert_eq!(mft_balance_of(&pool, ":0", &pool.account_id()), 0);
    assert_eq!(mft_total_supply(&pool, ":0"), to_yocto("1"));
    let deposit_before_donation_share = get_storage_state(&pool, to_va(root.account_id.clone())).unwrap().deposit;
    call!(
        root,
        pool.donation_share(
            0, None, None
        ),
        deposit = 1
    ).assert_success();
    assert_eq!(deposit_before_donation_share, get_storage_state(&pool, to_va(root.account_id.clone())).unwrap().deposit);
    assert_eq!(mft_balance_of(&pool, ":0", &root.account_id()), 0);
    assert_eq!(mft_balance_of(&pool, ":0", &pool.account_id()), to_yocto("1"));
    assert_eq!(mft_total_supply(&pool, ":0"), to_yocto("1"));

    assert!(mft_has_registered(&pool, ":0", root.valid_account_id()));
    call!(
        root,
        pool.mft_unregister(":0".to_string()),
        deposit = 1
    ).assert_success();
    assert!(!mft_has_registered(&pool, ":0", root.valid_account_id()));

    call!(
        root,
        pool.add_liquidity(0, vec![U128(to_yocto("10")), U128(to_yocto("20"))], None),
        deposit = to_yocto("0.0007")
    )
    .assert_success();
    assert_eq!(mft_balance_of(&pool, ":0", &root.account_id()), 999999999999999999999999);
    assert_eq!(mft_balance_of(&pool, ":0", &pool.account_id()), to_yocto("1"));
    assert_eq!(mft_total_supply(&pool, ":0"), 1999999999999999999999999u128);
    call!(
        root,
        pool.donation_share(
            0, Some(U128(1)), None
        ),
        deposit = 1
    ).assert_success();
    assert_eq!(mft_balance_of(&pool, ":0", &root.account_id()), 999999999999999999999998);
    assert_eq!(mft_balance_of(&pool, ":0", &pool.account_id()), to_yocto("1") + 1);
    assert_eq!(mft_total_supply(&pool, ":0"), 1999999999999999999999999u128);
    let deposit_before_donation_share = get_storage_state(&pool, to_va(root.account_id.clone())).unwrap().deposit.0;
    let outcome = call!(
        root,
        pool.donation_share(
            0, None, Some(true)
        ),
        deposit = 1
    );
    println!("{:#?}", get_logs(&outcome));

    assert_eq!(mft_balance_of(&pool, ":0", &root.account_id()), 0);
    assert_eq!(mft_balance_of(&pool, ":0", &pool.account_id()), 1999999999999999999999999u128);
    assert_eq!(mft_total_supply(&pool, ":0"), 1999999999999999999999999u128);
    assert!(deposit_before_donation_share < get_storage_state(&pool, to_va(root.account_id.clone())).unwrap().deposit.0);
}

#[test]
fn donation_token() {
    let (root, owner, pool, token1, _token2, _token3) = setup_pool_with_liquidity();
    let user = root.create_user("user".to_string(), to_yocto("100"));
    mint_and_deposit_token(&user, &token1, &pool, to_yocto("100"));
    let balances = view!(pool.get_deposits(to_va(user.account_id.clone())))
        .unwrap_json::<HashMap<AccountId, U128>>();
    let balances = balances.get(&token1.account_id()).unwrap().0;
    assert_eq!(balances, to_yocto("100"));
    let balances = view!(pool.get_deposits(to_va(owner.account_id.clone())))
        .unwrap_json::<HashMap<AccountId, U128>>();
    assert!(balances.is_empty());
    call!(
        user,
        pool.donation_token(
            token1.valid_account_id(), None, None
        ),
        deposit = 1
    ).assert_success();
    let balances = view!(pool.get_deposits(to_va(user.account_id.clone())))
        .unwrap_json::<HashMap<AccountId, U128>>();
    let balances = balances.get(&token1.account_id()).unwrap().0;
    assert_eq!(balances, 0);
    let balances = view!(pool.get_deposits(to_va(owner.account_id.clone())))
        .unwrap_json::<HashMap<AccountId, U128>>();
    let balances = balances.get(&token1.account_id()).unwrap().0;
    assert_eq!(balances, to_yocto("100"));

    let user1 = root.create_user("user1".to_string(), to_yocto("500"));
    mint_and_deposit_token(&user1, &token1, &pool, to_yocto("500"));
    let balances = view!(pool.get_deposits(to_va(user1.account_id.clone())))
        .unwrap_json::<HashMap<AccountId, U128>>();
    let balances = balances.get(&token1.account_id()).unwrap().0;
    assert_eq!(balances, to_yocto("500"));
    call!(
        user1,
        pool.donation_token(
            token1.valid_account_id(), Some(U128(to_yocto("100"))), None
        ),
        deposit = 1
    ).assert_success();
    let balances = view!(pool.get_deposits(to_va(user1.account_id.clone())))
        .unwrap_json::<HashMap<AccountId, U128>>();
    let balances = balances.get(&token1.account_id()).unwrap().0;
    assert_eq!(balances, to_yocto("400"));
    let balances = view!(pool.get_deposits(to_va(owner.account_id.clone())))
        .unwrap_json::<HashMap<AccountId, U128>>();
    let balances = balances.get(&token1.account_id()).unwrap().0;
    assert_eq!(balances, to_yocto("200"));

    let outcome = call!(
        user1,
        pool.donation_token(
            token1.valid_account_id(), None, Some(true)
        ),
        deposit = 1
    );
    println!("{:#?}", get_logs(&outcome));

    let balances = view!(pool.get_deposits(to_va(user1.account_id.clone())))
        .unwrap_json::<HashMap<AccountId, U128>>();
    assert!(balances.is_empty());
    let balances = view!(pool.get_deposits(to_va(owner.account_id.clone())))
        .unwrap_json::<HashMap<AccountId, U128>>();
    let balances = balances.get(&token1.account_id()).unwrap().0;
    assert_eq!(balances, to_yocto("600"));

}