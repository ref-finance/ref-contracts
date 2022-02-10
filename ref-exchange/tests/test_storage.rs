/// The storage in REF consists of inner-account storage (A storage) and LP-token storage (T storage).
/// For A storage:
///   Basic cost is 0.00102 Near (102 bytes),
///   Each token cost is 0.00148 Near (148 bytes),
///   Following actions will examine A storage:
///     ft::ft_transfer_call to deposit token into,
///     [withdraw], [register_tokens], [unregister_tokens],
/// For T storage:
///   Each pool has its own LP token, 
///   Each lp as a token holder would do storage_register, in REF, that is,
///     lp can call explicitly [mft_register], suggested deposit amount is 0.005, unused part would refund,
///     lp can call [add_liquidity], suggested deposit amount is 0.005, unused part would refund,
///   The contract self would be registered by pool creator 
///     when [add_simple_pool] and [add_stable_swap_pool], 
///     suggested deposit amount is 0.01, unused part would refund
use near_sdk::json_types::{U128};
use near_sdk_sim::{call, to_yocto};

use ref_exchange::SwapAction;
use crate::common::utils::*;
pub mod common;

const ONE_LPT: u128 = 1000000000000000000;
const ONE_DAI: u128 = 1000000000000000000;
const ONE_USDT: u128 = 1000000;
const ONE_USDC: u128 = 1000000;

#[test]
fn storage_scenario_01() {
    let (root, _, pool, token1, _, _) = setup_pool_with_liquidity();
    let new_user = root.create_user("new_user".to_string(), to_yocto("100"));

    println!("Storage Case 0101: withdraw MAX using None");
    call!(
        new_user,
        pool.storage_deposit(None, None),
        deposit = to_yocto("1")
    )
    .assert_success();
    let sb = get_storage_balance(&pool, new_user.valid_account_id()).unwrap();
    assert_eq!(sb.total.0, to_yocto("1"));
    assert_eq!(sb.total.0 - sb.available.0, to_yocto("0.00102"));
    let orig_user_balance = new_user.account().unwrap().amount;

    // withdraw as much storage near as he can
    let out_come = call!(
        new_user,
        pool.storage_withdraw(None),
        deposit = 1
    );
    out_come.assert_success();

    let sb = get_storage_balance(&pool, new_user.valid_account_id()).unwrap();
    assert_eq!(sb.total.0, to_yocto("0.00102"));
    assert_eq!(sb.available.0, to_yocto("0"));
    // println!("{}", new_user.account().unwrap().amount - orig_user_balance);
    assert!(
        new_user.account().unwrap().amount - orig_user_balance > 
        to_yocto("0.998")
    );

    println!("Storage Case 0102: deposit token would fail with insufficient storage deposit");
    call!(
        new_user,
        token1.mint(to_va(new_user.account_id.clone()), U128(to_yocto("10")))
    )
    .assert_success();
    let out_come = call!(
        new_user,
        token1.ft_transfer_call(to_va(swap()), to_yocto("5").into(), None, "".to_string()),
        deposit = 1
    );
    out_come.assert_success();
    let ex_status = format!("{:?}", out_come.promise_errors()[0].as_ref().unwrap().status());
    assert!(ex_status.contains("E11: insufficient $NEAR storage deposit"));

    println!("Storage Case 0103: deposit token would success with enough storage deposit");
    call!(
        new_user,
        pool.storage_deposit(None, Some(false)),
        deposit = to_yocto("1")
    )
    .assert_success();
    let out_come = call!(
        new_user,
        token1.ft_transfer_call(to_va(swap()), to_yocto("5").into(), None, "".to_string()),
        deposit = 1
    );
    out_come.assert_success();
    assert_eq!(out_come.promise_errors().len(), 0);

    println!("Storage Case 0104: storage withdraw more than available");
    let prev_sb = get_storage_balance(&pool, new_user.valid_account_id()).unwrap();
    let orig_user_balance = new_user.account().unwrap().amount;
    // println!("{:#?}", prev_sb);

    let out_come = call!(
        new_user,
        pool.storage_withdraw(Some(U128(to_yocto("1")))),
        deposit = 1
    );
    assert!(!out_come.is_ok());

    let sb = get_storage_balance(&pool, new_user.valid_account_id()).unwrap();
    assert_eq!(sb.total.0, prev_sb.total.0);
    assert_eq!(sb.available.0, prev_sb.available.0);
    assert!(new_user.account().unwrap().amount < orig_user_balance);

    println!("Storage Case 0105: storage withdraw specific amount");
    let prev_sb = get_storage_balance(&pool, new_user.valid_account_id()).unwrap();
    let orig_user_balance = new_user.account().unwrap().amount;

    let out_come = call!(
        new_user,
        pool.storage_withdraw(Some(U128(to_yocto("0.5")))),
        deposit = 1
    );
    out_come.assert_success();

    let sb = get_storage_balance(&pool, new_user.valid_account_id()).unwrap();
    assert_eq!(sb.total.0, prev_sb.total.0 - to_yocto("0.5"));
    assert_eq!(sb.available.0, prev_sb.available.0 - to_yocto("0.5"));
    assert!(new_user.account().unwrap().amount - orig_user_balance > to_yocto("0.499"));

}


