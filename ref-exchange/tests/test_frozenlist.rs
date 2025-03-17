use near_sdk::json_types::{U128};
use near_sdk_sim::{call, to_yocto};

use ref_exchange::{Action, SwapAction};
use crate::common::utils::*;
pub mod common;

#[test]
fn frozenlist_scenario_01() {
    let (root, owner, pool, _, _, _) = setup_pool_with_liquidity();
    let guard1 = root.create_user("guard1".to_string(), to_yocto("100"));
    
    call!(
        guard1,
        pool.storage_deposit(None, None),
        deposit = to_yocto("1")
    )
    .assert_success();

    call!(
        owner,
        pool.extend_guardians(vec![guard1.valid_account_id()]),
        deposit=1
    ).assert_success();

    println!("Frozenlist Case 0101: only owner and guardians can manage frozenlist");

    let out_come = call!(
        guard1,
        pool.extend_frozenlist_tokens(vec![to_va(eth()), to_va(dai()), to_va(usdt())]),
        deposit=1
    );
    out_come.assert_success();
    assert_eq!(get_error_count(&out_come), 0);
    let fl = get_frozenlist(&pool);
    assert_eq!(fl.len(), 3);
    assert_eq!(fl.get(0).unwrap().clone(), eth());
    assert_eq!(fl.get(1).unwrap().clone(), dai());
    assert_eq!(fl.get(2).unwrap().clone(), usdt());

    let out_come = call!(
        root,
        pool.remove_frozenlist_tokens(vec![to_va(eth()), to_va(dai())]),
        deposit=1
    );
    assert!(!out_come.is_ok());
    assert_eq!(get_error_count(&out_come), 1);
    assert!(get_error_status(&out_come).contains("E100: no permission to invoke this"));
    let fl = get_frozenlist(&pool);
    assert_eq!(fl.len(), 3);
    assert_eq!(fl.get(0).unwrap().clone(), eth());
    assert_eq!(fl.get(1).unwrap().clone(), dai());
    assert_eq!(fl.get(2).unwrap().clone(), usdt());

    let out_come = call!(
        owner,
        pool.remove_frozenlist_tokens(vec![to_va(dai())]),
        deposit=1
    );
    out_come.assert_success();
    assert_eq!(get_error_count(&out_come), 0);
    let fl = get_frozenlist(&pool);
    assert_eq!(fl.len(), 2);
    assert_eq!(fl.get(0).unwrap().clone(), eth());
    assert_eq!(fl.get(1).unwrap().clone(), usdt());


    let out_come = call!(
        guard1,
        pool.remove_frozenlist_tokens(vec![to_va(eth())]),
        deposit=1
    );
    out_come.assert_success();
    assert_eq!(get_error_count(&out_come), 0);
    let fl = get_frozenlist(&pool);
    assert_eq!(fl.len(), 1);
    assert_eq!(fl.get(0).unwrap().clone(), usdt());

    let out_come = call!(
        guard1,
        pool.remove_frozenlist_tokens(vec![to_va(eth())]),
        deposit=1
    );
    assert!(!out_come.is_ok());
    assert_eq!(get_error_count(&out_come), 1);
    assert!(get_error_status(&out_come).contains("E53: token not in list"));
}

