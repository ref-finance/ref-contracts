use near_sdk_sim::{call, init_simulator, to_yocto, view};
use ref_farming_v2::{CDStrategyInfo};

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
    let (_pool, _token1, _) = prepair_pool_and_liquidity(
        &root, &owner, farming_id(), vec![]);
    println!("<<----- The pool prepaired.");

    // deploy farming contract and register user
    println!("----->> Deploy farming.");
    let farming = deploy_farming(&root, farming_id(), owner.account_id());
    let strategy_info = view!(farming.get_cd_strategy()).unwrap_json::<CDStrategyInfo>();
    for index in 0..32{
        assert_strategy(&strategy_info, index, 0, 0, false, 0);
    }

    call!(
        owner,
        farming.modify_cd_strategy_item(0, 100, 10)
    ).assert_success();
    let strategy_info = view!(farming.get_cd_strategy()).unwrap_json::<CDStrategyInfo>();
    assert_strategy(&strategy_info, 0, 100, 10, true, 0);

    call!(
        owner,
        farming.modify_cd_strategy_item(0, 200, 20)
    ).assert_success();
    let strategy_info = view!(farming.get_cd_strategy()).unwrap_json::<CDStrategyInfo>();
    assert_strategy(&strategy_info, 0, 200, 20, true, 0);

    call!(
        owner,
        farming.modify_cd_strategy_item(0, 0, 20)
    ).assert_success();
    let strategy_info = view!(farming.get_cd_strategy()).unwrap_json::<CDStrategyInfo>();
    assert_strategy(&strategy_info, 0, 0, 0, false, 0);

    call!(
        owner,
        farming.modify_cd_strategy_damage(20)
    ).assert_success();
    let strategy_info = view!(farming.get_cd_strategy()).unwrap_json::<CDStrategyInfo>();
    assert_strategy(&strategy_info, 0, 0, 0, false, 20);
    
}