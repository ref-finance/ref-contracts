use near_sdk::json_types::{U128};
use near_sdk_sim::{call, to_yocto};
use crate::common::utils::*;
pub mod common;



#[test]
fn inner_account_scenario_01() {
    let (root, _, pool, token1, token2, _) = setup_pool_with_liquidity();
    let new_user = root.create_user("new_user".to_string(), to_yocto("100"));
    call!(
        new_user,
        token1.mint(new_user.valid_account_id(), U128(to_yocto("10")))
    )
    .assert_success();
    call!(
        new_user,
        pool.storage_deposit(None, None),
        deposit = to_yocto("1")
    )
    .assert_success();

    let out_come = call!(
        new_user,
        token1.ft_transfer_call(pool.valid_account_id(), to_yocto("10").into(), None, "".to_string()),
        deposit = 1
    );
    out_come.assert_success();
    assert_eq!(out_come.promise_errors().len(), 0);
    assert_eq!(balance_of(&token1, &new_user.account_id), to_yocto("0"));
    assert_eq!(get_deposits(&pool, new_user.valid_account_id()).get(&token1.account_id()).unwrap().0, to_yocto("10"));

    println!("Inner Account Case 0101: withdraw half");
    let out_come = call!(
        new_user,
        pool.withdraw(token1.valid_account_id(), U128(to_yocto("5")), None, None),
        deposit = 1
    );
    out_come.assert_success();
    assert_eq!(out_come.promise_errors().len(), 0);
    assert_eq!(balance_of(&token1, &new_user.account_id()), to_yocto("5"));
    assert_eq!(get_deposits(&pool, new_user.valid_account_id()).get(&token1.account_id()).unwrap().0, to_yocto("5"));

    println!("Inner Account Case 0102: withdraw more than have");
    let out_come = call!(
        new_user,
        pool.withdraw(token1.valid_account_id(), U128(to_yocto("6")), None, None),
        deposit = 1
    );
    assert!(!out_come.is_ok());
    assert_eq!(get_error_count(&out_come), 1);
    assert!(get_error_status(&out_come).contains("E22: not enough tokens in deposit"));
    assert_eq!(balance_of(&token1, &new_user.account_id()), to_yocto("5"));
    assert_eq!(get_deposits(&pool, new_user.valid_account_id()).get(&token1.account_id()).unwrap().0, to_yocto("5"));

    println!("Inner Account Case 0103: withdraw some and unregister");
    let out_come = call!(
        new_user,
        pool.withdraw(token1.valid_account_id(), U128(to_yocto("1")), Some(true), None),
        deposit = 1
    );
    assert!(!out_come.is_ok());
    assert_eq!(get_error_count(&out_come), 1);
    assert!(get_error_status(&out_come).contains("E24: non-zero token balance"));
    assert_eq!(balance_of(&token1, &new_user.account_id()), to_yocto("5"));
    assert_eq!(get_deposits(&pool, new_user.valid_account_id()).get(&token1.account_id()).unwrap().0, to_yocto("5"));

    println!("Inner Account Case 0104: withdraw non-empty token with 0 amonut");
    let out_come = call!(
        new_user,
        pool.withdraw(token1.valid_account_id(), U128(0), None, None),
        deposit = 1
    );
    out_come.assert_success();
    assert_eq!(out_come.promise_errors().len(), 0);
    assert_eq!(balance_of(&token1, &new_user.account_id()), to_yocto("10"));
    assert_eq!(get_deposits(&pool, new_user.valid_account_id()).get(&token1.account_id()).unwrap().0, to_yocto("0"));
    
    println!("Inner Account Case 0105: withdraw unregister token");
    let out_come = call!(
        new_user,
        pool.withdraw(token2.valid_account_id(), U128(to_yocto("1")), None, None),
        deposit = 1
    );
    assert!(!out_come.is_ok());
    assert_eq!(get_error_count(&out_come), 1);
    assert!(get_error_status(&out_come).contains("E21: token not registered"));

    println!("Inner Account Case 0106: withdraw empty token with 0 amount");
    let out_come = call!(
        new_user,
        pool.withdraw(token1.valid_account_id(), U128(0), None, None),
        deposit = 1
    );
    assert!(!out_come.is_ok());
    assert_eq!(get_error_count(&out_come), 1);
    assert!(get_error_status(&out_come).contains("E29: Illegal withdraw amount"));
}
