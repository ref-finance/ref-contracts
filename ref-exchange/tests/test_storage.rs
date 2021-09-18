use near_sdk::json_types::{U128};
use near_sdk_sim::{call, to_yocto};

use crate::common::utils::*;
pub mod common;

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
