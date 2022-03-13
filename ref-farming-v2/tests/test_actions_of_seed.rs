use near_sdk_sim::{call, init_simulator, to_yocto, view};
use near_sdk::json_types::{U128};
use near_sdk::serde_json::Value;

use ref_farming_v2::{HRSimpleFarmTerms};

use crate::common::utils::*;
use crate::common::init::{deploy_farming, deploy_token};
use crate::common::views::*;
use crate::common::actions::*;

mod common;

#[test]
/// reward: 10.pow(33), seed: 10.pow(0)
/// rps: 10.pow(57)
fn seed_amount_little() {
    generate_user_account!(root, owner, farmer1);

    let (pool, token1, _) = prepair_pool_and_liquidity(
        &root, &owner, farming_id(), vec![&farmer1]);

    let farming = deploy_farming(&root, farming_id(), owner.account_id());

    assert_err!(call!(
        farmer1,
        pool.mft_transfer_call(":0".to_string(), to_va(farming_id()), to_yocto("0.5").into(), None, "".to_string()),
        deposit = 1
    ), "E31: seed not exist");

    let farm_id = "swap@0#0".to_string();
    call!(
        owner,
        farming.create_simple_farm(HRSimpleFarmTerms{
            seed_id: format!("{}@0", pool.account_id()),
            reward_token: token1.valid_account_id(),
            start_at: 0,
            reward_per_session: to_yocto("1000000000").into(),
            session_interval: 60,
        }, None),
        deposit = to_yocto("1")
    ).assert_success();

    assert_err!(call!(
        farmer1,
        farming.withdraw_seed("swap@0".to_string(), to_yocto("0.6").into()),
        deposit = 1
    ), "E10: account not registered");

    call!(farmer1, farming.storage_deposit(None, None), deposit = to_yocto("1")).assert_success();

    call!(
        root,
        token1.storage_deposit(Some(to_va(farming_id())), None),
        deposit = to_yocto("1")
    ).assert_success();
    mint_token(&token1, &root, to_yocto("10000000000"));
    call!(
        root,
        token1.ft_transfer_call(to_va(farming_id()), U128(to_yocto("10000000000")), None, generate_reward_msg(farm_id.clone())),
        deposit = 1
    ).assert_success();
    let farm_info = show_farminfo(&farming, farm_id.clone(), false);

    call!(
        farmer1,
        pool.mft_transfer_call(":0".to_string(), to_va(farming_id()), to_yocto("1").into(), None, "".to_string()),
        deposit = 1
    ).assert_success();

    assert_err!(call!(
        farmer1,
        farming.withdraw_seed(format!("{}@1", pool.account_id()), to_yocto("0.5").into()),
        deposit = 1
    ), "E31: seed not exist");

    assert_err!(call!(
        farmer1,
        farming.withdraw_seed("swap@0".to_string(), to_yocto("10").into()),
        deposit = 1
    ), "E32: not enough amount of seed");

    call!(
        farmer1,
        farming.withdraw_seed(farm_info.seed_id.clone(), to_yocto("0.999999999999999999999999").into()),
        deposit = 1
    ).assert_success();

    assert!(root.borrow_runtime_mut().produce_blocks(60).is_ok());
    let farm_info = show_farminfo(&farming, farm_id.clone(), false);
    assert_farming(&farm_info, "Running".to_string(), to_yocto("10000000000"), 1, 0, 0, to_yocto("1000000000"), 0);
    let user_seeds = show_user_seed_amounts(&farming, farmer1.account_id(), false);
    assert_eq!(user_seeds.get(&String::from("swap@0")).unwrap().0, 1);
    let unclaim = show_unclaim(&farming, farmer1.account_id(), farm_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("1000000000"));

    assert!(root.borrow_runtime_mut().produce_blocks(60).is_ok());
    let farm_info = show_farminfo(&farming, farm_id.clone(), false);
    assert_farming(&farm_info, "Running".to_string(), to_yocto("10000000000"), 2, 0, to_yocto("0"), to_yocto("2000000000"), to_yocto("0"));
    let unclaim = show_unclaim(&farming, farmer1.account_id(), farm_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("2000000000"));
    call!(
        farmer1,
        farming.claim_reward_by_farm(farm_id.clone()),
        deposit = 0
    ).assert_success();

    let farm_info = show_farminfo(&farming, farm_id.clone(), false);
    assert_farming(&farm_info, "Running".to_string(), to_yocto("10000000000"), 2, 2, to_yocto("2000000000"), to_yocto("0"), to_yocto("0"));
    let unclaim = show_unclaim(&farming, farmer1.account_id(), farm_id.clone(), false);
    assert_eq!(unclaim.0, 0_u128);
    let reward = show_reward(&farming, farmer1.account_id(), token1.account_id(), false);
    assert_eq!(reward.0, to_yocto("2000000000"));
    let user_rps = view!(farming.get_user_rps(farmer1.valid_account_id(), farm_id)).unwrap_json::<Option<String>>().unwrap();
    assert_eq!(user_rps, String::from("2000000000000000000000000000000000000000000000000000000000"));
}

