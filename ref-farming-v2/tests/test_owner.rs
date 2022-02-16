use near_sdk::collections::Vector;
use near_sdk_sim::{call, init_simulator, to_yocto, view};
use near_sdk::json_types::{U128};
use near_sdk::serde_json::Value;

use ref_farming_v2::{HRSimpleFarmTerms, CDStrategyInfo};

use crate::common::utils::*;
use crate::common::init::deploy_farming;
use crate::common::views::*;
use crate::common::actions::*;

mod common;

#[test]
pub fn test_strategy(){
    let root = init_simulator(None);

    println!("----->> Prepare accounts.");
    let owner = root.create_user("owner".to_string(), to_yocto("100"));
    println!("<<----- owner prepared.");

    println!("----->> Prepare ref-exchange and swap pool.");
    let (pool, token1, _) = prepair_pool_and_liquidity(
        &root, &owner, farming_id(), vec![]);
    println!("<<----- The pool prepaired.");

    // deploy farming contract and register user
    println!("----->> Deploy farming.");
    let farming = deploy_farming(&root, farming_id(), owner.account_id());
    let strategy_info = view!(farming.get_cd_strategy()).unwrap_json::<CDStrategyInfo>();
    assert_strategy(&strategy_info, vec![], vec![], 0, 0);

    call!(
        owner,
        farming.modify_cd_strategy_lock_time(1, 101)
    ).assert_success();
    let strategy_info = view!(farming.get_cd_strategy()).unwrap_json::<CDStrategyInfo>();
    assert_strategy(&strategy_info, vec![101], vec![], 0, 0);

    call!(
        owner,
        farming.modify_cd_strategy_additional(1, 10)
    ).assert_success();
    let strategy_info = view!(farming.get_cd_strategy()).unwrap_json::<CDStrategyInfo>();
    assert_strategy(&strategy_info, vec![101], vec![10], 0, 0);

    call!(
        owner,
        farming.modify_cd_strategy_damage(111)
    ).assert_success();
    let strategy_info = view!(farming.get_cd_strategy()).unwrap_json::<CDStrategyInfo>();
    assert_strategy(&strategy_info, vec![101], vec![10], 111, 0);

    call!(
        owner,
        farming.modify_cd_strategy_denominator(222)
    ).assert_success();
    let strategy_info = view!(farming.get_cd_strategy()).unwrap_json::<CDStrategyInfo>();
    assert_strategy(&strategy_info, vec![101], vec![10], 111, 222);

    call!(
        owner,
        farming.modify_cd_strategy_lock_time(1, 202)
    ).assert_success();
    let strategy_info = view!(farming.get_cd_strategy()).unwrap_json::<CDStrategyInfo>();
    assert_strategy(&strategy_info, vec![101, 202], vec![10], 111, 222);

    call!(
        owner,
        farming.modify_cd_strategy_lock_time(1, 333)
    ).assert_success();
    let strategy_info = view!(farming.get_cd_strategy()).unwrap_json::<CDStrategyInfo>();
    assert_strategy(&strategy_info, vec![101, 333], vec![10], 111, 222);

    call!(
        owner,
        farming.modify_cd_strategy_lock_time(0, 0)
    ).assert_success();
    let strategy_info = view!(farming.get_cd_strategy()).unwrap_json::<CDStrategyInfo>();
    assert_strategy(&strategy_info, vec![333], vec![10], 111, 222);

    call!(
        owner,
        farming.modify_cd_strategy_additional(1, 20)
    ).assert_success();
    let strategy_info = view!(farming.get_cd_strategy()).unwrap_json::<CDStrategyInfo>();
    assert_strategy(&strategy_info, vec![333], vec![10, 20], 111, 222);

    call!(
        owner,
        farming.modify_cd_strategy_additional(0, 0)
    ).assert_success();
    let strategy_info = view!(farming.get_cd_strategy()).unwrap_json::<CDStrategyInfo>();
    assert_strategy(&strategy_info, vec![333], vec![20], 111, 222);
}