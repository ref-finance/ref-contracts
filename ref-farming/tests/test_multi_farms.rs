use near_sdk_sim::{call, init_simulator, to_yocto, view};
use near_sdk::json_types::{U128};

use crate::common::utils::*;
use crate::common::views::*;
use crate::common::actions::*;

mod common;

#[test]
fn multi_farm_in_single_seed() {
    let root = init_simulator(None);

    // prepair users
    let owner = root.create_user("owner".to_string(), to_yocto("100"));
    let farmer = root.create_user("farmer".to_string(), to_yocto("100"));
    println!("----->> owner and farmer prepaired.");

    // prepair pool and tokens
    let(pool, token1, token2) = prepair_pool(&root, &owner);
    println!("----->> The pool prepaired.");

    // farmer add liqidity 
    add_liqudity(&farmer, &pool, &token1, &token2, 0);
    assert_eq!(
        view!(pool.mft_balance_of("0".to_string(), to_va(farmer.account_id.clone())))
            .unwrap_json::<U128>()
            .0,
        to_yocto("1")
    );
    println!("----->> Liquidity added by farmer.");

    // create farm with token1
    let (farming, farm_ids) = prepair_multi_farms(&root, &owner, &token1, to_yocto("10"), 10);
    let farm_id = farm_ids[farm_ids.len() - 1].clone();
    println!("----->> Farm till {} is ready.", farm_id.clone());

    // register LP token to farming contract
    call!(root, pool.mft_register("0".to_string(), to_va(farming_id())), deposit = to_yocto("1"))
    .assert_success();
    println!("----->> Registered LP 0 to {}.", farming_id());
    // register farmer to farming contract and stake liquidity token
    call!(farmer, farming.storage_deposit(None, None), deposit = to_yocto("1"))
    .assert_success();
    println!("----->> Registered farmer to {}.", farming_id());
    let out_come = call!(
        farmer,
        pool.mft_transfer_call("0".to_string(), to_va(farming_id()), to_yocto("1").into(), None, "".to_string()),
        deposit = 1
    );
    out_come.assert_success();
    // println!("{:#?}", out_come.promise_results());
    let farm_info = show_farminfo(&farming, farm_id.clone(), false);
    assert_farming(&farm_info, "Running".to_string(), to_yocto("10"), 0, 0, 0, 0);
    let user_seeds = show_userseeds(&farming, farmer.account_id(), false);
    assert_eq!(user_seeds.get(&String::from("swap@0")).unwrap().0, to_yocto("1"));
    let unclaim = show_unclaim(&farming, farmer.account_id(), farm_id.clone(), false);
    assert_eq!(unclaim.0, 0_u128);
    show_seedsinfo(&farming, false);
    println!("----->> Farmer staked liquidity at #{}.", root.borrow_runtime().current_block().block_height);

    // chain goes for 60 blocks
    if root.borrow_runtime_mut().produce_blocks(60).is_ok() {
        println!();
        println!("*** Chain goes for 60 blocks *** now height: {}", 
            root.borrow_runtime().current_block().block_height,
        );
        let farm_info = show_farminfo(&farming, farm_id.clone(), false);
        assert_farming(&farm_info, "Running".to_string(), to_yocto("10"), 1, 0, 0, to_yocto("1"));
        let unclaim = show_unclaim(&farming, farmer.account_id(), farm_id.clone(), false);
        assert_eq!(unclaim.0, to_yocto("1"));
    }

    // farmer claim reward
    println!();
    println!("********** Farmer claim reward by seed_id ************");

    let out_come = call!(
        farmer,
        farming.claim_reward_by_seed(String::from("swap@0")),
        deposit = 0
    );
    out_come.assert_success();
    // println!("{:#?}", out_come.promise_results());
    // println!(
    //     "profile_data: {:#?} \n\ntokens_burnt: {} Near", 
    //     out_come.profile_data(), 
    //     (out_come.tokens_burnt()) as f64 / 1e24
    // );
    println!("\ntokens_burnt: {} Near", (out_come.tokens_burnt()) as f64 / 1e24);
    println!("Gas_burnt: {} TGas \n", (out_come.gas_burnt()) as f64 / 1e12);
    // make sure the total gas is less then 100T
    assert!(out_come.gas_burnt() < 100 * u64::pow(10, 12));

    // println!("profile_data: {:#?} \n", out_come.profile_data());
    let farm_info = show_farminfo(&farming, farm_id.clone(), false);
    assert_farming(&farm_info, "Running".to_string(), to_yocto("10"), 1, 1, to_yocto("1"), 0);
    let unclaim = show_unclaim(&farming, farmer.account_id(), farm_id.clone(), false);
    assert_eq!(unclaim.0, 0_u128);
    println!("----->> Farmer claimed reward at #{}.", root.borrow_runtime().current_block().block_height);

    let farms_info = show_farms_by_seed(&farming, String::from("swap@0"), false);

    // // chain goes for 60 blocks
    // if root.borrow_runtime_mut().produce_blocks(60).is_ok() {
    //     println!();
    //     println!("*** Chain goes for 60 blocks *** now height: {}", 
    //         root.borrow_runtime().current_block().block_height,
    //     );
    //     let farm_info = show_farminfo(&farming, farm_id.clone(), false);
    //     assert_farming(&farm_info, "Ended".to_string(), to_yocto("10"), 10, 10, to_yocto("10"), 0);
    // }

}