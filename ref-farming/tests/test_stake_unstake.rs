use near_sdk_sim::{call, init_simulator, to_yocto};

use crate::common::utils::*;
use crate::common::views::*;
use crate::common::actions::*;

mod common;


/// staking, unstaking, staking again, half unstaking
/// append staking
#[test]
fn lpt_stake_unstake() {
    let root = init_simulator(None);

    // prepair users
    let owner = root.create_user("owner".to_string(), to_yocto("100"));
    let farmer1 = root.create_user("farmer1".to_string(), to_yocto("100"));

    let (pool, token1, _) = prepair_pool_and_liquidity(&root, &owner, farming_id(), vec![&farmer1]);

    let (farming, farm_id) = prepair_farm(&root, &owner, &token1, to_yocto("500"));
    let farm_info = show_farminfo(&farming, farm_id.clone(), false);
    assert_farming(&farm_info, "Running".to_string(), to_yocto("500"), 0, 0, 0, 0, 0);

    call!(farmer1, farming.storage_deposit(None, None), deposit = to_yocto("1"))
    .assert_success();
    let out_come = call!(
        farmer1,
        pool.mft_transfer_call(":0".to_string(), to_va(farming_id()), to_yocto("1").into(), None, "".to_string()),
        deposit = 1
    );
    out_come.assert_success();
    let user_seeds = show_userseeds(&farming, farmer1.account_id(), false);
    assert_eq!(user_seeds.get(&String::from("swap@0")).unwrap().0, to_yocto("1"));

    assert!(root.borrow_runtime_mut().produce_blocks(60).is_ok());
    let out_come = call!(
        farmer1,
        farming.withdraw_seed(format!("{}@0", swap()), to_yocto("1").into()),
        deposit = 1
    );
    out_come.assert_success();
    let user_seeds = show_userseeds(&farming, farmer1.account_id(), false);
    assert!(user_seeds.get(&String::from("swap@0")).is_none());
    
    assert!(root.borrow_runtime_mut().produce_blocks(120).is_ok());
    let out_come = call!(
        farmer1,
        pool.mft_transfer_call(":0".to_string(), to_va(farming_id()), to_yocto("0.5").into(), None, "".to_string()),
        deposit = 1
    );
    out_come.assert_success();
    let user_seeds = show_userseeds(&farming, farmer1.account_id(), false);
    assert_eq!(user_seeds.get(&String::from("swap@0")).unwrap().0, to_yocto("0.5"));
    
    assert!(root.borrow_runtime_mut().produce_blocks(60).is_ok());
    let out_come = call!(
        farmer1,
        pool.mft_transfer_call(":0".to_string(), to_va(farming_id()), to_yocto("0.5").into(), None, "".to_string()),
        deposit = 1
    );
    out_come.assert_success();
    let user_seeds = show_userseeds(&farming, farmer1.account_id(), false);
    assert_eq!(user_seeds.get(&String::from("swap@0")).unwrap().0, to_yocto("1"));
    
    assert!(root.borrow_runtime_mut().produce_blocks(60).is_ok());
    let out_come = call!(
        farmer1,
        farming.withdraw_seed(format!("{}@0", swap()), to_yocto("0.5").into()),
        deposit = 1
    );
    out_come.assert_success();
    let user_seeds = show_userseeds(&farming, farmer1.account_id(), false);
    assert_eq!(user_seeds.get(&String::from("swap@0")).unwrap().0, to_yocto("0.5"));
    
    assert!(root.borrow_runtime_mut().produce_blocks(60).is_ok());
    let out_come = call!(
        farmer1,
        farming.withdraw_seed(format!("{}@0", swap()), to_yocto("0.5").into()),
        deposit = 1
    );
    out_come.assert_success();
    let user_seeds = show_userseeds(&farming, farmer1.account_id(), false);
    assert!(user_seeds.get(&String::from("swap@0")).is_none());

    assert!(root.borrow_runtime_mut().produce_blocks(60).is_ok());
    let out_come = call!(
        farmer1,
        pool.mft_transfer_call(":0".to_string(), to_va(farming_id()), to_yocto("1").into(), None, "".to_string()),
        deposit = 1
    );
    out_come.assert_success();
    let user_seeds = show_userseeds(&farming, farmer1.account_id(), false);
    assert_eq!(user_seeds.get(&String::from("swap@0")).unwrap().0, to_yocto("1"));
    
    assert!(root.borrow_runtime_mut().produce_blocks(60).is_ok());
    let farm_info = show_farminfo(&farming, farm_id.clone(), false);
    assert_farming(&farm_info, "Running".to_string(), to_yocto("500"), 8, 7, to_yocto("7"), to_yocto("1"), to_yocto("3"));
    let unclaim = show_unclaim(&farming, farmer1.account_id(), farm_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("1"));
    let out_come = call!(
        farmer1,
        farming.claim_reward_by_farm(farm_id.clone()),
        deposit = 0
    );
    out_come.assert_success();
    let farm_info = show_farminfo(&farming, farm_id.clone(), false);
    assert_farming(&farm_info, "Running".to_string(), to_yocto("500"), 8, 8, to_yocto("8"), 0, to_yocto("3"));
    let unclaim = show_unclaim(&farming, farmer1.account_id(), farm_id.clone(), false);
    assert_eq!(unclaim.0, 0_u128);
}