#[test]
/// reward 10.pow(17), seed: 10.pow(38) 
/// rps 10.pow(3) 
fn seed_amount_huge() {
    // println!("{}", u128::MAX);
    // 340282366920938.463463374607431768211455

    generate_user_account!(root, owner, farmer1);

    let (pool, token1, token2) = prepair_pool(&root, &owner);
    call!(root, pool.mft_register(":0".to_string(), to_va(farming_id())), deposit = to_yocto("1"))
    .assert_success();

    mint_token(&token1, &farmer1, u128::MAX);
    mint_token(&token2, &farmer1, u128::MAX);
    call!(
        farmer1,
        pool.storage_deposit(None, None),
        deposit = to_yocto("1")
    ).assert_success();
    call!(
        farmer1,
        token1.ft_transfer_call(to_va(swap()), U128(to_yocto("1")), None, "".to_string()),
        deposit = 1
    ).assert_success();
    call!(
        farmer1,
        token2.ft_transfer_call(to_va(swap()), U128(to_yocto("1")), None, "".to_string()),
        deposit = 1
    ).assert_success();
    call!(
        farmer1,
        pool.add_liquidity(0, vec![U128(to_yocto("1")), U128(to_yocto("1"))], None),
        deposit = to_yocto("0.01")
    ).assert_success();

    call!(
        farmer1,
        token1.ft_transfer_call(to_va(swap()), U128(to_yocto("340282366920937")), None, "".to_string()),
        deposit = 1
    ).assert_success();
    call!(
        farmer1,
        token2.ft_transfer_call(to_va(swap()), U128(to_yocto("340282366920937")), None, "".to_string()),
        deposit = 1
    ).assert_success();
    call!(
        farmer1,
        pool.add_liquidity(0, vec![U128(to_yocto("340282366920937")), U128(to_yocto("340282366920937"))], None),
        deposit = to_yocto("0.01")
    ).assert_success();


    let farming = deploy_farming(&root, farming_id(), owner.account_id());
    call!(farmer1, farming.storage_deposit(None, None), deposit = to_yocto("1")).assert_success();

    let token3 = deploy_token(&root, String::from("rft"), vec![farming_id()]);
    mint_token(&token3, &farmer1, to_yocto("10"));

    let farm_id = "swap@0#0".to_string();
    let single_reward = 100000000000000000_u128;
    call!(
        owner,
        farming.create_simple_farm(HRSimpleFarmTerms{
            seed_id: format!("{}@0", pool.account_id()),
            reward_token: token3.valid_account_id(),
            start_at: 0,
            reward_per_session: U128(single_reward),
            session_interval: 60,
        }, None),
        deposit = to_yocto("1")
    ).assert_success();

    call!(
        farmer1,
        token3.ft_transfer_call(to_va(farming_id()), U128(to_yocto("10")), None, generate_reward_msg(farm_id.clone())),
        deposit = 1
    ).assert_success();
    let _farm_info = show_farminfo(&farming, farm_id.clone(), false);

    call!(
        farmer1,
        pool.mft_transfer_call(":0".to_string(), to_va(farming_id()), U128(to_yocto("340282366920938")), None, "".to_string()),
        deposit = 1
    ).assert_success();
    

    assert!(root.borrow_runtime_mut().produce_blocks(60).is_ok());
    let farm_info = show_farminfo(&farming, farm_id.clone(), false);
    assert_farming(&farm_info, "Running".to_string(), to_yocto("10"), 1, 0, 0, single_reward, 0);
    let user_seeds = show_user_seed_amounts(&farming, farmer1.account_id(), false);
    assert_eq!(user_seeds.get(&String::from("swap@0")).unwrap().0, to_yocto("340282366920938"));
    let unclaim = show_unclaim(&farming, farmer1.account_id(), farm_id.clone(), false);
    assert!(unclaim.0 > 99000000000000000);

    assert!(root.borrow_runtime_mut().produce_blocks(60).is_ok());
    let farm_info = show_farminfo(&farming, farm_id.clone(), false);
    assert_farming(&farm_info, "Running".to_string(), to_yocto("10"), 2, 0, to_yocto("0"), 2 * single_reward, to_yocto("0"));
    let unclaim = show_unclaim(&farming, farmer1.account_id(), farm_id.clone(), false);
    assert!(unclaim.0 > 199000000000000000);
    call!(
        farmer1,
        farming.claim_reward_by_farm(farm_id.clone()),
        deposit = 0
    ).assert_success();
    let unclaim = show_unclaim(&farming, farmer1.account_id(), farm_id.clone(), false);
    assert_eq!(unclaim.0, 0_u128);
    let reward = show_reward(&farming, farmer1.account_id(), token3.account_id(), false);
    assert!(reward.0 > 199000000000000000);
    let user_rps = view!(farming.get_user_rps(farmer1.valid_account_id(), farm_id)).unwrap_json::<Option<String>>().unwrap();
    assert_eq!(user_rps, String::from("587"));
}