#[test]
fn storage_scenario_02() {
    let (root, _owner, pool, tokens) = 
        setup_stable_pool_with_liquidity(
            vec![dai(), usdt(), usdc()],
            vec![100000*ONE_DAI, 100000*ONE_USDT, 100000*ONE_USDC],
            vec![18, 6, 6],
            25,
            10000,
        );
    let tokens = &tokens;

    // prepare a new user with 3 tokens storage 102 + 3 * 148 = 102 + 444 = 546
    let new_user = root.create_user("new_user1".to_string(), to_yocto("100"));
    mint_and_deposit_token(&new_user, &tokens[0], &pool, 500*ONE_DAI);
    mint_and_deposit_token(&new_user, &tokens[1], &pool, 500*ONE_USDT);
    mint_and_deposit_token(&new_user, &tokens[2], &pool, 500*ONE_USDC);
    let ss = get_storage_state(&pool, new_user.valid_account_id()).unwrap();
    assert_eq!(ss.usage.0, to_yocto("0.00546"));

    // appending balanced liquidity with basic lp register storage fee
    println!("Storage Case 0201: appending balanced liquidity need deposit storage");
    call!(
        new_user,
        pool.add_stable_liquidity(0, vec![U128(10*ONE_DAI), U128(10*ONE_USDT), U128(10*ONE_USDC)], U128(1)),
        deposit = to_yocto("0.00074")
    )
    .assert_success();
    let ss = get_storage_state(&pool, new_user.valid_account_id()).unwrap();
    assert_eq!(ss.usage.0, to_yocto("0.00546"));

    // appending imba liquidity with extra storage fee for exchange share
    println!("Storage Case 0202: appending imba liquidity need deposit storage");
    let out_come = call!(
        new_user,
        pool.add_stable_liquidity(0, vec![U128(5*ONE_DAI), U128(10*ONE_USDT), U128(15*ONE_USDC)], U128(1)),
        deposit = to_yocto("0.00074")
    );
    out_come.assert_success();
    let ss = get_storage_state(&pool, new_user.valid_account_id()).unwrap();
    assert_eq!(ss.usage.0, to_yocto("0.00546"));

    // remove liquidity by share
    println!("Storage Case 0203: remove liquidity by share");
    let out_come = call!(
        new_user,
        pool.remove_liquidity(0, U128(10*ONE_LPT), vec![U128(3*ONE_DAI), U128(3*ONE_USDT), U128(3*ONE_USDC)]),
        deposit = 1
    );
    out_come.assert_success();
    let ss = get_storage_state(&pool, new_user.valid_account_id()).unwrap();
    assert_eq!(ss.usage.0, to_yocto("0.00546"));

    // remove liquidity by token
    println!("Storage Case 0204: remove liquidity by token");
    let out_come = call!(
        new_user,
        pool.remove_liquidity_by_tokens(0, vec![U128(10*ONE_DAI), U128(1*ONE_USDT), U128(1*ONE_USDC)], U128(13*ONE_LPT)),
        deposit = 1
    );
    out_come.assert_success();
    let ss = get_storage_state(&pool, new_user.valid_account_id()).unwrap();
    assert_eq!(ss.usage.0, to_yocto("0.00546"));

    // swap 
    println!("Storage Case 0205: swap would fail if storage insufficient");
    let out_come = call!(
        new_user,
        pool.swap(
            vec![SwapAction {
                pool_id: 0,
                token_in: dai(),
                amount_in: Some(U128(ONE_DAI)),
                token_out: usdc(),
                min_amount_out: U128(1)
            }],
            None
        ),
        deposit = 1
    );
    out_come.assert_success();
    // println!("{:#?}", get_logs(&out_come));
    let ss = get_storage_state(&pool, new_user.valid_account_id()).unwrap();
    assert_eq!(ss.usage.0, to_yocto("0.00546"));

    let user2 = root.create_user("user2".to_string(), to_yocto("100"));
    mint_and_deposit_token(&user2, &tokens[0], &pool, 500*ONE_DAI);

    let out_come = call!(
        user2,
        pool.storage_withdraw(None),
        deposit = 1
    );
    out_come.assert_success();

    let ss = get_storage_state(&pool, user2.valid_account_id()).unwrap();
    assert_eq!(ss.deposit.0, to_yocto("0.00250"));
    assert_eq!(ss.usage.0, to_yocto("0.00250"));

    let out_come = call!(
        user2,
        pool.swap(
            vec![SwapAction {
                pool_id: 0,
                token_in: dai(),
                amount_in: Some(U128(ONE_DAI)),
                token_out: usdc(),
                min_amount_out: U128(1)
            }],
            None
        ),
        deposit = 1
    );
    assert!(!out_come.is_ok());
    let ex_status = format!("{:?}", out_come.promise_errors()[0].as_ref().unwrap().status());
    // println!("{}", ex_status);
    assert!(ex_status.contains("E11: insufficient $NEAR storage deposit"));

    call!(
        user2,
        pool.storage_deposit(None, Some(false)),
        deposit = to_yocto("0.00148")
    )
    .assert_success();
    let ss = get_storage_state(&pool, user2.valid_account_id()).unwrap();
    assert_eq!(ss.deposit.0, to_yocto("0.00398"));
    assert_eq!(ss.usage.0, to_yocto("0.00250"));

    let out_come = call!(
        user2,
        pool.swap(
            vec![SwapAction {
                pool_id: 0,
                token_in: dai(),
                amount_in: Some(U128(ONE_DAI)),
                token_out: usdc(),
                min_amount_out: U128(1)
            }],
            None
        ),
        deposit = 1
    );
    out_come.assert_success();
    let ss = get_storage_state(&pool, user2.valid_account_id()).unwrap();
    assert_eq!(ss.deposit.0, to_yocto("0.00398"));
    assert_eq!(ss.usage.0, to_yocto("0.00398"));

    println!("Storage Case 0206: transfer lp would fail if receiver not registered");
    let user3 = root.create_user("user3".to_string(), to_yocto("100"));
    let out_come = call!(
        new_user,
        pool.mft_transfer(":0".to_string(), user3.valid_account_id(), U128(5*ONE_LPT), None),
        deposit = 1
    );
    assert!(!out_come.is_ok());
    let ex_status = format!("{:?}", out_come.promise_errors()[0].as_ref().unwrap().status());
    assert!(ex_status.contains("E13: LP not registered"));
    
    println!("Storage Case 0207: remove liquidity would fail if not enough storage for received token");
    let out_come = call!(
        new_user,
        pool.mft_register(":0".to_string(), user3.valid_account_id()),
        deposit = to_yocto("0.00074")
    );
    out_come.assert_success();
    let out_come = call!(
        new_user,
        pool.mft_transfer(":0".to_string(), user3.valid_account_id(), U128(5*ONE_LPT), None),
        deposit = 1
    );
    out_come.assert_success();
    let out_come = call!(
        user3,
        pool.remove_liquidity(0, U128(5*ONE_LPT), vec![U128(1*ONE_DAI), U128(1*ONE_USDT), U128(1*ONE_USDC)]),
        deposit = 1
    );
    assert!(!out_come.is_ok());
    let ex_status = format!("{:?}", out_come.promise_errors()[0].as_ref().unwrap().status());
    assert!(ex_status.contains("E11: insufficient $NEAR storage deposit"));
    assert!(get_storage_state(&pool, user3.valid_account_id()).is_none());

    call!(
        user3,
        pool.storage_deposit(None, None),
        deposit = to_yocto("0.00546")
    )
    .assert_success();

    let out_come = call!(
        user3,
        pool.remove_liquidity(0, U128(5*ONE_LPT), vec![U128(1*ONE_DAI), U128(1*ONE_USDT), U128(1*ONE_USDC)]),
        deposit = 1
    );
    out_come.assert_success();
    let ss = get_storage_state(&pool, user3.valid_account_id()).unwrap();
    assert_eq!(ss.deposit.0, to_yocto("0.00546"));
    assert_eq!(ss.usage.0, to_yocto("0.00546"));
}
