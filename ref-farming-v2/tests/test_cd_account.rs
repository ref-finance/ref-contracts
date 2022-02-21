use near_sdk_sim::{call, init_simulator, to_yocto, view};
use near_sdk::json_types::{U128};
use near_sdk::serde_json::Value;

use ref_farming_v2::{HRSimpleFarmTerms, UserSeedInfo};

use crate::common::utils::*;
use crate::common::init::deploy_farming;
use crate::common::views::*;
use crate::common::actions::*;

mod common;


#[test]
fn single_farm_cd_account() {
    let root = init_simulator(None);

    println!("----->> Prepare accounts.");
    let owner = root.create_user("owner".to_string(), to_yocto("100"));
    let farmer1 = root.create_user("farmer1".to_string(), to_yocto("100"));
    let farmer2 = root.create_user("farmer2".to_string(), to_yocto("100"));
    println!("<<----- owner and 2 farmers prepared.");

    println!("----->> Prepare ref-exchange and swap pool.");
    let (pool, token1, _) = prepair_pool_and_liquidity(
        &root, &owner, farming_id(), vec![&farmer1, &farmer2]);
    println!("<<----- The pool prepaired.");

    // deploy farming contract and register user
    println!("----->> Deploy farming and register farmers.");
    let farming = deploy_farming(&root, farming_id(), owner.account_id());
    call!(farmer1, farming.storage_deposit(None, None), deposit = to_yocto("1")).assert_success();
    call!(farmer2, farming.storage_deposit(None, None), deposit = to_yocto("1")).assert_success();
    println!("<<----- farming deployed, farmers registered.");

    // create farm
    println!("----->> Create farm.");
    let farm_id = "swap@0#0".to_string();
    let seed_id = format!("{}@0", pool.account_id());
    let out_come = call!(
        owner,
        farming.create_simple_farm(HRSimpleFarmTerms{
            seed_id: seed_id.clone(),
            reward_token: token1.valid_account_id(),
            start_at: 0,
            reward_per_session: to_yocto("1").into(),
            session_interval: 60,
        }, None),
        deposit = to_yocto("1")
    );
    out_come.assert_success();
    assert_eq!(Value::String(farm_id.clone()), out_come.unwrap_json_value());
    println!("<<----- Farm {} created at #{}, ts:{}.", 
    farm_id,
    root.borrow_runtime().current_block().block_height, 
    root.borrow_runtime().current_block().block_timestamp);

    // deposit reward
    println!("----->> Deposit reward to turn farm Running.");
    call!(
        root,
        token1.storage_deposit(Some(to_va(farming_id())), None),
        deposit = to_yocto("1")
    )
    .assert_success();
    mint_token(&token1, &root, to_yocto("10"));
    call!(
        root,
        token1.ft_transfer_call(to_va(farming_id()), U128(to_yocto("10")), None, generate_reward_msg(farm_id.clone())),
        deposit = 1
    )
    .assert_success();
    show_farminfo(&farming, farm_id.clone(), true);

    call!(
        owner,
        farming.modify_cd_strategy_item(0, 1000, 10_000)
    ).assert_success();

    println!("<<----- Farm {} deposit reward at #{}, ts:{}.", 
    farm_id,
    root.borrow_runtime().current_block().block_height, 
    root.borrow_runtime().current_block().block_timestamp);

    // farmer1 staking lpt 
    println!("----->> Farmer1 staking lpt.");
    let out_come = call!(
        farmer1,
        pool.mft_transfer_call(":0".to_string(), to_va(farming_id()), to_yocto("1").into(), None, "".to_string()),
        deposit = 1
    );
    out_come.assert_success();
    
    println!("<<----- Farmer1 staked liquidity at #{}, ts:{}.", 
    root.borrow_runtime().current_block().block_height, 
    root.borrow_runtime().current_block().block_timestamp);
    let farm_info = show_farminfo(&farming, farm_id.clone(), false);
    assert_farming(&farm_info, "Running".to_string(), to_yocto("10"), 0, 0, 0, 0, 0);
    let user_seed_amounts = show_user_seed_amounts(&farming, farmer1.account_id(), false);
    assert_eq!(user_seed_amounts.get(&seed_id.clone()).unwrap().0, to_yocto("1"));
    let user_seed_powers = show_user_seed_powers(&farming, farmer1.account_id(), false);
    assert_eq!(user_seed_powers.get(&String::from("swap@0")).unwrap().0, to_yocto("1"));
    
    let unclaim = show_unclaim(&farming, farmer1.account_id(), farm_id.clone(), false);
    assert_eq!(unclaim.0, 0_u128);

    println!("----->> move to 60 secs later.");
    assert!(root.borrow_runtime_mut().produce_blocks(60).is_ok());
    println!("<<----- Chain goes 60 blocks, now #{}, ts:{}.", 
    root.borrow_runtime().current_block().block_height, 
    root.borrow_runtime().current_block().block_timestamp);
    let farm_info = show_farminfo(&farming, farm_id.clone(), false);
    assert_farming(&farm_info, "Running".to_string(), to_yocto("10"), 1, 0, 0, to_yocto("1"), 0);
    let user_seed_amounts = show_user_seed_amounts(&farming, farmer1.account_id(), false);
    assert_eq!(user_seed_amounts.get(&String::from("swap@0")).unwrap().0, to_yocto("1"));
    let user_seed_powers = show_user_seed_powers(&farming, farmer1.account_id(), false);
    assert_eq!(user_seed_powers.get(&String::from("swap@0")).unwrap().0, to_yocto("1"));
    let unclaim = show_unclaim(&farming, farmer1.account_id(), farm_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("1"));

    // farmer2 staking lpt 
    println!("----->> Farmer2 add cd account.");
    let out_come = call!(
        farmer2,
        pool.mft_transfer_call(":0".to_string(), to_va(farming_id()), to_yocto("0.5").into(), None, generate_cd_account_msg(0, seed_id.clone(), 0)),
        deposit = 1
    );
    out_come.assert_success();
    println!("<<----- Farmer2 add cd account at #{}, ts:{}.", 
    root.borrow_runtime().current_block().block_height, 
    root.borrow_runtime().current_block().block_timestamp);
    let farm_info = show_farminfo(&farming, farm_id.clone(), false);
    assert_farming(&farm_info, "Running".to_string(), to_yocto("10"), 1, 1, 0, to_yocto("1"), 0);
    let user_seed_amounts = show_user_seed_amounts(&farming, farmer2.account_id(), false);
    assert_eq!(user_seed_amounts.get(&String::from("swap@0")), None);
    let user_seed_powers = show_user_seed_powers(&farming, farmer2.account_id(), false);
    assert_eq!(user_seed_powers.get(&String::from("swap@0")).unwrap().0, to_yocto("1"));
    let unclaim = show_unclaim(&farming, farmer2.account_id(), farm_id.clone(), false);
    assert_eq!(unclaim.0, 0_u128);

    println!("----->> move to 60 secs later.");
    assert!(root.borrow_runtime_mut().produce_blocks(60).is_ok());
    println!("<<----- Chain goes 60 blocks, now #{}, ts:{}.", 
    root.borrow_runtime().current_block().block_height, 
    root.borrow_runtime().current_block().block_timestamp);
    let farm_info = show_farminfo(&farming, farm_id.clone(), false);
    assert_farming(&farm_info, "Running".to_string(), to_yocto("10"), 2, 1, 0, to_yocto("2"), 0);
    let unclaim = show_unclaim(&farming, farmer1.account_id(), farm_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("1.5"));
    let unclaim = show_unclaim(&farming, farmer2.account_id(), farm_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("0.5"));

    println!("----->> move to 60 secs later and farmer1 claim reward by farm_id.");
    assert!(root.borrow_runtime_mut().produce_blocks(60).is_ok());
    println!("        Chain goes 60 blocks, now #{}, ts:{}.", 
    root.borrow_runtime().current_block().block_height, 
    root.borrow_runtime().current_block().block_timestamp);
    let farm_info = show_farminfo(&farming, farm_id.clone(), false);
    assert_farming(&farm_info, "Running".to_string(), to_yocto("10"), 3, 1, 0, to_yocto("3"), 0);
    let unclaim = show_unclaim(&farming, farmer1.account_id(), farm_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("2"));
    let unclaim = show_unclaim(&farming, farmer2.account_id(), farm_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("1"));
    let out_come = call!(
        farmer1,
        farming.claim_reward_by_farm(farm_id.clone()),
        deposit = 0
    );
    out_come.assert_success();
    // // println!("{:#?}", out_come.promise_results());
    let farm_info = show_farminfo(&farming, farm_id.clone(), false);
    assert_farming(&farm_info, "Running".to_string(), to_yocto("10"), 3, 3, to_yocto("2"), to_yocto("1"), 0);
    let unclaim = show_unclaim(&farming, farmer1.account_id(), farm_id.clone(), false);
    assert_eq!(unclaim.0, 0_u128);
    let unclaim = show_unclaim(&farming, farmer2.account_id(), farm_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("1"));
    let reward = show_reward(&farming, farmer1.account_id(), token1.account_id(), false);
    assert_eq!(reward.0, to_yocto("2"));
    println!("<<----- Farmer1 claimed reward by farmid, now #{}, ts:{}.", 
    root.borrow_runtime().current_block().block_height, 
    root.borrow_runtime().current_block().block_timestamp);

    println!("----->> move to 60 secs later and farmer2 claim reward by seed_id.");
    assert!(root.borrow_runtime_mut().produce_blocks(60).is_ok());
    println!("        Chain goes 60 blocks, now #{}, ts:{}.", 
    root.borrow_runtime().current_block().block_height, 
    root.borrow_runtime().current_block().block_timestamp);
    let farm_info = show_farminfo(&farming, farm_id.clone(), false);
    assert_farming(&farm_info, "Running".to_string(), to_yocto("10"), 4, 3, to_yocto("2"), to_yocto("2"), 0);
    let unclaim = show_unclaim(&farming, farmer1.account_id(), farm_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("0.5"));
    let unclaim = show_unclaim(&farming, farmer2.account_id(), farm_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("1.5"));
    let out_come = call!(
        farmer2,
        farming.claim_reward_by_seed(farm_info.seed_id.clone()),
        deposit = 0
    );
    out_come.assert_success();
    // println!("{:#?}", out_come.promise_results());
    let farm_info = show_farminfo(&farming, farm_id.clone(), false);
    assert_farming(&farm_info, "Running".to_string(), to_yocto("10"), 4, 4, to_yocto("3.5"), to_yocto("0.5"), 0);
    let unclaim = show_unclaim(&farming, farmer1.account_id(), farm_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("0.5"));
    let unclaim = show_unclaim(&farming, farmer2.account_id(), farm_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("0"));
    let reward = show_reward(&farming, farmer2.account_id(), token1.account_id(), false);
    assert_eq!(reward.0, to_yocto("1.5"));
    println!("<<----- Farmer2 claimed reward by seedid, now #{}, ts:{}.", 
    root.borrow_runtime().current_block().block_height, 
    root.borrow_runtime().current_block().block_timestamp);

    println!("----->> move to 60 secs later and farmer1 unstake half lpt.");
    assert!(root.borrow_runtime_mut().produce_blocks(60).is_ok());
    println!("        Chain goes 60 blocks, now #{}, ts:{}.", 
    root.borrow_runtime().current_block().block_height, 
    root.borrow_runtime().current_block().block_timestamp);
    let farm_info = show_farminfo(&farming, farm_id.clone(), false);
    assert_farming(&farm_info, "Running".to_string(), to_yocto("10"), 5, 4, to_yocto("3.5"), to_yocto("1.5"), 0);
    let unclaim = show_unclaim(&farming, farmer1.account_id(), farm_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("1"));
    let unclaim = show_unclaim(&farming, farmer2.account_id(), farm_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("0.5"));

    let out_come = call!(
        farmer1,
        farming.withdraw_seed(farm_info.seed_id.clone(), to_yocto("0.4").into()),
        deposit = 1
    );
    out_come.assert_success();
    

    let farm_info = show_farminfo(&farming, farm_id.clone(), false);
    assert_farming(&farm_info, "Running".to_string(), to_yocto("10"), 5, 5, to_yocto("4.5"), to_yocto("0.5"), 0);
    let unclaim = show_unclaim(&farming, farmer1.account_id(), farm_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("0"));
    let unclaim = show_unclaim(&farming, farmer2.account_id(), farm_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("0.5"));
    let reward = show_reward(&farming, farmer1.account_id(), token1.account_id(), false);
    assert_eq!(reward.0, to_yocto("3"));
    let user_seed_amounts = show_user_seed_amounts(&farming, farmer1.account_id(), false);
    assert_eq!(user_seed_amounts.get(&seed_id.clone()).unwrap().0, to_yocto("0.6"));
    let user_seed_powers = show_user_seed_powers(&farming, farmer1.account_id(), false);
    assert_eq!(user_seed_powers.get(&seed_id.clone()).unwrap().0, to_yocto("0.6"));
    println!("<<----- Farmer1 unstake half lpt, now #{}, ts:{}.", 
    root.borrow_runtime().current_block().block_height, 
    root.borrow_runtime().current_block().block_timestamp);

    println!("----->> move to 60 secs later and farmer2 remove cd account.");
    assert!(root.borrow_runtime_mut().produce_blocks(60).is_ok());
    println!("        Chain goes 60 blocks, now #{}, ts:{}.", 
    root.borrow_runtime().current_block().block_height, 
    root.borrow_runtime().current_block().block_timestamp);
    let farm_info = show_farminfo(&farming, farm_id.clone(), false);
    assert_farming(&farm_info, "Running".to_string(), to_yocto("10"), 6, 5, to_yocto("4.5"), to_yocto("1.5"), 0);
    let unclaim = show_unclaim(&farming, farmer1.account_id(), farm_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("0.375"));
    let unclaim = show_unclaim(&farming, farmer2.account_id(), farm_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("1.125"));
    let out_come = call!(
        farmer2,
        farming.withdraw_seed(farm_info.seed_id.clone(), to_yocto("1").into()),
        deposit = 1
    );
    let ex_status = format!("{:?}", out_come.promise_errors()[0].as_ref().unwrap().status());
    assert!(ex_status.contains("E31: seed not exist"));

    let out_come = call!(
        farmer2,
        farming.remove_cd_account(0),
        deposit = 1
    );
    out_come.assert_success();
    // println!("{:#?}", out_come.promise_results());
    let farm_info = show_farminfo(&farming, farm_id.clone(), false);
    assert_farming(&farm_info, "Running".to_string(), to_yocto("10"), 6, 6, to_yocto("5.625"), to_yocto("0.375"), 0);
    let unclaim = show_unclaim(&farming, farmer1.account_id(), farm_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("0.375"));
    let unclaim = show_unclaim(&farming, farmer2.account_id(), farm_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("0"));
    let reward = show_reward(&farming, farmer2.account_id(), token1.account_id(), false);
    assert_eq!(reward.0, to_yocto("2.625"));
    let user_seed_amounts = show_user_seed_amounts(&farming, farmer2.account_id(), false);
    assert!(user_seed_amounts.get(&seed_id.clone()).is_none());
    let user_seed_powers = show_user_seed_powers(&farming, farmer2.account_id(), false);
    assert!(user_seed_powers.get(&seed_id.clone()).is_none());
    println!("<<----- Farmer2 remove cd account, now #{}, ts:{}.", 
    root.borrow_runtime().current_block().block_height, 
    root.borrow_runtime().current_block().block_timestamp);

    println!("----->> move to 60 secs later and farmer1 unstake the other half lpt.");
    assert!(root.borrow_runtime_mut().produce_blocks(60).is_ok());
    println!("        Chain goes 60 blocks, now #{}, ts:{}.", 
    root.borrow_runtime().current_block().block_height, 
    root.borrow_runtime().current_block().block_timestamp);
    let farm_info = show_farminfo(&farming, farm_id.clone(), false);
    assert_farming(&farm_info, "Running".to_string(), to_yocto("10"), 7, 6, to_yocto("5.625"), to_yocto("1.375"), 0);
    let unclaim = show_unclaim(&farming, farmer1.account_id(), farm_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("1.374999999999999999999999"));
    let unclaim = show_unclaim(&farming, farmer2.account_id(), farm_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("0"));
    let out_come = call!(
        farmer1,
        farming.withdraw_seed(farm_info.seed_id.clone(), to_yocto("0.6").into()),
        deposit = 1
    );
    out_come.assert_success();
    // println!("{:#?}", out_come.promise_results());
    let farm_info = show_farminfo(&farming, farm_id.clone(), false);
    assert_farming(&farm_info, "Running".to_string(), to_yocto("10"), 7, 7, to_yocto("6.999999999999999999999999"), 1, 0);
    let unclaim = show_unclaim(&farming, farmer1.account_id(), farm_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("0"));
    let unclaim = show_unclaim(&farming, farmer2.account_id(), farm_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("0"));
    let reward = show_reward(&farming, farmer1.account_id(), token1.account_id(), false);
    assert_eq!(reward.0, to_yocto("4.374999999999999999999999"));
    let user_seed_amounts = show_user_seed_amounts(&farming, farmer1.account_id(), false);
    assert!(user_seed_amounts.get(&seed_id.clone()).is_none());
    let user_seed_powers = show_user_seed_powers(&farming, farmer1.account_id(), false);
    assert!(user_seed_powers.get(&seed_id.clone()).is_none());
    println!("<<----- Farmer1 unstake the other half lpt, now #{}, ts:{}.", 
    root.borrow_runtime().current_block().block_height, 
    root.borrow_runtime().current_block().block_timestamp);

    println!("----->> move to 60 secs later and farmer1 restake lpt.");
    assert!(root.borrow_runtime_mut().produce_blocks(60).is_ok());
    println!("        Chain goes 60 blocks, now #{}, ts:{}.", 
    root.borrow_runtime().current_block().block_height, 
    root.borrow_runtime().current_block().block_timestamp);
    let farm_info = show_farminfo(&farming, farm_id.clone(), false);
    assert_farming(&farm_info, "Running".to_string(), to_yocto("10"), 8, 7, to_yocto("6.999999999999999999999999"), 1 + to_yocto("1"), 0);
    let unclaim = show_unclaim(&farming, farmer1.account_id(), farm_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("0"));
    let unclaim = show_unclaim(&farming, farmer2.account_id(), farm_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("0"));
    let out_come = call!(
        farmer1,
        pool.mft_transfer_call(":0".to_string(), to_va(farming_id()), to_yocto("1").into(), None, "".to_string()),
        deposit = 1
    );
    out_come.assert_success();
    println!("<<----- Farmer1 staked liquidity at #{}, ts:{}.", 
    root.borrow_runtime().current_block().block_height, 
    root.borrow_runtime().current_block().block_timestamp);
    let farm_info = show_farminfo(&farming, farm_id.clone(), false);
    assert_farming(&farm_info, "Running".to_string(), to_yocto("10"), 8, 8, to_yocto("8"), to_yocto("0"), to_yocto("1") + 1);
    let user_seed_amounts = show_user_seed_amounts(&farming, farmer1.account_id(), false);
    assert_eq!(user_seed_amounts.get(&seed_id.clone()).unwrap().0, to_yocto("1"));
    let user_seed_powers = show_user_seed_powers(&farming, farmer1.account_id(), false);
    assert_eq!(user_seed_powers.get(&seed_id.clone()).unwrap().0, to_yocto("1"));
    let unclaim = show_unclaim(&farming, farmer1.account_id(), farm_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("0"));
    let unclaim = show_unclaim(&farming, farmer2.account_id(), farm_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("0"));
    let out_come = call!(
        owner,
        farming.force_clean_farm(farm_id.clone()),
        deposit = 0
    );
    out_come.assert_success();
    assert_eq!(Value::Bool(false), out_come.unwrap_json_value());

    println!("----->> move to 40 secs later and farmer2 add cd account.");
    assert!(root.borrow_runtime_mut().produce_blocks(40).is_ok());
    println!("        Chain goes 40 blocks, now #{}, ts:{}.", 
    root.borrow_runtime().current_block().block_height, 
    root.borrow_runtime().current_block().block_timestamp);
    let farm_info = show_farminfo(&farming, farm_id.clone(), false);
    assert_farming(&farm_info, "Running".to_string(), to_yocto("10"), 9, 8, to_yocto("8"), to_yocto("1"), to_yocto("1") + 1);
    let unclaim = show_unclaim(&farming, farmer1.account_id(), farm_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("1"));
    let unclaim = show_unclaim(&farming, farmer2.account_id(), farm_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("0"));
    let out_come = call!(
        farmer2,
        pool.mft_transfer_call(":0".to_string(), to_va(farming_id()), to_yocto("0.5").into(), None, generate_cd_account_msg(0, seed_id.clone(), 0)),
        deposit = 1
    );
    out_come.assert_success();
    println!("<<----- Farmer2 add cd account at #{}, ts:{}.", 
    root.borrow_runtime().current_block().block_height, 
    root.borrow_runtime().current_block().block_timestamp);
    let farm_info = show_farminfo(&farming, farm_id.clone(), false);
    assert_farming(&farm_info, "Running".to_string(), to_yocto("10"), 9, 9, to_yocto("8"), to_yocto("1"), to_yocto("1") + 1);
    let user_seed_amounts = show_user_seed_amounts(&farming, farmer2.account_id(), false);
    assert!(user_seed_amounts.get(&seed_id.clone()).is_none());
    let user_seed_powers = show_user_seed_powers(&farming, farmer2.account_id(), false);
    assert_eq!(user_seed_powers.get(&seed_id.clone()).unwrap().0, to_yocto("1"));
    let unclaim = show_unclaim(&farming, farmer1.account_id(), farm_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("1"));
    let unclaim = show_unclaim(&farming, farmer2.account_id(), farm_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("0"));

    println!("----->> move to 60 secs later.");
    assert!(root.borrow_runtime_mut().produce_blocks(60).is_ok());
    println!("<<----- Chain goes 60 blocks, now #{}, ts:{}.", 
    root.borrow_runtime().current_block().block_height, 
    root.borrow_runtime().current_block().block_timestamp);
    let farm_info = show_farminfo(&farming, farm_id.clone(), false);
    assert_farming(&farm_info, "Ended".to_string(), to_yocto("10"), 10, 9, to_yocto("8"), to_yocto("2"), to_yocto("1") + 1);
    let unclaim = show_unclaim(&farming, farmer1.account_id(), farm_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("1.5"));
    let unclaim = show_unclaim(&farming, farmer2.account_id(), farm_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("0.5"));

    println!("----->> move to 60 secs later, and force remove farm");
    assert!(root.borrow_runtime_mut().produce_blocks(60).is_ok());
    println!("<<----- Chain goes 60 blocks, now #{}, ts:{}.", 
    root.borrow_runtime().current_block().block_height, 
    root.borrow_runtime().current_block().block_timestamp);
    let farm_info = show_farminfo(&farming, farm_id.clone(), false);
    assert_farming(&farm_info, "Ended".to_string(), to_yocto("10"), 10, 9, to_yocto("8"), to_yocto("2"), to_yocto("1") + 1);
    let unclaim = show_unclaim(&farming, farmer1.account_id(), farm_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("1.5"));
    let unclaim = show_unclaim(&farming, farmer2.account_id(), farm_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("0.5"));
    let out_come = call!(
        owner,
        farming.force_clean_farm(farm_id.clone()),
        deposit = 0
    );
    out_come.assert_success();
    assert_eq!(Value::Bool(true), out_come.unwrap_json_value());
    assert_eq!(view!(farming.get_number_of_farms()).unwrap_json::<u64>(), 0);
    assert_eq!(view!(farming.get_number_of_outdated_farms()).unwrap_json::<u64>(), 1);
    let farm_info = show_outdated_farminfo(&farming, farm_id.clone(), true);
    assert_farming(&farm_info, "Cleared".to_string(), to_yocto("10"), 10, 10, to_yocto("10"), to_yocto("0"), to_yocto("3") + 1);
}