#[test]
fn cd_account_invalid_id_and_limit(){
    println!("----->> Prepare accounts.");
    generate_user_account!(root, owner, farmer);
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
        farming.modify_cd_strategy_item(0, 1000, 10_000),
        deposit = 1
    ).assert_success();

    assert_err!(call!(
        farmer,
        pool.mft_transfer_call(":0".to_string(), to_va(farming_id()), to_yocto("0.01").into(), None, generate_cd_account_msg(1, 0)),
        deposit = 1
    ), "E63: invalid CDAccount index");

    for index in 0..16{
        let out_come = call!(
            farmer,
            pool.mft_transfer_call(":0".to_string(), to_va(farming_id()), to_yocto("0.01").into(), None, generate_cd_account_msg(index, 0)),
            deposit = 1
        );
        out_come.assert_success();
        let user_seed_info = get_user_seed_info(&farming, farmer.account_id(), seed_id.clone());
        assert!(user_seed_info.cds.len() == (index + 1) as usize);
    }
    
    assert_err!(call!(
        farmer,
        pool.mft_transfer_call(":0".to_string(), to_va(farming_id()), to_yocto("0.01").into(), None, generate_cd_account_msg(16, 0)),
        deposit = 1
    ), "E63: invalid CDAccount index");
}

#[test]
fn cd_account_remove_and_withdraw_seed_slashed(){
    println!("----->> Prepare accounts.");
    generate_user_account!(root, owner, farmer);
    println!("<<----- owner and farmer prepared.");

    println!("----->> Prepare ref-exchange and swap pool.");
    let (pool, token1, _) = prepair_pool_and_liquidity(
        &root, &owner, farming_id(), vec![&farmer, &owner]);
    println!("<<----- The pool prepaired.");

    // deploy farming contract and register user
    println!("----->> Deploy farming and register farmers.");
    let farming = deploy_farming(&root, farming_id(), owner.account_id());
    call!(farmer, farming.storage_deposit(None, None), deposit = to_yocto("1")).assert_success();
    println!("<<----- farming deployed, farmers registered.");

    //set cd strategy
    call!(
        owner,
        farming.modify_cd_strategy_item(0, 1000, 10_000),
        deposit = 1
    ).assert_success();

    call!(
        owner,
        farming.modify_default_seed_slash_rate(10_000),
        deposit = 1
    ).assert_success();

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

    //remove after end sec
    call!(
        farmer,
        pool.mft_transfer_call(":0".to_string(), to_va(farming_id()), to_yocto("0.01").into(), None, generate_cd_account_msg(0, 0)),
        deposit = 1
    ).assert_success();
    let current_timestamp = root.borrow_runtime().current_block().block_timestamp;
    assert_eq!(view!(pool.mft_balance_of(":0".to_string(), farmer.valid_account_id()))
        .unwrap_json::<U128>().0, to_yocto("0.99"));

    assert_err!(call!(farmer, farming.storage_unregister(None), deposit = 1),
        "E13: still has seed power when unregister");

    let user_seed_info = get_user_seed_info(&farming, farmer.account_id(), seed_id.clone());
    assert_eq!(user_seed_info.cds[0].seed_amount.0, to_yocto("0.01"));
    assert_eq!(user_seed_info.cds[0].seed_power.0, to_yocto("0.02"));
    root.borrow_runtime_mut().cur_block.block_timestamp = current_timestamp + to_nano(1000);
    call!(
        farmer,
        farming.withdraw_seed_from_cd_account(0, to_yocto("0.01").into()),
        deposit = 1
    ).assert_success();
    assert_eq!(view!(pool.mft_balance_of(":0".to_string(), farmer.valid_account_id()))
        .unwrap_json::<U128>().0, to_yocto("1"));
    
    //remove before end sec
    call!(
        farmer,
        pool.mft_transfer_call(":0".to_string(), to_va(farming_id()), to_yocto("1").into(), None, generate_cd_account_msg(0, 0)),
        deposit = 1
    ).assert_success();
    let current_timestamp = root.borrow_runtime().current_block().block_timestamp;
    assert_eq!(view!(pool.mft_balance_of(":0".to_string(), farmer.valid_account_id()))
        .unwrap_json::<U128>().0, to_yocto("0"));
    
    assert_err!(call!(
        owner,
        farming.withdraw_seed_slashed(seed_id.clone()),
        deposit = 1
    ), "E32: not enough amount of seed");

    let user_seed_info = get_user_seed_info(&farming, farmer.account_id(), seed_id.clone());
    assert_eq!(user_seed_info.cds[0].seed_amount.0, to_yocto("1"));
    assert_eq!(user_seed_info.cds[0].seed_power.0, to_yocto("2"));
    root.borrow_runtime_mut().cur_block.block_timestamp = current_timestamp + to_nano(500);
    call!(
        farmer,
        farming.withdraw_seed_from_cd_account(0, to_yocto("1").into()),
        deposit = 1
    ).assert_success();
    let seeds_slashed_info = show_shashed(&farming, false);
    assert!(seeds_slashed_info.get(&seed_id).unwrap().0 > to_yocto("0.49"));
    assert!(seeds_slashed_info.get(&seed_id).unwrap().0 < to_yocto("0.51"));
    assert!(view!(pool.mft_balance_of(":0".to_string(), farmer.valid_account_id()))
        .unwrap_json::<U128>().0 > to_yocto("0.49"));
    assert!(view!(pool.mft_balance_of(":0".to_string(), farmer.valid_account_id()))
        .unwrap_json::<U128>().0 < to_yocto("0.51"));
    assert_eq!(view!(pool.mft_balance_of(":0".to_string(), farmer.valid_account_id()))
        .unwrap_json::<U128>().0 + seeds_slashed_info.get(&seed_id).unwrap().0, to_yocto("1"));

    let before_withdraw = view!(pool.mft_balance_of(":0".to_string(), owner.valid_account_id())).unwrap_json::<U128>().0;
    call!(
        owner,
        farming.withdraw_seed_slashed(seed_id.clone()),
        deposit = 1
    ).assert_success();
    let after_withdraw = view!(pool.mft_balance_of(":0".to_string(), owner.valid_account_id())).unwrap_json::<U128>().0;
    assert!(after_withdraw - before_withdraw > to_yocto("0.49"));
    assert!(after_withdraw - before_withdraw < to_yocto("0.51"));

    assert_err!(call!(
        farmer,
        pool.mft_transfer_call(":0".to_string(), to_va(farming_id()), to_yocto("0.01").into(), None, append_cd_account_msg(0)),
        deposit = 1
    ), "E66: Empty CDAccount");

    assert_err!(call!(
        farmer,
        farming.withdraw_seed_from_cd_account(0, to_yocto("0.01").into()),
        deposit = 1
    ), "E66: Empty CDAccount");
}

