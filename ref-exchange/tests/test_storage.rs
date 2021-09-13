
use near_sdk::json_types::{U128, ValidAccountId};
use near_sdk_sim::{call, to_yocto};

use ref_exchange::ContractContract as Exchange;

use crate::common::utils::*;
pub mod common;

#[test]
fn storage_scenario_01() {
    let (root, owner, pool, token1, token2, _) = setup_pool_with_liquidity();
    let new_user = root.create_user("new_user".to_string(), to_yocto("100"));
    // call!(
    //     new_user,
    //     token1.mint(to_va(new_user.account_id.clone()), U128(to_yocto("10")))
    // )
    // .assert_success();

    call!(
        new_user,
        pool.storage_deposit(None, None),
        deposit = to_yocto("1")
    )
    .assert_success();
    let sb = get_storage_balance(&pool, new_user.valid_account_id()).unwrap();
    println!("deposit 1, t:{} a:{}", sb.total.0, sb.available.0);
    println!("user balance: {}", new_user.account().unwrap().amount);
    
    // withdraw as much storage near as he can
    let out_come = call!(
        new_user,
        pool.storage_withdraw(None),
        deposit = 1
    );
    out_come.assert_success();
    
    // println!("{:#?}", out_come.promise_results());
    let sb = get_storage_balance(&pool, new_user.valid_account_id()).unwrap();
    // the storage remain the same
    println!("withdraw max, t:{} a:{}", sb.total.0, sb.available.0);
    // but user really get near back
    println!("user balance: {}", new_user.account().unwrap().amount);
}