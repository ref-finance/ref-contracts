use near_sdk_sim::{call, init_simulator, to_yocto, view};
use near_sdk::json_types::{U128};

use crate::common::utils::*;
use crate::common::views::*;
use crate::common::actions::*;

mod common;


/// staking, unstaking, staking again, half unstaking
/// append staking
#[test]
fn one_farm_staking() {
    let root = init_simulator(None);

    // prepair users
    let owner = root.create_user("owner".to_string(), to_yocto("100"));
    let farmer1 = root.create_user("farmer1".to_string(), to_yocto("100"));
    println!("----->> farmer1 prepaired.");

    // prepair pool and tokens
    let(pool, token1, token2) = prepair_pool(&root, &owner);
    println!("----->> The pool prepaired.");

    // farmer1 add liqidity 
    add_liqudity(&farmer1, &pool, &token1, &token2, 0);
    assert_eq!(
        view!(pool.mft_balance_of("0".to_string(), to_va(farmer1.account_id.clone())))
            .unwrap_json::<U128>()
            .0,
        to_yocto("1")
    );
    println!("----->> Liquidity added by farmer1.");

    // create farm
    println!("----->> Creating farm.");
    let (farming, farm_id) = prepair_farm(&root, &owner, &token1, to_yocto("500"));
    let farm_info = show_farminfo(&farming, farm_id.clone(), false);
    assert_farming(&farm_info, "Running".to_string(), to_yocto("500"), 0, 0, 0, 0, 0);

    // register LP for farming contract
    println!("---->> Registering LP 0 for {}.", farming_id());
    call!(root, pool.mft_register("0".to_string(), to_va(farming_id())), deposit = to_yocto("1"))
    .assert_success();
    
    // farmer1 register and stake liquidity token
    println!("---->> Step01: Farmer1 register and stake liquidity token.");
    call!(farmer1, farming.storage_deposit(None, None), deposit = to_yocto("1"))
    .assert_success();
    let out_come = call!(
        farmer1,
        pool.mft_transfer_call("0".to_string(), to_va(farming_id()), to_yocto("1").into(), None, "".to_string()),
        deposit = 1
    );
    out_come.assert_success();
    // println!("{:#?}", out_come.promise_results());
    let farm_info = show_farminfo(&farming, farm_id.clone(), false);
    assert_farming(&farm_info, "Running".to_string(), to_yocto("500"), 0, 0, 0, 0, 0);
    let unclaim = show_unclaim(&farming, farmer1.account_id(), farm_id.clone(), false);
    assert_eq!(unclaim.0, 0_u128);
    println!("  Farmer1 staked liquidity at #{}.", root.borrow_runtime().current_block().block_height);

    println!("---->> Step02: Farmer1 unstake seeds after 60 blocks************");
    assert!(root.borrow_runtime_mut().produce_blocks(60).is_ok());
    println!("  Chain goes for 60 blocks *** now height: {}", root.borrow_runtime().current_block().block_height);
    let out_come = call!(
        farmer1,
        farming.withdraw_seed(format!("{}@0", swap()), to_yocto("1").into()),
        deposit = 1
    );
    out_come.assert_success();
    let farm_info = show_farminfo(&farming, farm_id.clone(), false);
    assert_farming(&farm_info, "Running".to_string(), to_yocto("500"), 1, 1, to_yocto("1"), 0, 0);
    println!("  Farmer1 unstake seeds at #{}.", root.borrow_runtime().current_block().block_height);

    println!("---->> Step03: Farmer1 staking liquidity again after 120 blocks.");
    assert!(root.borrow_runtime_mut().produce_blocks(120).is_ok());
    println!("  Chain goes for 120 blocks *** now height: {}", root.borrow_runtime().current_block().block_height);
    let farm_info = show_farminfo(&farming, farm_id.clone(), false);
    assert_farming(&farm_info, "Running".to_string(), to_yocto("500"), 3, 1, to_yocto("1"), to_yocto("2"), 0);
    let out_come = call!(
        farmer1,
        pool.mft_transfer_call("0".to_string(), to_va(farming_id()), to_yocto("0.5").into(), None, "".to_string()),
        deposit = 1
    );
    out_come.assert_success();
    let farm_info = show_farminfo(&farming, farm_id.clone(), false);
    assert_farming(&farm_info, "Running".to_string(), to_yocto("500"), 3, 3, to_yocto("3"), 0, to_yocto("2"));
    println!("  Farmer1 staked liquidity again at #{}.", root.borrow_runtime().current_block().block_height);

    println!("---->> Step04: Farmer1 append staking liquidity after 60 blocks.");
    assert!(root.borrow_runtime_mut().produce_blocks(60).is_ok());
    println!("  Chain goes for 60 blocks *** now height: {}", root.borrow_runtime().current_block().block_height);
    let out_come = call!(
        farmer1,
        pool.mft_transfer_call("0".to_string(), to_va(farming_id()), to_yocto("0.5").into(), None, "".to_string()),
        deposit = 1
    );
    out_come.assert_success();
    let farm_info = show_farminfo(&farming, farm_id.clone(), false);
    assert_farming(&farm_info, "Running".to_string(), to_yocto("500"), 4, 4, to_yocto("4"), 0, to_yocto("2"));
    println!("  Farmer1 append staking liquidity at #{}.", root.borrow_runtime().current_block().block_height);

    println!("---->> Step05: Farmer1 unstake half seeds after 60 blocks************");
    assert!(root.borrow_runtime_mut().produce_blocks(60).is_ok());
    println!("  Chain goes for 60 blocks *** now height: {}", root.borrow_runtime().current_block().block_height);
    let out_come = call!(
        farmer1,
        farming.withdraw_seed(format!("{}@0", swap()), to_yocto("0.5").into()),
        deposit = 1
    );
    out_come.assert_success();
    let farm_info = show_farminfo(&farming, farm_id.clone(), false);
    assert_farming(&farm_info, "Running".to_string(), to_yocto("500"), 5, 5, to_yocto("5"), 0, to_yocto("2"));
    println!("  Farmer1 unstake half seeds at #{}.", root.borrow_runtime().current_block().block_height);

    println!("---->> Step06: Farmer1 unstake another half seeds after 60 blocks************");
    assert!(root.borrow_runtime_mut().produce_blocks(60).is_ok());
    println!("  Chain goes for 60 blocks *** now height: {}", root.borrow_runtime().current_block().block_height);
    let out_come = call!(
        farmer1,
        farming.withdraw_seed(format!("{}@0", swap()), to_yocto("0.5").into()),
        deposit = 1
    );
    out_come.assert_success();
    let farm_info = show_farminfo(&farming, farm_id.clone(), false);
    assert_farming(&farm_info, "Running".to_string(), to_yocto("500"), 6, 6, to_yocto("6"), 0, to_yocto("2"));
    println!("  Farmer1 unstake another half seeds at #{}.", root.borrow_runtime().current_block().block_height);

    println!("---->> Step07: Farmer1 staking liquidity after 60 blocks.");
    assert!(root.borrow_runtime_mut().produce_blocks(60).is_ok());
    println!("  Chain goes for 60 blocks *** now height: {}", root.borrow_runtime().current_block().block_height);
    let out_come = call!(
        farmer1,
        pool.mft_transfer_call("0".to_string(), to_va(farming_id()), to_yocto("1").into(), None, "".to_string()),
        deposit = 1
    );
    out_come.assert_success();
    let farm_info = show_farminfo(&farming, farm_id.clone(), false);
    assert_farming(&farm_info, "Running".to_string(), to_yocto("500"), 7, 7, to_yocto("7"), 0, to_yocto("3"));
    println!("  Farmer1 staking liquidity at #{}.", root.borrow_runtime().current_block().block_height);


    println!("----->> Step08: Farmer1 claiming reward by farm_id after 60 blocks ************");
    assert!(root.borrow_runtime_mut().produce_blocks(60).is_ok());
    println!("  Chain goes for 60 blocks *** now height: {}", root.borrow_runtime().current_block().block_height);
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
    println!("  Farmer1 claimed reward at #{}.", root.borrow_runtime().current_block().block_height);
}

