use near_sdk_sim::{call, init_simulator, to_yocto, view};
use near_sdk::json_types::{U128};
use near_sdk::serde_json::Value;

use ref_farming::{HRSimpleFarmTerms};

use crate::common::utils::*;
use crate::common::views::*;
use crate::common::actions::*;
use crate::common::init::deploy_farming;

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
        view!(pool.mft_balance_of(":0".to_string(), to_va(farmer.account_id.clone())))
            .unwrap_json::<U128>()
            .0,
        to_yocto("1")
    );
    println!("----->> Liquidity added by farmer.");

    // create farm with token1
    let (farming, farm_ids) = prepair_multi_farms(&root, &owner, &token1, to_yocto("10"), 32);
    let farm_id = farm_ids[farm_ids.len() - 1].clone();
    println!("----->> Farm till {} is ready.", farm_id.clone());

    // register LP token to farming contract
    call!(root, pool.mft_register(":0".to_string(), to_va(farming_id())), deposit = to_yocto("1"))
    .assert_success();
    println!("----->> Registered LP 0 to {}.", farming_id());
    // register farmer to farming contract and stake liquidity token
    call!(farmer, farming.storage_deposit(None, None), deposit = to_yocto("1"))
    .assert_success();
    println!("----->> Registered farmer to {}.", farming_id());
    let out_come = call!(
        farmer,
        pool.mft_transfer_call(":0".to_string(), to_va(farming_id()), to_yocto("0.5").into(), None, "".to_string()),
        deposit = 1
    );
    out_come.assert_success();
    // println!("{:#?}", out_come.promise_results());
    let farm_info = show_farminfo(&farming, farm_id.clone(), false);
    assert_farming(&farm_info, "Running".to_string(), to_yocto("10"), 0, 0, 0, 0, 0);
    let user_seeds = show_userseeds(&farming, farmer.account_id(), false);
    assert_eq!(user_seeds.get(&String::from("swap@0")).unwrap().0, to_yocto("0.5"));
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
        assert_farming(&farm_info, "Running".to_string(), to_yocto("10"), 1, 0, 0, to_yocto("1"), 0);
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
    // make sure the total gas is less then 300T
    assert!(out_come.gas_burnt() < 300 * u64::pow(10, 12));

    // println!("profile_data: {:#?} \n", out_come.profile_data());
    let farm_info = show_farminfo(&farming, farm_id.clone(), false);
    assert_farming(&farm_info, "Running".to_string(), to_yocto("10"), 1, 1, to_yocto("1"), 0, 0);
    let unclaim = show_unclaim(&farming, farmer.account_id(), farm_id.clone(), false);
    assert_eq!(unclaim.0, 0_u128);
    println!("----->> Farmer claimed reward at #{}.", root.borrow_runtime().current_block().block_height);

    // chain goes for 60 blocks
    if root.borrow_runtime_mut().produce_blocks(60).is_ok() {
        println!();
        println!("*** Chain goes for 60 blocks *** now height: {}", 
            root.borrow_runtime().current_block().block_height,
        );
        let farm_info = show_farminfo(&farming, farm_id.clone(), false);
        assert_farming(&farm_info, "Running".to_string(), to_yocto("10"), 2, 1, to_yocto("1"), to_yocto("1"), 0);
        let unclaim = show_unclaim(&farming, farmer.account_id(), farm_id.clone(), false);
        assert_eq!(unclaim.0, to_yocto("1"));
    }

    // add lptoken
    println!();
    println!("********** Farmer add seed ************");
    let out_come = call!(
        farmer,
        pool.mft_transfer_call(":0".to_string(), to_va(farming_id()), to_yocto("0.5").into(), None, "".to_string()),
        deposit = 1
    );
    out_come.assert_success();
    println!("\ntokens_burnt: {} Near", (out_come.tokens_burnt()) as f64 / 1e24);
    println!("Gas_burnt: {} TGas \n", (out_come.gas_burnt()) as f64 / 1e12);

    let user_seeds = show_userseeds(&farming, farmer.account_id(), false);
    assert_eq!(user_seeds.get(&String::from("swap@0")).unwrap().0, to_yocto("1"));
    let farm_info = show_farminfo(&farming, farm_id.clone(), false);
    assert_farming(&farm_info, "Running".to_string(), to_yocto("10"), 2, 2, to_yocto("2"), 0, 0);
    let unclaim = show_unclaim(&farming, farmer.account_id(), farm_id.clone(), false);
    assert_eq!(unclaim.0, 0_u128);
    println!("----->> Farmer added seed at #{}.", root.borrow_runtime().current_block().block_height);

}

