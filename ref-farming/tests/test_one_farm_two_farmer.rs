use near_sdk_sim::{call, init_simulator, to_yocto, view};
use near_sdk::json_types::{U128};
use crate::common::*;

mod common;

#[test]
fn test_one_farm_two_farmers() {
    let root = init_simulator(None);

    // prepair users
    let owner = root.create_user("owner".to_string(), to_yocto("100"));
    let farmer1 = root.create_user("farmer1".to_string(), to_yocto("100"));
    let farmer2 = root.create_user("farmer2".to_string(), to_yocto("100"));
    println!("----->> Four accounts prepaired.");

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

    // farmer2 add liqidity 
    add_liqudity(&farmer2, &pool, &token1, &token2, 0);
    assert_eq!(
        view!(pool.mft_balance_of("0".to_string(), to_va(farmer2.account_id.clone())))
            .unwrap_json::<U128>()
            .0,
        to_yocto("1")
    );
    println!("----->> Liquidity added by farmer2.");

    // create farm
    let (farming, farm_id) = prepair_farm(&root, &owner, &token1);
    println!("----->> Farm {} is ready.", farm_id.clone());

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
    show_farminfo(&farming, farm_id.clone());
    show_userseeds(&farming, farmer1.account_id());
    show_unclaim(&farming, farmer1.account_id(), farm_id.clone());
    println!("----->> Farmer1 staked liquidity at #{}.", root.borrow_runtime().current_block().block_height);

    // chain goes for 60 blocks
    if root.borrow_runtime_mut().produce_blocks(60).is_ok() {
        println!();
        println!("*** Chain goes for 60 blocks *** now height: {}", 
            root.borrow_runtime().current_block().block_height);
        show_unclaim(&farming, farmer1.account_id(), farm_id.clone());
    }

    // farmer2 register and stake liquidity token
    call!(farmer2, farming.storage_deposit(None, None), deposit = to_yocto("1"))
    .assert_success();
    let out_come = call!(
        farmer2,
        pool.mft_transfer_call("0".to_string(), to_va(farming_id()), to_yocto("1").into(), None, "".to_string()),
        deposit = 1
    );
    out_come.assert_success();
    // println!("{:#?}", out_come.promise_results());
    show_farminfo(&farming, farm_id.clone());
    show_userseeds(&farming, farmer2.account_id());
    show_unclaim(&farming, farmer1.account_id(), farm_id.clone());
    show_unclaim(&farming, farmer2.account_id(), farm_id.clone());
    println!("----->> Farmer2 staked liquidity at #{}.", root.borrow_runtime().current_block().block_height);

    // chain goes for 60 blocks
    if root.borrow_runtime_mut().produce_blocks(60).is_ok() {
        println!();
        println!("*** Chain goes for 60 blocks *** now height: {}", 
            root.borrow_runtime().current_block().block_height);
        show_unclaim(&farming, farmer1.account_id(), farm_id.clone());
        show_unclaim(&farming, farmer2.account_id(), farm_id.clone());
    }

    // farmer1 claim reward
    println!();
    println!("********** Farmer1 claim reward by farm_id ************");
    let out_come = call!(
        farmer1,
        farming.claim_reward_by_farm(farm_id.clone()),
        deposit = 1
    );
    out_come.assert_success();
    show_farminfo(&farming, farm_id.clone());
    show_userseeds(&farming, farmer1.account_id());
    show_unclaim(&farming, farmer1.account_id(), farm_id.clone());
    println!("----->> Farmer1 claimed reward at #{}.", root.borrow_runtime().current_block().block_height);

    // chain goes for 60 blocks
    if root.borrow_runtime_mut().produce_blocks(60).is_ok() {
        println!();
        println!("*** Chain goes for 60 blocks *** now height: {}", 
            root.borrow_runtime().current_block().block_height);
        show_unclaim(&farming, farmer1.account_id(), farm_id.clone());
        show_unclaim(&farming, farmer2.account_id(), farm_id.clone());
    }

    // farmer1 unstake
    println!();
    println!("********** Farmer1 unstake seeds ************");
    let out_come = call!(
        farmer1,
        farming.withdraw_seed(format!("{}@0", swap()), to_yocto("1").into()),
        deposit = 1
    );
    out_come.assert_success();
    // println!("{:#?}", out_come.promise_results());
    show_userseeds(&farming, farmer1.account_id());
    show_unclaim(&farming, farmer1.account_id(), farm_id.clone());
    show_unclaim(&farming, farmer2.account_id(), farm_id.clone());
    println!("----->> Farmer1 unstake seeds at #{}.", root.borrow_runtime().current_block().block_height);

    // chain goes for 60 blocks
    if root.borrow_runtime_mut().produce_blocks(60).is_ok() {
        println!();
        println!("*** Chain goes for 60 blocks *** now height: {}", 
            root.borrow_runtime().current_block().block_height);
        show_unclaim(&farming, farmer1.account_id(), farm_id.clone());
        show_unclaim(&farming, farmer2.account_id(), farm_id.clone());
    }

    // farmer1 withdraw reward
    println!();
    println!("********** Farmer1 withdraw reward ************");
    show_reward(&farming, farmer1.account_id(), dai());
    let out_come = call!(
        farmer1,
        farming.withdraw_reward(to_va(dai()), None),
        deposit = 1
    );
    out_come.assert_success();
    println!("----->> Farmer1 withdraw reward at #{}.", root.borrow_runtime().current_block().block_height);
    show_reward(&farming, farmer1.account_id(), dai());
}