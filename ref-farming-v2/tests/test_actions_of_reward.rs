use near_sdk_sim::{call, init_simulator, to_yocto};
use near_sdk::json_types::{U128};
use ref_farming_v2::{HRSimpleFarmTerms};

use crate::common::utils::*;
use crate::common::init::deploy_farming;
use crate::common::views::*;
use crate::common::actions::*;

mod common;

#[test]
fn test_remove_user_rps_by_farm(){
    generate_user_account!(root, owner, farmer1);

    let (pool, token1, _) = prepair_pool_and_liquidity(
        &root, &owner, farming_id(), vec![&farmer1]);

    let farm_id = "swap@0#0".to_string();
    let farming = deploy_farming(&root, farming_id(), owner.account_id());
    call!(farmer1, farming.storage_deposit(None, None), deposit = to_yocto("1")).assert_success();

    call!(
        owner,
        farming.create_simple_farm(HRSimpleFarmTerms{
            seed_id: format!("{}@0", pool.account_id()),
            reward_token: to_va(token1.account_id()),
            start_at: 0,
            reward_per_session: to_yocto("1").into(),
            session_interval: 60,
        }, None),
        deposit = to_yocto("1")
    ).assert_success();

    // deposit reward
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
    ).assert_success();

    // farmer1 staking lpt 
    println!("----->> Farmer1 staking lpt.");
    call!(
        farmer1,
        pool.mft_transfer_call(":0".to_string(), to_va(farming_id()), to_yocto("1").into(), None, "".to_string()),
        deposit = 1
    ).assert_success();
    assert_eq!(get_user_rps(&farming, farmer1.account_id(),farm_id.clone()).unwrap(), "0");

    // should panic when remove_user_rps_by_farm
    assert_err!(call!(
        farmer1,
        farming.remove_user_rps_by_farm("swap".to_string()),
        deposit = 0
    ), "E42: invalid farm id");

    assert_eq!(call!(
        farmer1,
        farming.remove_user_rps_by_farm(farm_id.clone())
    ).unwrap_json::<bool>(), false);

    //The rewards have been handed out, but farm not expire
    root.borrow_runtime_mut().cur_block.block_timestamp += to_nano(60 * 11);

    assert_eq!(show_unclaim(&farming, farmer1.account_id(), farm_id.clone(), false).0, to_yocto("10"));
    
    root.borrow_runtime_mut().cur_block.block_timestamp += to_nano(3600 * 24 * 30);

    call!(
        owner,
        farming.force_clean_farm(farm_id.clone())
    ).assert_success();

    let farm_info = show_outdated_farminfo(&farming, farm_id.clone(), false);
    assert_farming(&farm_info, "Cleared".to_string(), to_yocto("10"), 0, 0, to_yocto("10"), 0, to_yocto("10"));

    call!(
        farmer1,
        farming.remove_user_rps_by_farm(farm_id.clone())
    ).assert_success();

    assert!(get_user_rps(&farming, farmer1.account_id(),farm_id.clone()).is_none());
}

#[test]
fn test_claim_reward_by_farm(){
    generate_user_account!(root, owner, farmer1);

    let (pool, token1, _) = prepair_pool_and_liquidity(
        &root, &owner, farming_id(), vec![&farmer1]);

    let farm_id = "swap@0#0".to_string();
    let farming = deploy_farming(&root, farming_id(), owner.account_id());
    

    call!(
        owner,
        farming.create_simple_farm(HRSimpleFarmTerms{
            seed_id: format!("{}@0", pool.account_id()),
            reward_token: to_va(token1.account_id()),
            start_at: 0,
            reward_per_session: to_yocto("1").into(),
            session_interval: 60,
        }, None),
        deposit = to_yocto("1")
    ).assert_success();

    // deposit reward
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
    ).assert_success();

    assert_err!(call!(
        farmer1,
        farming.claim_reward_by_farm(farm_id.clone())
    ), "E10: account not registered");

    call!(farmer1, farming.storage_deposit(None, None), deposit = to_yocto("1")).assert_success();

    // farmer1 staking lpt 
    println!("----->> Farmer1 staking lpt.");
    call!(
        farmer1,
        pool.mft_transfer_call(":0".to_string(), to_va(farming_id()), to_yocto("1").into(), None, "".to_string()),
        deposit = 1
    ).assert_success();
    
    root.borrow_runtime_mut().cur_block.block_timestamp += to_nano(60);

    assert_err!(call!(
        farmer1,
        farming.claim_reward_by_farm("random".to_string())
    ), "E42: invalid farm id");

    call!(
        farmer1,
        farming.claim_reward_by_farm(farm_id.clone()),
        deposit = 0
    ).assert_success();

    let farm_info = show_farminfo(&farming, farm_id.clone(), false);
    assert_farming(&farm_info, "Running".to_string(), to_yocto("10"), 1, 1, to_yocto("1"), 0, 0);
}