#[test]
fn multi_farm_with_different_state() {
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
        view!(pool.mft_balance_of(":0".to_string(), to_va(farmer.account_id.clone())))
            .unwrap_json::<U128>()
            .0,
        to_yocto("1")
    );
    println!("----->> Liquidity added by farmer.");

    println!("----->> Deploying farming contract.");
    let farming = deploy_farming(&root, farming_id(), owner.account_id());
    call!(
        root,
        token1.storage_deposit(Some(to_va(farming_id())), None),
        deposit = to_yocto("1")
    )
    .assert_success();

    println!("----->> Creating farm0.");
    let out_come = call!(
        owner,
        farming.create_simple_farm(HRSimpleFarmTerms{
            seed_id: format!("{}@0", swap()),
            reward_token: token1.valid_account_id(),
            start_at: 0,
            reward_per_session: to_yocto("1").into(),
            session_interval: 50,
        }, Some(U128(1000000000000000000))),
        deposit = to_yocto("1")
    );
    out_come.assert_success();
    let farm0_id: String;
    if let Value::String(farmid) = out_come.unwrap_json_value() {
        farm0_id = farmid.clone();
    } else {
        farm0_id = String::from("N/A");
    }
    println!("    Farm {} created at Height#{}", farm0_id.clone(), root.borrow_runtime().current_block().block_height);
    mint_token(&token1, &root, to_yocto("5000"));
    call!(
        root,
        token1.ft_transfer_call(to_va(farming_id()), to_yocto("500").into(), None, farm0_id.clone()),
        deposit = 1
    )
    .assert_success();
    println!("    Farm {} running at Height#{}", farm0_id.clone(), root.borrow_runtime().current_block().block_height);

    println!("----->> Creating farm1.");
    let out_come = call!(
        owner,
        farming.create_simple_farm(HRSimpleFarmTerms{
            seed_id: format!("{}@0", swap()),
            reward_token: token1.valid_account_id(),
            start_at: 0,
            reward_per_session: to_yocto("1").into(),
            session_interval: 50,
        }, Some(U128(1000000000000000000))),
        deposit = to_yocto("1")
    );
    out_come.assert_success();
    let farm1_id: String;
    if let Value::String(farmid) = out_come.unwrap_json_value() {
        farm1_id = farmid.clone();
    } else {
        farm1_id = String::from("N/A");
    }
    println!("    Farm {} created at Height#{}", farm1_id.clone(), root.borrow_runtime().current_block().block_height);
    
    println!("----->> Creating farm2.");
    let out_come = call!(
        owner,
        farming.create_simple_farm(HRSimpleFarmTerms{
            seed_id: format!("{}@0", swap()),
            reward_token: token1.valid_account_id(),
            start_at: 300,
            reward_per_session: to_yocto("1").into(),
            session_interval: 50,
        }, Some(U128(1000000000000000000))),
        deposit = to_yocto("1")
    );
    out_come.assert_success();
    let farm2_id: String;
    if let Value::String(farmid) = out_come.unwrap_json_value() {
        farm2_id = farmid.clone();
    } else {
        farm2_id = String::from("N/A");
    }
    println!("    Farm {} created at Height#{}", farm2_id.clone(), root.borrow_runtime().current_block().block_height);
    call!(
        root,
        token1.ft_transfer_call(to_va(farming_id()), to_yocto("500").into(), None, farm2_id.clone()),
        deposit = 1
    )
    .assert_success();
    println!("    Farm {} deposit reward at Height#{}", farm2_id.clone(), root.borrow_runtime().current_block().block_height);

    println!("---->> Registering LP 0 for {}.", farming_id());
    call!(root, pool.mft_register(":0".to_string(), to_va(farming_id())), deposit = to_yocto("1"))
    .assert_success();

    println!("---->> Step01: Farmer register and stake liquidity token.");
    call!(farmer, farming.storage_deposit(None, None), deposit = to_yocto("1"))
    .assert_success();
    let out_come = call!(
        farmer,
        pool.mft_transfer_call(":0".to_string(), to_va(farming_id()), to_yocto("1").into(), None, "".to_string()),
        deposit = 1
    );
    out_come.assert_success();
    // println!("{:#?}", out_come.promise_results());
    let farm_info = show_farminfo(&farming, farm0_id.clone(), false);
    assert_farming(&farm_info, "Running".to_string(), to_yocto("500"), 0, 0, 0, 0, 0);
    let unclaim = show_unclaim(&farming, farmer.account_id(), farm0_id.clone(), false);
    assert_eq!(unclaim.0, 0_u128);
    let farm_info = show_farminfo(&farming, farm1_id.clone(), false);
    assert_farming(&farm_info, "Created".to_string(), to_yocto("0"), 0, 0, 0, 0, 0);
    let unclaim = show_unclaim(&farming, farmer.account_id(), farm1_id.clone(), false);
    assert_eq!(unclaim.0, 0_u128);
    println!("  Farmer staked liquidity at #{}.", root.borrow_runtime().current_block().block_height);

    println!("----->> Step02: Farmer claiming reward by seed_id after 50 blocks ************");
    assert!(root.borrow_runtime_mut().produce_blocks(50).is_ok());
    println!("  Chain goes for 50 blocks *** now height: {}", root.borrow_runtime().current_block().block_height);
    let farm_info = show_farminfo(&farming, farm0_id.clone(), false);
    assert_farming(&farm_info, "Running".to_string(), to_yocto("500"), 1, 0, to_yocto("0"), to_yocto("1"), to_yocto("0"));
    let unclaim = show_unclaim(&farming, farmer.account_id(), farm0_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("1"));
    let farm_info = show_farminfo(&farming, farm1_id.clone(), false);
    assert_farming(&farm_info, "Created".to_string(), to_yocto("0"), 0, 0, to_yocto("0"), to_yocto("0"), to_yocto("0"));
    let unclaim = show_unclaim(&farming, farmer.account_id(), farm1_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("0"));
    let out_come = call!(
        farmer,
        farming.claim_reward_by_seed(format!("{}@0", swap())),
        deposit = 0
    );
    out_come.assert_success();
    let farm_info = show_farminfo(&farming, farm0_id.clone(), false);
    assert_farming(&farm_info, "Running".to_string(), to_yocto("500"), 1, 1, to_yocto("1"), 0, to_yocto("0"));
    let unclaim = show_unclaim(&farming, farmer.account_id(), farm0_id.clone(), false);
    assert_eq!(unclaim.0, 0_u128);
    let farm_info = show_farminfo(&farming, farm1_id.clone(), false);
    assert_farming(&farm_info, "Created".to_string(), to_yocto("0"), 0, 0, to_yocto("0"), to_yocto("0"), to_yocto("0"));
    let unclaim = show_unclaim(&farming, farmer.account_id(), farm1_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("0"));
    println!("  Farmer claimed reward at #{}.", root.borrow_runtime().current_block().block_height);

    println!("----->> Step03: Active farm1 after 50 blocks ************");
    assert!(root.borrow_runtime_mut().produce_blocks(50).is_ok());
    println!("  Chain goes for 50 blocks *** now height: {}", root.borrow_runtime().current_block().block_height);
    let farm_info = show_farminfo(&farming, farm0_id.clone(), false);
    assert_farming(&farm_info, "Running".to_string(), to_yocto("500"), 2, 1, to_yocto("1"), to_yocto("1"), to_yocto("0"));
    let unclaim = show_unclaim(&farming, farmer.account_id(), farm0_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("1"));
    let farm_info = show_farminfo(&farming, farm1_id.clone(), false);
    assert_farming(&farm_info, "Created".to_string(), to_yocto("0"), 0, 0, to_yocto("0"), to_yocto("0"), to_yocto("0"));
    let unclaim = show_unclaim(&farming, farmer.account_id(), farm1_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("0"));
    call!(
        root,
        token1.ft_transfer_call(to_va(farming_id()), to_yocto("500").into(), None, farm1_id.clone()),
        deposit = 1
    )
    .assert_success();
    let farm_info = show_farminfo(&farming, farm1_id.clone(), false);
    assert_farming(&farm_info, "Running".to_string(), to_yocto("500"), 0, 0, to_yocto("0"), to_yocto("0"), to_yocto("0"));
    println!("    Farm {} running at Height#{}", farm1_id.clone(), root.borrow_runtime().current_block().block_height);

    println!("----->> Step04: Farmer claiming reward by seed_id after 50 blocks ************");
    assert!(root.borrow_runtime_mut().produce_blocks(50).is_ok());
    println!("  Chain goes for 50 blocks *** now height: {}", root.borrow_runtime().current_block().block_height);
    let farm_info = show_farminfo(&farming, farm0_id.clone(), false);
    assert_farming(&farm_info, "Running".to_string(), to_yocto("500"), 3, 1, to_yocto("1"), to_yocto("2"), to_yocto("0"));
    let unclaim = show_unclaim(&farming, farmer.account_id(), farm0_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("2"));
    let farm_info = show_farminfo(&farming, farm1_id.clone(), false);
    assert_farming(&farm_info, "Running".to_string(), to_yocto("500"), 1, 0, to_yocto("0"), to_yocto("1"), to_yocto("0"));
    let unclaim = show_unclaim(&farming, farmer.account_id(), farm1_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("1"));
    let out_come = call!(
        farmer,
        farming.claim_reward_by_seed(format!("{}@0", swap())),
        deposit = 0
    );
    out_come.assert_success();
    let farm_info = show_farminfo(&farming, farm0_id.clone(), false);
    assert_farming(&farm_info, "Running".to_string(), to_yocto("500"), 3, 3, to_yocto("3"), 0, to_yocto("0"));
    let unclaim = show_unclaim(&farming, farmer.account_id(), farm0_id.clone(), false);
    assert_eq!(unclaim.0, 0_u128);
    let farm_info = show_farminfo(&farming, farm1_id.clone(), false);
    assert_farming(&farm_info, "Running".to_string(), to_yocto("500"), 1, 1, to_yocto("1"), to_yocto("0"), to_yocto("0"));
    let unclaim = show_unclaim(&farming, farmer.account_id(), farm1_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("0"));
    println!("  Farmer claimed reward at #{}.", root.borrow_runtime().current_block().block_height);

    println!("----->> Step05: Farmer claiming reward by seed_id after 100 blocks ************");
    assert!(root.borrow_runtime_mut().produce_blocks(100).is_ok());
    println!("  Chain goes for 100 blocks *** now height: {}", root.borrow_runtime().current_block().block_height);
    let farm_info = show_farminfo(&farming, farm0_id.clone(), false);
    assert_farming(&farm_info, "Running".to_string(), to_yocto("500"), 5, 3, to_yocto("3"), to_yocto("2"), to_yocto("0"));
    let unclaim = show_unclaim(&farming, farmer.account_id(), farm0_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("2"));
    let farm_info = show_farminfo(&farming, farm1_id.clone(), false);
    assert_farming(&farm_info, "Running".to_string(), to_yocto("500"), 3, 1, to_yocto("1"), to_yocto("2"), to_yocto("0"));
    let unclaim = show_unclaim(&farming, farmer.account_id(), farm1_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("2"));
    let farm_info = show_farminfo(&farming, farm2_id.clone(), false);
    assert_farming(&farm_info, "Running".to_string(), to_yocto("500"), 1, 0, to_yocto("0"), to_yocto("1"), to_yocto("0"));
    let unclaim = show_unclaim(&farming, farmer.account_id(), farm2_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("1"));
    let out_come = call!(
        farmer,
        farming.claim_reward_by_seed(format!("{}@0", swap())),
        deposit = 0
    );
    out_come.assert_success();
    let farm_info = show_farminfo(&farming, farm0_id.clone(), false);
    assert_farming(&farm_info, "Running".to_string(), to_yocto("500"), 5, 5, to_yocto("5"), 0, to_yocto("0"));
    let unclaim = show_unclaim(&farming, farmer.account_id(), farm0_id.clone(), false);
    assert_eq!(unclaim.0, 0_u128);
    let farm_info = show_farminfo(&farming, farm1_id.clone(), false);
    assert_farming(&farm_info, "Running".to_string(), to_yocto("500"), 3, 3, to_yocto("3"), to_yocto("0"), to_yocto("0"));
    let unclaim = show_unclaim(&farming, farmer.account_id(), farm1_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("0"));
    let farm_info = show_farminfo(&farming, farm2_id.clone(), false);
    assert_farming(&farm_info, "Running".to_string(), to_yocto("500"), 1, 1, to_yocto("1"), to_yocto("0"), to_yocto("0"));
    let unclaim = show_unclaim(&farming, farmer.account_id(), farm2_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("0"));
    println!("  Farmer claimed reward at #{}.", root.borrow_runtime().current_block().block_height);

}