use near_sdk::json_types::{U128};
use near_sdk_sim::{call, to_yocto};

use ref_exchange::SwapAction;
use crate::common::utils::*;
pub mod common;

#[test]
fn frozenlist_scenario_01() {
    let (root, owner, pool, _, _, _) = setup_pool_with_liquidity();
    let guard1 = root.create_user("guard1".to_string(), to_yocto("100"));
    
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
}

#[test]
fn frozenlist_scenario_02() {
    let (root, owner, pool, token1, token2, _) = setup_pool_with_liquidity();
    let guard1 = root.create_user("guard1".to_string(), to_yocto("100"));
    let new_user = root.create_user("new_user".to_string(), to_yocto("100"));

    call!(
        owner,
        pool.extend_guardians(vec![guard1.valid_account_id()]),
        deposit=1
    ).assert_success();

    println!("Frozenlist Case 0102: token in frozenlist");

    let out_come = call!(
        guard1,
        pool.extend_frozenlist_tokens(vec![to_va(eth()), to_va(dai()), to_va(usdt())]),
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
        pool.withdraw(to_va(eth()), U128(to_yocto("1")), None),
        deposit = 1
    );
    assert!(!out_come.is_ok());
    assert_eq!(get_error_count(&out_come), 1);
    assert!(get_error_status(&out_come).contains("E52: token frozen"));

    // swap would fail, even only token_out is frozen
    let out_come = call!(
        owner,
        pool.remove_frozenlist_tokens(vec![to_va(dai())]),
        deposit=1
    );
    out_come.assert_success();
    
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
            None
        ),
        deposit = 1
    );
    assert!(!out_come.is_ok());
    assert_eq!(get_error_count(&out_come), 1);
    assert!(get_error_status(&out_come).contains("E52: token frozen"));
}