#[test]
fn test_claim_reward_by_seed(){
    generate_user_account!(root, owner, farmer1);

    let (pool, token1, _) = prepair_pool_and_liquidity(
        &root, &owner, farming_id(), vec![&farmer1]);

    let farm_id1 = format!("{}@0#0", pool.account_id());
    let farm_id2 = format!("{}@0#1", pool.account_id());
    let seed_id = format!("{}@0", pool.account_id());
    let farming = deploy_farming(&root, farming_id(), owner.account_id());
    
    call!(
        owner,
        farming.create_simple_farm(HRSimpleFarmTerms{
            seed_id: seed_id.clone(),
            reward_token: to_va(token1.account_id()),
            start_at: 0,
            reward_per_session: to_yocto("1").into(),
            session_interval: 60,
        }, None),
        deposit = to_yocto("1")
    ).assert_success();

    call!(
        owner,
        farming.create_simple_farm(HRSimpleFarmTerms{
            seed_id: seed_id.clone(),
            reward_token: to_va(token1.account_id()),
            start_at: 0,
            reward_per_session: to_yocto("1").into(),
            session_interval: 60,
        }, None),
        deposit = to_yocto("1")
    ).assert_success();

    // deposit reward
    call!(
        root,
        token1.storage_deposit(Some(to_va(farming_id())), None),
        deposit = to_yocto("1")
    )
    .assert_success();
    mint_token(&token1, &root, to_yocto("20"));
    call!(
        root,
        token1.ft_transfer_call(to_va(farming_id()), U128(to_yocto("10")), None, generate_reward_msg(farm_id1.clone())),
        deposit = 1
    ).assert_success();
    call!(
        root,
        token1.ft_transfer_call(to_va(farming_id()), U128(to_yocto("10")), None, generate_reward_msg(farm_id2.clone())),
        deposit = 1
    ).assert_success();

    assert_err!(call!(
        farmer1,
        farming.claim_reward_by_seed(seed_id.clone())
    ), "E10: account not registered");

    call!(farmer1, farming.storage_deposit(None, None), deposit = to_yocto("1")).assert_success();

    // farmer1 staking lpt 
    println!("----->> Farmer1 staking lpt.");
    call!(
        farmer1,
        pool.mft_transfer_call(":0".to_string(), to_va(farming_id()), to_yocto("1").into(), None, "".to_string()),
        deposit = 1
    ).assert_success();
    
    root.borrow_runtime_mut().cur_block.block_timestamp += to_nano(60);

    call!(
        farmer1,
        farming.claim_reward_by_seed(seed_id.clone()),
        deposit = 0
    ).assert_success();

    let farm_info = show_farminfo(&farming, farm_id1.clone(), false);
    assert_farming(&farm_info, "Running".to_string(), to_yocto("10"), 1, 1, to_yocto("1"), 0, 0);

    let farm_info = show_farminfo(&farming, farm_id2.clone(), false);
    assert_farming(&farm_info, "Running".to_string(), to_yocto("10"), 1, 1, to_yocto("1"), 0, 0);
}

#[test]
fn test_withdraw_reward(){
    generate_user_account!(root, owner, farmer1);

    let (pool, token1, _) = prepair_pool_and_liquidity(
        &root, &owner, farming_id(), vec![&farmer1]);

    let farm_id = "swap@0#0".to_string();
    let farming = deploy_farming(&root, farming_id(), owner.account_id());

    call!(
        owner,
        farming.create_simple_farm(HRSimpleFarmTerms{
            seed_id: format!("{}@0", pool.account_id()),
            reward_token: to_va(token1.account_id()),
            start_at: 0,
            reward_per_session: to_yocto("1").into(),
            session_interval: 60,
        }, None),
        deposit = to_yocto("1")
    ).assert_success();

    // deposit reward
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
    ).assert_success();

    assert_err!(call!(
        farmer1,
        farming.withdraw_reward(token1.valid_account_id(), None),
        deposit = 1
    ), "E10: account not registered");

    call!(farmer1, farming.storage_deposit(None, None), deposit = to_yocto("1")).assert_success();

    // farmer1 staking lpt 
    println!("----->> Farmer1 staking lpt.");
    call!(
        farmer1,
        pool.mft_transfer_call(":0".to_string(), to_va(farming_id()), to_yocto("1").into(), None, "".to_string()),
        deposit = 1
    ).assert_success();
    
    root.borrow_runtime_mut().cur_block.block_timestamp += to_nano(60);

    assert_err!(call!(
        farmer1,
        farming.withdraw_reward(token1.valid_account_id(), None),
        deposit = 1
    ), "E21: token not registered");

    call!(
        farmer1,
        farming.claim_reward_by_farm(farm_id.clone()),
        deposit = 0
    ).assert_success();

    assert_err!(call!(
        farmer1,
        farming.withdraw_reward(token1.valid_account_id(), Some(U128(to_yocto("1.1")))),
        deposit = 1
    ), "E22: not enough tokens in deposit");
}