#[test]
fn cd_account_append(){
    println!("----->> Prepare accounts.");
    generate_user_account!(root, owner, farmer);
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
        farming.modify_cd_strategy_item(0, 1000, 10_000),
        deposit = 1
    ).assert_success();

    call!(
        owner,
        farming.modify_default_seed_slash_rate(10_000),
        deposit = 1
    ).assert_success();

    call!(
        farmer,
        pool.mft_transfer_call(":0".to_string(), to_va(farming_id()), to_yocto("0.01").into(), None, generate_cd_account_msg(0, 0)),
        deposit = 1
    ).assert_success();
    let current_timestamp = root.borrow_runtime().current_block().block_timestamp;
    assert_eq!(view!(pool.mft_balance_of(":0".to_string(), farmer.valid_account_id()))
    .unwrap_json::<U128>()
    .0, to_yocto("0.99"));

    let user_seed_info = get_user_seed_info(&farming, farmer.account_id(), seed_id.clone());
    assert_eq!(user_seed_info.cds[0].seed_amount.0, to_yocto("0.01"));
    assert_eq!(user_seed_info.cds[0].seed_power.0, to_yocto("0.02"));
    let out_come = call!(
        farmer,
        pool.mft_transfer_call(":0".to_string(), to_va(farming_id()), to_yocto("0.01").into(), None, append_cd_account_msg(0)),
        deposit = 1
    );
    out_come.assert_success();
    let user_seed_info = get_user_seed_info(&farming, farmer.account_id(), seed_id.clone());
    assert_eq!(user_seed_info.cds[0].seed_amount.0, to_yocto("0.02"));
    assert!(user_seed_info.cds[0].seed_power.0 < to_yocto("0.04"));
    assert!(user_seed_info.cds[0].seed_power.0 > to_yocto("0.039"));

    root.borrow_runtime_mut().cur_block.block_timestamp = current_timestamp + to_nano(500);

    let out_come = call!(
        farmer,
        pool.mft_transfer_call(":0".to_string(), to_va(farming_id()), to_yocto("0.01").into(), None, append_cd_account_msg(0)),
        deposit = 1
    );
    out_come.assert_success();
    let user_seed_info = get_user_seed_info(&farming, farmer.account_id(), seed_id.clone());

    assert_eq!(user_seed_info.cds[0].seed_amount.0, to_yocto("0.03"));
    assert!(user_seed_info.cds[0].seed_power.0 < to_yocto("0.055"));
    assert!(user_seed_info.cds[0].seed_power.0 > to_yocto("0.054"));

    root.borrow_runtime_mut().cur_block.block_timestamp = current_timestamp + to_nano(1000);
    call!(
        farmer,
        pool.mft_transfer_call(":0".to_string(), to_va(farming_id()), to_yocto("0.01").into(), None, append_cd_account_msg(0)),
        deposit = 1
    ).assert_success();
    let user_seed_info = get_user_seed_info(&farming, farmer.account_id(), seed_id.clone());

    assert_eq!(user_seed_info.cds[0].seed_amount.0, to_yocto("0.04"));
    assert!(user_seed_info.cds[0].seed_power.0 < to_yocto("0.065"));
    assert!(user_seed_info.cds[0].seed_power.0 > to_yocto("0.064"));
}