#[test]
fn frozenlist_scenario_02() {
    let (root, owner, pool, token1, token2, _) = setup_pool_with_liquidity();
    let guard1 = root.create_user("guard1".to_string(), to_yocto("100"));
    let new_user = root.create_user("new_user".to_string(), to_yocto("100"));

    call!(
        guard1,
        pool.storage_deposit(None, None),
        deposit = to_yocto("1")
    )
    .assert_success();

    call!(
        new_user,
        pool.storage_deposit(None, None),
        deposit = to_yocto("1")
    )
    .assert_success();

    call!(
        owner,
        pool.extend_guardians(vec![guard1.valid_account_id()]),
        deposit=1
    ).assert_success();

    println!("Frozenlist Case 0102: token in frozenlist");

    // add token1 & token3 into frozen, leave toekn2 (eth) still valid, 
    let out_come = call!(
        guard1,
        pool.extend_frozenlist_tokens(vec![to_va(dai()), to_va(usdt())]),
        deposit=1
    );
    out_come.assert_success();

    // deposit token would fail
    let out_come = call!(
        root,
        token1.ft_transfer_call(to_va(swap()), to_yocto("5").into(), None, "".to_string()),
        deposit = 1
    );
    out_come.assert_success();
    assert_eq!(get_error_count(&out_come), 1);
    assert!(get_error_status(&out_come).contains("E52: token frozen"));

    // add liquidity would fail
    let out_come = call!(
        root,
        pool.add_liquidity(0, vec![U128(to_yocto("10")), U128(to_yocto("20"))], None),
        deposit = to_yocto("0.0007")
    );
    assert!(!out_come.is_ok());
    assert_eq!(get_error_count(&out_come), 1);
    assert!(get_error_status(&out_come).contains("E52: token frozen"));

    // remove liquidity would fail
    let out_come = call!(
        root,
        pool.remove_liquidity(0, U128(to_yocto("1")), vec![U128(to_yocto("1")), U128(to_yocto("2"))]),
        deposit = 1
    );
    assert!(!out_come.is_ok());
    assert_eq!(get_error_count(&out_come), 1);
    assert!(get_error_status(&out_come).contains("E52: token frozen"));

    // execute_actions would fail
    let out_come = call!(
        root,
        pool.execute_actions(
            vec![Action::Swap(
                SwapAction {
                    pool_id: 0,
                    token_in: dai(),
                    amount_in: Some(U128(to_yocto("1"))),
                    token_out: eth(),
                    min_amount_out: U128(1)
            })],
            None,
            None
        ),
        deposit = 1
    );
    assert!(!out_come.is_ok());
    assert_eq!(get_error_count(&out_come), 1);
    assert!(get_error_status(&out_come).contains("E52: token frozen"));

    // swap would fail
    let out_come = call!(
        root,
        pool.swap(
            vec![SwapAction {
                pool_id: 0,
                token_in: dai(),
                amount_in: Some(U128(to_yocto("1"))),
                token_out: eth(),
                min_amount_out: U128(1)
            }],
            None,
            None
        ),
        deposit = 1
    );
    assert!(!out_come.is_ok());
    assert_eq!(get_error_count(&out_come), 1);
    assert!(get_error_status(&out_come).contains("E52: token frozen"));

    // instant swap would fail
    call!(
        new_user,
        token2.mint(to_va(new_user.account_id.clone()), U128(to_yocto("10")))
    )
    .assert_success();
    let msg = format!(
        "{{\"pool_id\": {}, \"token_in\": \"{}\", \"token_out\": \"{}\", \"min_amount_out\": \"{}\"}}",
        0, token2.account_id(), token1.account_id(), 1
    );
    let msg_str = format!("{{\"force\": 0, \"actions\": [{}]}}", msg);
    let out_come = call!(
        new_user,
        token2.ft_transfer_call(to_va(swap()), to_yocto("1").into(), None, msg_str.clone()),
        deposit = 1
    );
    out_come.assert_success();
    assert_eq!(get_error_count(&out_come), 1);
    assert!(get_error_status(&out_come).contains("E52: token frozen"));

    // withdraw token would fail
    let out_come = call!(
        root,
        pool.withdraw(to_va(dai()), U128(to_yocto("1")), None, None),
        deposit = 1
    );
    assert!(!out_come.is_ok());
    assert_eq!(get_error_count(&out_come), 1);
    assert!(get_error_status(&out_come).contains("E52: token frozen"));

    // swap would fail, even only token_out is frozen
    let out_come = call!(
        root,
        pool.swap(
            vec![SwapAction {
                pool_id: 0,
                token_in: eth(),
                amount_in: Some(U128(to_yocto("1"))),
                token_out: dai(),
                min_amount_out: U128(1)
            }],
            None,
            None
        ),
        deposit = 1
    );
    assert!(!out_come.is_ok());
    assert_eq!(get_error_count(&out_come), 1);
    assert!(get_error_status(&out_come).contains("E52: token frozen"));
}

const ONE_LPT: u128 = 1000000000000000000;
const ONE_DAI: u128 = 1000000000000000000;
const ONE_USDT: u128 = 1000000;
const ONE_USDC: u128 = 1000000;

#[test]
fn frozenlist_scenario_03() {
    let (root, owner, pool, _tokens) = 
        setup_stable_pool_with_liquidity(
            vec![dai(), usdt(), usdc()],
            vec![100000*ONE_DAI, 100000*ONE_USDT, 100000*ONE_USDC],
            vec![18, 6, 6],
            25,
            10000,
        );

    let guard1 = root.create_user("guard1".to_string(), to_yocto("100"));
    let new_user = root.create_user("new_user".to_string(), to_yocto("100"));

    call!(
        guard1,
        pool.storage_deposit(None, None),
        deposit = to_yocto("1")
    )
    .assert_success();

    call!(
        new_user,
        pool.storage_deposit(None, None),
        deposit = to_yocto("1")
    )
    .assert_success();

    call!(
        owner,
        pool.extend_guardians(vec![guard1.valid_account_id()]),
        deposit=1
    ).assert_success();

    println!("Frozenlist Case 0103: stable pool token in frozenlist");
    let out_come = call!(
        guard1,
        pool.extend_frozenlist_tokens(vec![to_va(dai()), to_va(usdt())]),
        deposit=1
    );
    out_come.assert_success();

    // add liquidity would fail
    let out_come = call!(
        new_user,
        pool.add_stable_liquidity(0, vec![U128(10*ONE_DAI), U128(10*ONE_USDT), U128(10*ONE_USDC)], U128(1)),
        deposit = to_yocto("0.00074")
    );
    assert!(!out_come.is_ok());
    assert_eq!(get_error_count(&out_come), 1);
    assert!(get_error_status(&out_come).contains("E52: token frozen"));

    // remove liquidity would fail
    let out_come = call!(
        new_user,
        pool.remove_liquidity(0, U128(10*ONE_LPT), vec![U128(3*ONE_DAI), U128(3*ONE_USDT), U128(3*ONE_USDC)]),
        deposit = 1
    );
    assert!(!out_come.is_ok());
    assert_eq!(get_error_count(&out_come), 1);
    assert!(get_error_status(&out_come).contains("E52: token frozen"));

    // remove liquidity by token would fail
    let out_come = call!(
        new_user,
        pool.remove_liquidity_by_tokens(0, vec![U128(10*ONE_DAI), U128(1*ONE_USDT), U128(1*ONE_USDC)], U128(13*ONE_LPT)),
        deposit = 1
    );
    assert!(!out_come.is_ok());
    assert_eq!(get_error_count(&out_come), 1);
    assert!(get_error_status(&out_come).contains("E52: token frozen"));
}