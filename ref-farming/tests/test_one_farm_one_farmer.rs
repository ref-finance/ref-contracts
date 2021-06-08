use near_sdk_sim::{call, init_simulator, to_yocto, view};
use near_sdk::json_types::{U128};
use crate::common::*;

mod common;

#[test]
fn test_one_farm_one_farmers() {
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
    let (farming, farm_id) = prepair_farm(&root, &owner, &token1);
    show_seedsinfo(&farming);
    println!("----->> Farm {} is ready.", farm_id.clone());

    // register LP for farming contract
    call!(root, pool.mft_register("0".to_string(), to_va(farming_id())), deposit = to_yocto("1"))
    .assert_success();
    println!("Registered LP 0 for {}.", farming_id());
    // farmer1 register and stake liquidity token
    call!(farmer1, farming.storage_deposit(None, None), deposit = to_yocto("1"))
    .assert_success();
    let out_come = call!(
        farmer1,
        pool.mft_transfer_call("0".to_string(), to_va(farming_id()), to_yocto("1").into(), None, "".to_string()),
        deposit = 1
    );
    out_come.assert_success();
    // println!("{:#?}", out_come.promise_results());
    let farm_info = show_farminfo(&farming, farm_id.clone());
    assert_eq!(farm_info.cur_round.0, 0_u64);
    assert_eq!(farm_info.last_round.0, 0_u64);
    assert_eq!(farm_info.claimed_reward.0, 0_u128);
    assert_eq!(farm_info.unclaimed_reward.0, 0_u128);
    let user_seeds = show_userseeds(&farming, farmer1.account_id());
    assert_eq!(user_seeds.get(&String::from("swap@0")).unwrap().0, to_yocto("1"));
    let unclaim = show_unclaim(&farming, farmer1.account_id(), farm_id.clone());
    assert_eq!(unclaim.0, 0_u128);
    show_seedsinfo(&farming);
    println!("----->> Farmer1 staked liquidity at #{}.", root.borrow_runtime().current_block().block_height);

    // chain goes for 60 blocks
    if root.borrow_runtime_mut().produce_blocks(60).is_ok() {
        println!();
        println!("*** Chain goes for 60 blocks *** now height: {}", 
            root.borrow_runtime().current_block().block_height,
        );
        let unclaim = show_unclaim(&farming, farmer1.account_id(), farm_id.clone());
        assert_eq!(unclaim.0, to_yocto("1"));
    }

    let farm_info = show_farminfo(&farming, farm_id.clone());
    // assert_eq!(farm_info.cur_round.0, 1_u64);
    // assert_eq!(farm_info.last_round.0, 1_u64);
    // assert_eq!(farm_info.claimed_reward.0, 0_u128);
    // assert_eq!(farm_info.unclaimed_reward.0, to_yocto("1"));

    // chain goes for another 60 blocks
    if root.borrow_runtime_mut().produce_blocks(60).is_ok() {
        println!();
        println!("*** Chain goes for 60 blocks *** now height: {}", 
            root.borrow_runtime().current_block().block_height
        );
        let unclaim = show_unclaim(&farming, farmer1.account_id(), farm_id.clone());
        assert_eq!(unclaim.0, to_yocto("2"));
    }

    // farmer1 claim reward
    println!();
    println!("********** Farmer1 claim reward by farm_id ************");
    let out_come = call!(
        farmer1,
        farming.claim_reward_by_farm(farm_id.clone()),
        deposit = 0
    );
    out_come.assert_success();
    let farm_info = show_farminfo(&farming, farm_id.clone());
    let unclaim = show_unclaim(&farming, farmer1.account_id(), farm_id.clone());
    assert_eq!(unclaim.0, 0_u128);
    println!("----->> Farmer1 claimed reward at #{}.", root.borrow_runtime().current_block().block_height);

    // chain goes for 20 blocks
    if root.borrow_runtime_mut().produce_blocks(20).is_ok() {
        println!();
        println!("*** Chain goes for 20 blocks *** now height: {}", 
            root.borrow_runtime().current_block().block_height
        );
        let unclaim = show_unclaim(&farming, farmer1.account_id(), farm_id.clone());
        assert_eq!(unclaim.0, to_yocto("0"));
    }

    // farmer1 claim reward 
    println!();
    println!("********** Farmer1 claim reward again by farm_id ************");
    let out_come = call!(
        farmer1,
        farming.claim_reward_by_farm(farm_id.clone()),
        deposit = 0
    );
    out_come.assert_success();
    let farm_info = show_farminfo(&farming, farm_id.clone());
    let unclaim = show_unclaim(&farming, farmer1.account_id(), farm_id.clone());
    assert_eq!(unclaim.0, 0_u128);
    println!("----->> Farmer1 claimed reward again at #{}.", root.borrow_runtime().current_block().block_height);

    // chain goes for 60 blocks
    if root.borrow_runtime_mut().produce_blocks(60).is_ok() {
        println!();
        println!("*** Chain goes for 60 blocks *** now height: {}", 
            root.borrow_runtime().current_block().block_height
        );
        let unclaim = show_unclaim(&farming, farmer1.account_id(), farm_id.clone());
        assert_eq!(unclaim.0, to_yocto("1"));
    }

    // chain goes for 60 blocks
    if root.borrow_runtime_mut().produce_blocks(60).is_ok() {
        println!();
        println!("*** Chain goes for 60 blocks *** now height: {}", 
            root.borrow_runtime().current_block().block_height
        );
        let unclaim = show_unclaim(&farming, farmer1.account_id(), farm_id.clone());
        assert_eq!(unclaim.0, to_yocto("2"));
    }

    // chain goes for 60 blocks
    if root.borrow_runtime_mut().produce_blocks(60).is_ok() {
        println!();
        println!("*** Chain goes for 60 blocks *** now height: {}", 
            root.borrow_runtime().current_block().block_height
        );
        let unclaim = show_unclaim(&farming, farmer1.account_id(), farm_id.clone());
        assert_eq!(unclaim.0, to_yocto("3"));
    }

    // farmer1 claim reward
    println!();
    println!("********** Farmer1 claim reward by seed_id ************");
    let out_come = call!(
        farmer1,
        farming.claim_reward_by_seed(farm_info.seed_id.clone()),
        deposit = 0
    );
    out_come.assert_success();
    let farm_info = show_farminfo(&farming, farm_id.clone());
    let unclaim = show_unclaim(&farming, farmer1.account_id(), farm_id.clone());
    assert_eq!(unclaim.0, 0_u128);
    println!("----->> Farmer1 claimed reward at #{}.", root.borrow_runtime().current_block().block_height);

    // chain goes for 60 blocks
    if root.borrow_runtime_mut().produce_blocks(60).is_ok() {
        println!();
        println!("*** Chain goes for 60 blocks *** now height: {}", 
            root.borrow_runtime().current_block().block_height
        );
        let unclaim = show_unclaim(&farming, farmer1.account_id(), farm_id.clone());
        assert_eq!(unclaim.0, to_yocto("1"));
    }
}