#[test]
fn test_return_seed_lostfound(){
    generate_user_account!(root, owner, farmer1);

    let (_, token1, token2) = prepair_pool(&root, &owner);

    call!(
        root, token2.mint(farmer1.valid_account_id(), to_yocto("10000").into())
    )
    .assert_success();

    let farming = deploy_farming(&root, farming_id(), owner.account_id());
    call!(farmer1, farming.storage_deposit(None, None), deposit = to_yocto("1")).assert_success();
    call!(farming.user_account, token2.storage_deposit(None, None), deposit = to_yocto("1")).assert_success();

    call!(
        owner,
        farming.create_simple_farm(HRSimpleFarmTerms{
            seed_id: format!("{}", token2.account_id()),
            reward_token: token1.valid_account_id(),
            start_at: 0,
            reward_per_session: to_yocto("1").into(),
            session_interval: 60,
        }, Some(U128(100))),
        deposit = to_yocto("1")
    ).assert_success();

    call!(
        farmer1,
        token2.ft_transfer_call(to_va(farming_id()), U128(500), None, "".to_string()),
        deposit = 1
    ).assert_success();


    call!(farmer1, token2.storage_unregister(Some(true)), deposit = 1).assert_success();

    assert_err!(call!(
        farmer1,
        farming.withdraw_seed(token2.account_id(), U128(100)),
        deposit = 1
    ), "The account farmer1 is not registered");

    let user_seeds = show_user_seed_amounts(&farming, farmer1.account_id(), false);
    assert_eq!(user_seeds.get(&format!("{}", token2.account_id())).unwrap().0, 400);

    let lostfound_info = show_lostfound(&farming, false);
    assert_eq!(lostfound_info.get(&format!("{}", token2.account_id())).unwrap().0, 100);

    call!(farmer1, token2.storage_deposit(None, None), deposit = to_yocto("1")).assert_success();

    let before_withdraw = view!(token2.ft_balance_of(farmer1.valid_account_id())).unwrap_json::<U128>().0;
    assert_err!(call!(
        owner,
        farming.return_seed_lostfound(farmer1.valid_account_id(), format!("{}", token2.account_id()), U128(101)),
        deposit = 1
    ), "E32: not enough amount of seed");

    call!(
        owner,
        farming.return_seed_lostfound(farmer1.valid_account_id(), format!("{}", token2.account_id()), U128(100)),
        deposit = 1
    ).assert_success();

    let after_withdraw = view!(token2.ft_balance_of(farmer1.valid_account_id())).unwrap_json::<U128>().0;
    assert_eq!(after_withdraw - before_withdraw, 100);
}