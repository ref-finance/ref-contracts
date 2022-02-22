use near_sdk::json_types::{U128};
use near_sdk_sim::{call, to_yocto};

use crate::common::utils::*;
pub mod common;

#[test]
fn owner_scenario_01() {
    let (root, owner, pool, token1, _, _) = setup_pool_with_liquidity();
    let new_user = root.create_user("new_user".to_string(), to_yocto("100"));

    assert_eq!(balance_of(&token1, &pool.account_id()), to_yocto("105"));

    call!(
        root,
        token1.ft_transfer_call(pool.valid_account_id(), to_yocto("10").into(), None, "".to_string()),
        deposit = 1
    )
    .assert_success();

    assert_eq!(balance_of(&token1, &pool.account_id()), to_yocto("115"));

    println!("Owner Case 0101: only owner can retrieve unmanaged tokens");
    let out_come = call!(
        root,
        pool.retrieve_unmanaged_token(token1.valid_account_id(), to_yocto("10").into()),
        deposit = 1
    );
    assert!(!out_come.is_ok());
    assert_eq!(get_error_count(&out_come), 1);
    assert!(get_error_status(&out_come).contains("ERR_NOT_ALLOWED"));
    // println!("{}", get_error_status(&out_come));

    println!("Owner Case 0102: owner retrieve unmanaged token but unregstered");
    let out_come = call!(
        owner,
        pool.retrieve_unmanaged_token(token1.valid_account_id(), to_yocto("10").into()),
        deposit = 1
    );
    assert!(!out_come.is_ok());
    assert_eq!(get_error_count(&out_come), 1);
    assert!(get_error_status(&out_come).contains("The account owner is not registered"));
    assert_eq!(balance_of(&token1, &pool.account_id()), to_yocto("115"));

    
    println!("Owner Case 0103: owner retrieve unmanaged tokens");
    call!(
        owner,
        token1.storage_deposit(None, None),
        deposit = to_yocto("1")
    )
    .assert_success();
    let out_come = call!(
        owner,
        pool.retrieve_unmanaged_token(token1.valid_account_id(), to_yocto("10").into()),
        deposit = 1
    );
    out_come.assert_success();
    assert_eq!(get_error_count(&out_come), 0);
    assert_eq!(balance_of(&token1, &pool.account_id()), to_yocto("105"));
    assert_eq!(balance_of(&token1, &owner.account_id()), to_yocto("10"));
}