#[test]
fn multi_farm_with_different_state_cd_account() {
    let root = init_simulator(None);

    // prepair users
    let owner = root.create_user("owner".to_string(), to_yocto("100"));
    let farmer = root.create_user("farmer".to_string(), to_yocto("100"));
    println!("----->> owner and farmer prepaired.");

    // prepair pool and tokens
    let(pool, token1, token2) = prepair_pool(&root, &owner);
    println!("----->> The pool prepaired.");

    // farmer add liqidity 
    add_liquidity(&farmer, &pool, &token1, &token2, 0);
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
        token1.ft_transfer_call(to_va(farming_id()), to_yocto("500").into(), None, generate_reward_msg(farm0_id.clone())),
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
        token1.ft_transfer_call(to_va(farming_id()), to_yocto("500").into(), None, generate_reward_msg(farm2_id.clone())),
        deposit = 1
    )
    .assert_success();
    println!("    Farm {} deposit reward at Height#{}", farm2_id.clone(), root.borrow_runtime().current_block().block_height);

    println!("---->> Registering LP 0 for {}.", farming_id());
    call!(root, pool.mft_register(":0".to_string(), to_va(farming_id())), deposit = to_yocto("1"))
    .assert_success();

    call!(
        owner,
        farming.modify_cd_strategy_item(0, 1000, 10_000)
    ).assert_success();

    println!("---->> Step01: Farmer register and stake liquidity token.");
    call!(farmer, farming.storage_deposit(None, None), deposit = to_yocto("1"))
    .assert_success();
    let out_come = call!(
        farmer,
        pool.mft_transfer_call(":0".to_string(), to_va(farming_id()), to_yocto("0.5").into(), None, generate_cd_account_msg(0, format!("{}@0", swap()), 0)),
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
        token1.ft_transfer_call(to_va(farming_id()), to_yocto("500").into(), None, generate_reward_msg(farm1_id.clone())),
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

#[test]
fn cd_account_invalid_id_and_limit(){
    let root = init_simulator(None);

    println!("----->> Prepare accounts.");
    let owner = root.create_user("owner".to_string(), to_yocto("100"));
    let farmer = root.create_user("farmer1".to_string(), to_yocto("100"));
    println!("<<----- owner and farmer prepared.");

    println!("----->> Prepare ref-exchange and swap pool.");
    let (pool, token1, _) = prepair_pool_and_liquidity(
        &root, &owner, farming_id(), vec![&farmer]);
    println!("<<----- The pool prepaired.");

    // deploy farming contract and register user
    println!("----->> Deploy farming and register farmers.");
    let farming = deploy_farming(&root, farming_id(), owner.account_id());
    call!(farmer, farming.storage_deposit(None, None), deposit = to_yocto("1")).assert_success();
    println!("<<----- farming deployed, farmers registered.");

    // create farm
    println!("----->> Create farm.");
    let farm_id = "swap@0#0".to_string();
    let seed_id = format!("{}@0", pool.account_id());
    let out_come = call!(
        owner,
        farming.create_simple_farm(HRSimpleFarmTerms{
            seed_id: seed_id.clone(),
            reward_token: token1.valid_account_id(),
            start_at: 0,
            reward_per_session: to_yocto("1").into(),
            session_interval: 60,
        }, None),
        deposit = to_yocto("1")
    );
    out_come.assert_success();
    assert_eq!(Value::String(farm_id.clone()), out_come.unwrap_json_value());
    println!("<<----- Farm {} created at #{}, ts:{}.", 
    farm_id,
    root.borrow_runtime().current_block().block_height, 
    root.borrow_runtime().current_block().block_timestamp);

    call!(
        owner,
        farming.modify_cd_strategy_item(0, 1000, 10_000)
    ).assert_success();

    let out_come = call!(
        farmer,
        pool.mft_transfer_call(":0".to_string(), to_va(farming_id()), to_yocto("0.01").into(), None, generate_cd_account_msg(1, seed_id.clone(), 0)),
        deposit = 1
    );
    let ex_status = format!("{:?}", out_come.promise_errors()[0].as_ref().unwrap().status());
    assert!(ex_status.contains("E62: invalid CDStrategy index"));

    for index in 0..16{
        let out_come = call!(
            farmer,
            pool.mft_transfer_call(":0".to_string(), to_va(farming_id()), to_yocto("0.01").into(), None, generate_cd_account_msg(index, seed_id.clone(), 0)),
            deposit = 1
        );
        out_come.assert_success();
        let user_seed_info = view!(farming.get_user_seed_info(farmer.valid_account_id(), seed_id.clone())).unwrap_json::<UserSeedInfo>();
        assert!(user_seed_info.cds.len() == (index + 1) as usize);
    }
    
    let out_come = call!(
        farmer,
        pool.mft_transfer_call(":0".to_string(), to_va(farming_id()), to_yocto("0.01").into(), None, generate_cd_account_msg(16, seed_id.clone(), 0)),
        deposit = 1
    );
    let ex_status = format!("{:?}", out_come.promise_errors()[0].as_ref().unwrap().status());
    assert!(ex_status.contains("E61: the number of CDAccounts has reached its limit"));
}

#[test]
fn cd_account_remove(){
    let root = init_simulator(None);

    println!("----->> Prepare accounts.");
    let owner = root.create_user("owner".to_string(), to_yocto("100"));
    let farmer = root.create_user("farmer1".to_string(), to_yocto("100"));
    println!("<<----- owner and farmer prepared.");

    println!("----->> Prepare ref-exchange and swap pool.");
    let (pool, token1, _) = prepair_pool_and_liquidity(
        &root, &owner, farming_id(), vec![&farmer]);
    println!("<<----- The pool prepaired.");

    // deploy farming contract and register user
    println!("----->> Deploy farming and register farmers.");
    let farming = deploy_farming(&root, farming_id(), owner.account_id());
    call!(farmer, farming.storage_deposit(None, None), deposit = to_yocto("1")).assert_success();
    println!("<<----- farming deployed, farmers registered.");

    // create farm
    println!("----->> Create farm.");
    let farm_id = "swap@0#0".to_string();
    let seed_id = format!("{}@0", pool.account_id());
    let out_come = call!(
        owner,
        farming.create_simple_farm(HRSimpleFarmTerms{
            seed_id: seed_id.clone(),
            reward_token: token1.valid_account_id(),
            start_at: 0,
            reward_per_session: to_yocto("1").into(),
            session_interval: 60,
        }, None),
        deposit = to_yocto("1")
    );
    out_come.assert_success();
    assert_eq!(Value::String(farm_id.clone()), out_come.unwrap_json_value());
    println!("<<----- Farm {} created at #{}, ts:{}.", 
    farm_id,
    root.borrow_runtime().current_block().block_height, 
    root.borrow_runtime().current_block().block_timestamp);

    //set cd strategy
    call!(
        owner,
        farming.modify_cd_strategy_item(0, 1000, 10_000)
    ).assert_success();

    call!(
        owner,
        farming.modify_cd_strategy_damage(10_000)
    ).assert_success();


    //remove after end sec
    let out_come = call!(
        farmer,
        pool.mft_transfer_call(":0".to_string(), to_va(farming_id()), to_yocto("0.01").into(), None, generate_cd_account_msg(0, seed_id.clone(), 0)),
        deposit = 1
    );
    out_come.assert_success();
    let current_timestamp = root.borrow_runtime().current_block().block_timestamp;
    assert_eq!(view!(pool.mft_balance_of(":0".to_string(), farmer.valid_account_id()))
    .unwrap_json::<U128>()
    .0, to_yocto("0.99"));

    let user_seed_info = view!(farming.get_user_seed_info(farmer.valid_account_id(), seed_id.clone())).unwrap_json::<UserSeedInfo>();
    assert_eq!(user_seed_info.cds[0].seed_amount.0, to_yocto("0.01"));
    assert_eq!(user_seed_info.cds[0].seed_power.0, to_yocto("0.02"));
    root.borrow_runtime_mut().cur_block.block_timestamp = current_timestamp + to_nano(1000);
    let out_come = call!(
        farmer,
        farming.remove_cd_account(0),
        deposit = 1
    );
    out_come.assert_success();
    assert_eq!(view!(pool.mft_balance_of(":0".to_string(), farmer.valid_account_id()))
    .unwrap_json::<U128>()
    .0, to_yocto("1"));
    
    
    //remove before end sec
    let out_come = call!(
        farmer,
        pool.mft_transfer_call(":0".to_string(), to_va(farming_id()), to_yocto("1").into(), None, generate_cd_account_msg(0, seed_id.clone(), 0)),
        deposit = 1
    );
    out_come.assert_success();
    let current_timestamp = root.borrow_runtime().current_block().block_timestamp;
    assert_eq!(view!(pool.mft_balance_of(":0".to_string(), farmer.valid_account_id()))
    .unwrap_json::<U128>()
    .0, to_yocto("0"));

    let user_seed_info = view!(farming.get_user_seed_info(farmer.valid_account_id(), seed_id.clone())).unwrap_json::<UserSeedInfo>();
    assert_eq!(user_seed_info.cds[0].seed_amount.0, to_yocto("1"));
    assert_eq!(user_seed_info.cds[0].seed_power.0, to_yocto("2"));
    root.borrow_runtime_mut().cur_block.block_timestamp = current_timestamp + to_nano(500);
    let out_come = call!(
        farmer,
        farming.remove_cd_account(0),
        deposit = 1
    );
    out_come.assert_success();
    assert!(view!(pool.mft_balance_of(":0".to_string(), farmer.valid_account_id()))
    .unwrap_json::<U128>()
    .0 > to_yocto("0.49"));
    assert!(view!(pool.mft_balance_of(":0".to_string(), farmer.valid_account_id()))
    .unwrap_json::<U128>()
    .0 < to_yocto("0.51"));
}

#[test]
fn cd_account_append(){
    let root = init_simulator(None);

    println!("----->> Prepare accounts.");
    let owner = root.create_user("owner".to_string(), to_yocto("100"));
    let farmer = root.create_user("farmer1".to_string(), to_yocto("100"));
    println!("<<----- owner and farmer prepared.");

    println!("----->> Prepare ref-exchange and swap pool.");
    let (pool, token1, _) = prepair_pool_and_liquidity(
        &root, &owner, farming_id(), vec![&farmer]);
    println!("<<----- The pool prepaired.");

    // deploy farming contract and register user
    println!("----->> Deploy farming and register farmers.");
    let farming = deploy_farming(&root, farming_id(), owner.account_id());
    call!(farmer, farming.storage_deposit(None, None), deposit = to_yocto("1")).assert_success();
    println!("<<----- farming deployed, farmers registered.");

    // create farm
    println!("----->> Create farm.");
    let farm_id = "swap@0#0".to_string();
    let seed_id = format!("{}@0", pool.account_id());
    let out_come = call!(
        owner,
        farming.create_simple_farm(HRSimpleFarmTerms{
            seed_id: seed_id.clone(),
            reward_token: token1.valid_account_id(),
            start_at: 0,
            reward_per_session: to_yocto("1").into(),
            session_interval: 60,
        }, None),
        deposit = to_yocto("1")
    );
    out_come.assert_success();
    assert_eq!(Value::String(farm_id.clone()), out_come.unwrap_json_value());
    println!("<<----- Farm {} created at #{}, ts:{}.", 
    farm_id,
    root.borrow_runtime().current_block().block_height, 
    root.borrow_runtime().current_block().block_timestamp);

    //set cd strategy
    call!(
        owner,
        farming.modify_cd_strategy_item(0, 1000, 10_000)
    ).assert_success();

    call!(
        owner,
        farming.modify_cd_strategy_damage(10_000)
    ).assert_success();

    //
    let out_come = call!(
        farmer,
        pool.mft_transfer_call(":0".to_string(), to_va(farming_id()), to_yocto("0.01").into(), None, generate_cd_account_msg(0, seed_id.clone(), 0)),
        deposit = 1
    );
    out_come.assert_success();
    let current_timestamp = root.borrow_runtime().current_block().block_timestamp;
    assert_eq!(view!(pool.mft_balance_of(":0".to_string(), farmer.valid_account_id()))
    .unwrap_json::<U128>()
    .0, to_yocto("0.99"));

    let user_seed_info = view!(farming.get_user_seed_info(farmer.valid_account_id(), seed_id.clone())).unwrap_json::<UserSeedInfo>();
    assert_eq!(user_seed_info.cds[0].seed_amount.0, to_yocto("0.01"));
    assert_eq!(user_seed_info.cds[0].seed_power.0, to_yocto("0.02"));
    let out_come = call!(
        farmer,
        pool.mft_transfer_call(":0".to_string(), to_va(farming_id()), to_yocto("0.01").into(), None, append_cd_account_msg(0, seed_id.clone())),
        deposit = 1
    );
    out_come.assert_success();
    let user_seed_info = view!(farming.get_user_seed_info(farmer.valid_account_id(), seed_id.clone())).unwrap_json::<UserSeedInfo>();
    assert_eq!(user_seed_info.cds[0].seed_amount.0, to_yocto("0.02"));
    assert!(user_seed_info.cds[0].seed_power.0 < to_yocto("0.04"));
    assert!(user_seed_info.cds[0].seed_power.0 > to_yocto("0.039"));

    root.borrow_runtime_mut().cur_block.block_timestamp = current_timestamp + to_nano(500);

    let out_come = call!(
        farmer,
        pool.mft_transfer_call(":0".to_string(), to_va(farming_id()), to_yocto("0.01").into(), None, append_cd_account_msg(0, seed_id.clone())),
        deposit = 1
    );
    out_come.assert_success();
    let user_seed_info = view!(farming.get_user_seed_info(farmer.valid_account_id(), seed_id.clone())).unwrap_json::<UserSeedInfo>();

    assert_eq!(user_seed_info.cds[0].seed_amount.0, to_yocto("0.03"));
    println!("{}", user_seed_info.cds[0].seed_power.0);
    assert!(user_seed_info.cds[0].seed_power.0 < to_yocto("0.055"));
    assert!(user_seed_info.cds[0].seed_power.0 > to_yocto("0.054"));

    root.borrow_runtime_mut().cur_block.block_timestamp = current_timestamp + to_nano(1000);
    let out_come = call!(
        farmer,
        pool.mft_transfer_call(":0".to_string(), to_va(farming_id()), to_yocto("0.01").into(), None, append_cd_account_msg(0, seed_id.clone())),
        deposit = 1
    );
    let ex_status = format!("{:?}", out_come.promise_errors()[0].as_ref().unwrap().status());
    assert!(ex_status.contains("E64: expired CDAccount"));
    
}