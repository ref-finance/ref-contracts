use near_sdk_sim::{call, init_simulator, to_yocto};
use near_sdk::json_types::{U128};
use ref_farming::{HRSimpleFarmTerms};

use crate::common::utils::*;
use crate::common::init::deploy_farming;
use crate::common::views::*;
use crate::common::actions::*;

mod common;

#[test]
fn failure_e10_stake_before_register() {
    let root = init_simulator(None);

    let owner = root.create_user("owner".to_string(), to_yocto("100"));
    let farmer1 = root.create_user("farmer1".to_string(), to_yocto("100"));

    let (pool, token1, _) = prepair_pool_and_liquidity(
        &root, &owner, farming_id(), vec![&farmer1]);

    let farming = deploy_farming(&root, farming_id(), owner.account_id());

    let out_come = call!(
        owner,
        farming.create_simple_farm(HRSimpleFarmTerms{
            seed_id: format!("{}@0", pool.account_id()),
            reward_token: token1.valid_account_id(),
            start_at: 0,
            reward_per_session: to_yocto("1").into(),
            session_interval: 60,
        }, None),
        deposit = to_yocto("1")
    );
    out_come.assert_success();


    let out_come = call!(
        farmer1,
        pool.mft_transfer_call(":0".to_string(), to_va(farming_id()), U128(to_yocto("0.5")), None, "".to_string()),
        deposit = 1
    );
    out_come.assert_success();
    let ex_status = format!("{:?}", out_come.promise_errors()[0].as_ref().unwrap().status());
    assert!(ex_status.contains("E10: account not registered"));
    let user_seeds = show_userseeds(&farming, farmer1.account_id(), false);
    assert!(user_seeds.get(&String::from("swap@0")).is_none());
}

#[test]
fn failure_e10_unstake_before_register() {
    let root = init_simulator(None);

    let owner = root.create_user("owner".to_string(), to_yocto("100"));
    let farmer1 = root.create_user("farmer1".to_string(), to_yocto("100"));

    let (pool, token1, _) = prepair_pool_and_liquidity(
        &root, &owner, farming_id(), vec![&farmer1]);

    let farming = deploy_farming(&root, farming_id(), owner.account_id());

    let out_come = call!(
        owner,
        farming.create_simple_farm(HRSimpleFarmTerms{
            seed_id: format!("{}@0", pool.account_id()),
            reward_token: token1.valid_account_id(),
            start_at: 0,
            reward_per_session: to_yocto("1").into(),
            session_interval: 60,
        }, None),
        deposit = to_yocto("1")
    );
    out_come.assert_success();

    let out_come = call!(
        farmer1,
        farming.withdraw_seed("swap@0".to_string(), to_yocto("0.6").into()),
        deposit = 1
    );
    assert!(!out_come.is_ok());
    let ex_status = format!("{:?}", out_come.promise_errors()[0].as_ref().unwrap().status());
    // println!("ex_status: {}", ex_status);
    assert!(ex_status.contains("E10: account not registered"));
}

#[test]
fn failure_e10_claim_before_register() {
    let root = init_simulator(None);

    let owner = root.create_user("owner".to_string(), to_yocto("100"));
    let farmer1 = root.create_user("farmer1".to_string(), to_yocto("100"));

    let (pool, token1, _) = prepair_pool_and_liquidity(
        &root, &owner, farming_id(), vec![&farmer1]);

    let farming = deploy_farming(&root, farming_id(), owner.account_id());
    
    let out_come = call!(
        owner,
        farming.create_simple_farm(HRSimpleFarmTerms{
            seed_id: format!("{}@0", pool.account_id()),
            reward_token: token1.valid_account_id(),
            start_at: 0,
            reward_per_session: to_yocto("1").into(),
            session_interval: 60,
        }, None),
        deposit = to_yocto("1")
    );
    out_come.assert_success();

    let out_come = call!(
        farmer1,
        farming.claim_reward_by_seed("swap@0".to_string()),
        deposit = 0
    );
    assert!(!out_come.is_ok());
    let ex_status = format!("{:?}", out_come.promise_errors()[0].as_ref().unwrap().status());
    // println!("ex_status: {}", ex_status);
    assert!(ex_status.contains("E10: account not registered"));

    let out_come = call!(
        farmer1,
        farming.claim_reward_by_farm("swap@0#0".to_string()),
        deposit = 0
    );
    assert!(!out_come.is_ok());
    let ex_status = format!("{:?}", out_come.promise_errors()[0].as_ref().unwrap().status());
    // println!("ex_status: {}", ex_status);
    assert!(ex_status.contains("E10: account not registered"));
}

#[test]
fn failure_e10_storage_withdraw_before_register() {
    let root = init_simulator(None);

    let owner = root.create_user("owner".to_string(), to_yocto("100"));
    let farmer1 = root.create_user("farmer1".to_string(), to_yocto("100"));

    // let (pool, token1, token2) = prepair_pool_and_liquidity(
    //     &root, &owner, farming_id(), vec![&farmer1]);

    let farming = deploy_farming(&root, farming_id(), owner.account_id());


    let out_come = call!(
        farmer1,
        farming.storage_withdraw(None),
        deposit = 1
    );
    assert!(!out_come.is_ok());
    let ex_status = format!("{:?}", out_come.promise_errors()[0].as_ref().unwrap().status());
    assert!(ex_status.contains("E10: account not registered"));
}

#[test]
fn failure_e11_create_farm() {
    let root = init_simulator(None);

    let owner = root.create_user("owner".to_string(), to_yocto("100"));
    let farmer1 = root.create_user("farmer1".to_string(), to_yocto("100"));

    let (pool, token1, _) = prepair_pool_and_liquidity(
        &root, &owner, farming_id(), vec![&farmer1]);

    let farming = deploy_farming(&root, farming_id(), owner.account_id());

    let out_come = call!(
        owner,
        farming.create_simple_farm(HRSimpleFarmTerms{
            seed_id: format!("{}@0", pool.account_id()),
            reward_token: token1.valid_account_id(),
            start_at: 0,
            reward_per_session: to_yocto("1").into(),
            session_interval: 60,
        }, None),
        deposit = to_yocto("0.00001")
    );
    assert!(!out_come.is_ok());
    // println!("{:#?}", out_come.promise_results());
    let ex_status = format!("{:?}", out_come.promise_errors()[0].as_ref().unwrap().status());
    assert!(ex_status.contains("E11: insufficient $NEAR storage deposit"));
}

#[test]
fn failure_e11_register_new() {
    let root = init_simulator(None);

    let owner = root.create_user("owner".to_string(), to_yocto("100"));
    let farmer1 = root.create_user("farmer1".to_string(), to_yocto("100"));

    // let (pool, token1, token2) = prepair_pool_and_liquidity(
    //     &root, &owner, farming_id(), vec![&farmer1]);

    let farming = deploy_farming(&root, farming_id(), owner.account_id());

    let out_come = call!(farmer1, farming.storage_deposit(None, Some(true)), deposit = to_yocto("0.0001"));
    assert!(!out_come.is_ok());
    let ex_status = format!("{:?}", out_come.promise_errors()[0].as_ref().unwrap().status());
    assert!(ex_status.contains("E11: insufficient $NEAR storage deposit"));
}

#[test]
fn failure_e11_stake() {
    let root = init_simulator(None);

    let owner = root.create_user("owner".to_string(), to_yocto("100"));
    let farmer1 = root.create_user("farmer1".to_string(), to_yocto("100"));

    let (pool, token1, _) = prepair_pool_and_liquidity(
        &root, &owner, farming_id(), vec![&farmer1]);

    let farming = deploy_farming(&root, farming_id(), owner.account_id());
    call!(farmer1, farming.storage_deposit(None, None), deposit = to_yocto("1")).assert_success();
    call!(farmer1, farming.storage_withdraw(None), deposit = 1).assert_success();
    // call!(farmer1, farming.storage_deposit(None, None), deposit = to_yocto("1")).assert_success();

    let out_come = call!(
        owner,
        farming.create_simple_farm(HRSimpleFarmTerms{
            seed_id: format!("{}@0", pool.account_id()),
            reward_token: token1.valid_account_id(),
            start_at: 0,
            reward_per_session: to_yocto("1").into(),
            session_interval: 60,
        }, None),
        deposit = to_yocto("1")
    );
    out_come.assert_success();

    let out_come = call!(
        farmer1,
        pool.mft_transfer_call(":0".to_string(), to_va(farming_id()), to_yocto("0.5").into(), None, "".to_string()),
        deposit = 1
    );
    out_come.assert_success();
    let ex_status = format!("{:?}", out_come.promise_errors()[0].as_ref().unwrap().status());
    assert!(ex_status.contains("E11: insufficient $NEAR storage deposit"));
    let user_seeds = show_userseeds(&farming, farmer1.account_id(), false);
    assert!(user_seeds.get(&String::from("swap@0")).is_none());
}

#[test]
fn failure_e11_claim() {
    let root = init_simulator(None);

    let owner = root.create_user("owner".to_string(), to_yocto("100"));
    let farmer1 = root.create_user("farmer1".to_string(), to_yocto("100"));

    let (pool, token1, _) = prepair_pool_and_liquidity(
        &root, &owner, farming_id(), vec![&farmer1]);

    let farming = deploy_farming(&root, farming_id(), owner.account_id());
    call!(farmer1, farming.storage_deposit(None, None), deposit = to_yocto("1")).assert_success();
    

    let farm_id = "swap@0#0".to_string();
    let out_come = call!(
        owner,
        farming.create_simple_farm(HRSimpleFarmTerms{
            seed_id: format!("{}@0", pool.account_id()),
            reward_token: token1.valid_account_id(),
            start_at: 0,
            reward_per_session: to_yocto("1").into(),
            session_interval: 60,
        }, None),
        deposit = to_yocto("1")
    );
    out_come.assert_success();

    call!(
        root,
        token1.storage_deposit(Some(to_va(farming_id())), None),
        deposit = to_yocto("1")
    )
    .assert_success();
    mint_token(&token1, &root, to_yocto("10"));
    call!(
        root,
        token1.ft_transfer_call(to_va(farming_id()), U128(to_yocto("10")), None, farm_id.clone()),
        deposit = 1
    )
    .assert_success();

    let out_come = call!(
        farmer1,
        pool.mft_transfer_call(":0".to_string(), to_va(farming_id()), to_yocto("0.5").into(), None, "".to_string()),
        deposit = 1
    );
    out_come.assert_success();

    call!(farmer1, farming.storage_withdraw(None), deposit = 1).assert_success();

    assert!(root.borrow_runtime_mut().produce_blocks(60).is_ok());

    let out_come = call!(
        farmer1,
        farming.claim_reward_by_seed("swap@0".to_string()),
        deposit = 0
    );
    assert!(!out_come.is_ok());
    let ex_status = format!("{:?}", out_come.promise_errors()[0].as_ref().unwrap().status());
    assert!(ex_status.contains("E11: insufficient $NEAR storage deposit"));
    let reward = show_reward(&farming, farmer1.account_id(), token1.account_id(), false);
    assert_eq!(reward.0, to_yocto("0"));

    let out_come = call!(farmer1, farming.storage_deposit(None, None), deposit = to_yocto("1"));
    out_come.assert_success();
    let sb = out_come.unwrap_json::<StorageBalance>();
    assert_eq!(sb.total.0, to_yocto("1.00341"));
    assert_eq!(sb.available.0, to_yocto("1"));
    
    let out_come = call!(
        farmer1,
        farming.claim_reward_by_seed("swap@0".to_string()),
        deposit = 0
    );
    out_come.assert_success();
}

#[test]
fn failure_e12_e13() {
    let root = init_simulator(None);

    let owner = root.create_user("owner".to_string(), to_yocto("100"));
    let farmer1 = root.create_user("farmer1".to_string(), to_yocto("100"));

    let (pool, token1, _) = prepair_pool_and_liquidity(
        &root, &owner, farming_id(), vec![&farmer1]);

    let farming = deploy_farming(&root, farming_id(), owner.account_id());
    call!(farmer1, farming.storage_deposit(None, None), deposit = to_yocto("1")).assert_success();
    

    let farm_id = "swap@0#0".to_string();
    let out_come = call!(
        owner,
        farming.create_simple_farm(HRSimpleFarmTerms{
            seed_id: format!("{}@0", pool.account_id()),
            reward_token: token1.valid_account_id(),
            start_at: 0,
            reward_per_session: to_yocto("1").into(),
            session_interval: 60,
        }, None),
        deposit = to_yocto("1")
    );
    out_come.assert_success();

    call!(
        root,
        token1.storage_deposit(Some(to_va(farming_id())), None),
        deposit = to_yocto("1")
    )
    .assert_success();
    mint_token(&token1, &root, to_yocto("10"));
    call!(
        root,
        token1.ft_transfer_call(to_va(farming_id()), U128(to_yocto("10")), None, farm_id.clone()),
        deposit = 1
    )
    .assert_success();

    let out_come = call!(
        farmer1,
        pool.mft_transfer_call(":0".to_string(), to_va(farming_id()), to_yocto("0.5").into(), None, "".to_string()),
        deposit = 1
    );
    out_come.assert_success();


    let out_come = call!(farmer1, farming.storage_unregister(None), deposit = 1);
    assert!(!out_come.is_ok());
    let ex_status = format!("{:?}", out_come.promise_errors()[0].as_ref().unwrap().status());
    assert!(ex_status.contains("E13: still has staked seed when unregister"));

    assert!(root.borrow_runtime_mut().produce_blocks(60).is_ok());

    let out_come = call!(
        farmer1,
        farming.claim_reward_by_seed("swap@0".to_string()),
        deposit = 0
    );
    out_come.assert_success();

    let out_come = call!(farmer1, farming.storage_unregister(None), deposit = 1);
    assert!(!out_come.is_ok());
    let ex_status = format!("{:?}", out_come.promise_errors()[0].as_ref().unwrap().status());
    assert!(ex_status.contains("E12: still has rewards when unregister"));

    show_storage_balance(&farming, farmer1.account_id(), false);
}

#[test]
fn failure_e14() {
    let root = init_simulator(None);

    let owner = root.create_user("owner".to_string(), to_yocto("100"));
    let farmer1 = root.create_user("farmer1".to_string(), to_yocto("100"));


    let farming = deploy_farming(&root, farming_id(), owner.account_id());
    call!(farmer1, farming.storage_deposit(None, None), deposit = to_yocto("1")).assert_success();
    
    let out_come = call!(farmer1, farming.storage_deposit(None, Some(true)), deposit = to_yocto("1"));
    assert!(!out_come.is_ok());
    let ex_status = format!("{:?}", out_come.promise_errors()[0].as_ref().unwrap().status());
    assert!(ex_status.contains("E14: account already registered"));

    call!(farmer1, farming.storage_deposit(None, Some(false)), deposit = to_yocto("1")).assert_success();

    let sb = show_storage_balance(&farming, farmer1.account_id(), false);
    assert_eq!(sb.total.0, to_yocto("2"));
}

#[test]
fn failure_e21_e22() {
    let root = init_simulator(None);

    let owner = root.create_user("owner".to_string(), to_yocto("100"));
    let farmer1 = root.create_user("farmer1".to_string(), to_yocto("100"));

    let (pool, token1, _) = prepair_pool_and_liquidity(
        &root, &owner, farming_id(), vec![&farmer1]);

    let farming = deploy_farming(&root, farming_id(), owner.account_id());
    call!(farmer1, farming.storage_deposit(None, None), deposit = to_yocto("1")).assert_success();
    

    let farm_id = "swap@0#0".to_string();
    let out_come = call!(
        owner,
        farming.create_simple_farm(HRSimpleFarmTerms{
            seed_id: format!("{}@0", pool.account_id()),
            reward_token: token1.valid_account_id(),
            start_at: 0,
            reward_per_session: to_yocto("1").into(),
            session_interval: 60,
        }, None),
        deposit = to_yocto("1")
    );
    out_come.assert_success();

    call!(
        root,
        token1.storage_deposit(Some(to_va(farming_id())), None),
        deposit = to_yocto("1")
    )
    .assert_success();
    mint_token(&token1, &root, to_yocto("10"));
    call!(
        root,
        token1.ft_transfer_call(to_va(farming_id()), U128(to_yocto("10")), None, farm_id.clone()),
        deposit = 1
    )
    .assert_success();

    let out_come = call!(
        farmer1,
        pool.mft_transfer_call(":0".to_string(), to_va(farming_id()), to_yocto("0.5").into(), None, "".to_string()),
        deposit = 1
    );
    out_come.assert_success();


    assert!(root.borrow_runtime_mut().produce_blocks(60).is_ok());

    let out_come = call!(
        farmer1,
        farming.withdraw_reward(token1.valid_account_id(), None),
        deposit = 1
    );
    assert!(!out_come.is_ok());
    let ex_status = format!("{:?}", out_come.promise_errors()[0].as_ref().unwrap().status());
    assert!(ex_status.contains("E21: token not registered"));

    let out_come = call!(
        farmer1,
        farming.claim_reward_by_seed("swap@0".to_string()),
        deposit = 0
    );
    out_come.assert_success();

    let reward = show_reward(&farming, farmer1.account_id(), token1.account_id(), false);
    assert_eq!(reward.0, to_yocto("1"));

    let out_come = call!(
        farmer1,
        farming.withdraw_reward(token1.valid_account_id(), Some(U128(to_yocto("1.1")))),
        deposit = 1
    );
    assert!(!out_come.is_ok());
    let ex_status = format!("{:?}", out_come.promise_errors()[0].as_ref().unwrap().status());
    assert!(ex_status.contains("E22: not enough tokens in deposit"));

    let out_come = call!(
        farmer1,
        farming.withdraw_reward(token1.valid_account_id(), None),
        deposit = 1
    );
    out_come.assert_success();

    let reward = show_reward(&farming, farmer1.account_id(), token1.account_id(), false);
    assert_eq!(reward.0, to_yocto("0"));


}

#[test]
fn failure_e25_withdraw_reward() {
    let root = init_simulator(None);

    let owner = root.create_user("owner".to_string(), to_yocto("100"));
    let farmer1 = root.create_user("farmer1".to_string(), to_yocto("100"));

    let (pool, token1, _) = prepair_pool_and_liquidity(
        &root, &owner, farming_id(), vec![&farmer1]);

    let farming = deploy_farming(&root, farming_id(), owner.account_id());
    call!(farmer1, farming.storage_deposit(None, None), deposit = to_yocto("1")).assert_success();
    

    let farm_id = "swap@0#0".to_string();
    let out_come = call!(
        owner,
        farming.create_simple_farm(HRSimpleFarmTerms{
            seed_id: format!("{}@0", pool.account_id()),
            reward_token: token1.valid_account_id(),
            start_at: 0,
            reward_per_session: to_yocto("1").into(),
            session_interval: 60,
        }, None),
        deposit = to_yocto("1")
    );
    out_come.assert_success();

    call!(
        root,
        token1.storage_deposit(Some(to_va(farming_id())), None),
        deposit = to_yocto("1")
    )
    .assert_success();
    mint_token(&token1, &root, to_yocto("10"));
    call!(
        root,
        token1.ft_transfer_call(to_va(farming_id()), U128(to_yocto("10")), None, farm_id.clone()),
        deposit = 1
    )
    .assert_success();

    let out_come = call!(
        farmer1,
        pool.mft_transfer_call(":0".to_string(), to_va(farming_id()), to_yocto("0.5").into(), None, "".to_string()),
        deposit = 1
    );
    out_come.assert_success();


    assert!(root.borrow_runtime_mut().produce_blocks(60).is_ok());

    let out_come = call!(
        farmer1,
        farming.claim_reward_by_seed("swap@0".to_string()),
        deposit = 0
    );
    out_come.assert_success();

    let reward = show_reward(&farming, farmer1.account_id(), token1.account_id(), false);
    assert_eq!(reward.0, to_yocto("1"));

    call!(farmer1, token1.storage_unregister(Some(true)), deposit = 1).assert_success();

    let out_come = call!(
        farmer1,
        farming.withdraw_reward(token1.valid_account_id(), None),
        deposit = 1
    );
    out_come.assert_success();
    let ex_status = format!("{:?}", out_come.promise_errors()[0].as_ref().unwrap().status());
    assert!(ex_status.contains("The account farmer1 is not registered"));

    let reward = show_reward(&farming, farmer1.account_id(), token1.account_id(), false);
    assert_eq!(reward.0, to_yocto("1"));
}

#[test]
fn failure_e25_withdraw_seed_ft() {
    let root = init_simulator(None);

    let owner = root.create_user("owner".to_string(), to_yocto("100"));
    let farmer1 = root.create_user("farmer1".to_string(), to_yocto("100"));

    let (_, token1, token2) = prepair_pool(&root, &owner);

    call!(
        root, token2.mint(farmer1.valid_account_id(), to_yocto("10000").into())
    )
    .assert_success();

    let farming = deploy_farming(&root, farming_id(), owner.account_id());
    call!(farmer1, farming.storage_deposit(None, None), deposit = to_yocto("1")).assert_success();
    call!(farming.user_account, token2.storage_deposit(None, None), deposit = to_yocto("1")).assert_success();

    let out_come = call!(
        owner,
        farming.create_simple_farm(HRSimpleFarmTerms{
            seed_id: format!("{}", token2.account_id()),
            reward_token: token1.valid_account_id(),
            start_at: 0,
            reward_per_session: to_yocto("1").into(),
            session_interval: 60,
        }, Some(U128(100))),
        deposit = to_yocto("1")
    );
    out_come.assert_success();

    let out_come = call!(
        farmer1,
        token2.ft_transfer_call(to_va(farming_id()), U128(500), None, "".to_string()),
        deposit = 1
    );
    out_come.assert_success();


    call!(farmer1, token2.storage_unregister(Some(true)), deposit = 1).assert_success();

    let out_come = call!(
        farmer1,
        farming.withdraw_seed(token2.account_id(), U128(100)),
        deposit = 1
    );
    out_come.assert_success();
    let ex_status = format!("{:?}", out_come.promise_errors()[0].as_ref().unwrap().status());
    assert!(ex_status.contains("The account farmer1 is not registered"));

    let user_seeds = show_userseeds(&farming, farmer1.account_id(), false);
    assert_eq!(user_seeds.get(&format!("{}", token2.account_id())).unwrap().0, 500);
}

#[test]
fn failure_e31_unstake_seed() {
    let root = init_simulator(None);

    let owner = root.create_user("owner".to_string(), to_yocto("100"));
    let farmer1 = root.create_user("farmer1".to_string(), to_yocto("100"));

    let (pool, token1, _) = prepair_pool_and_liquidity(
        &root, &owner, farming_id(), vec![&farmer1]);

    let farming = deploy_farming(&root, farming_id(), owner.account_id());
    call!(farmer1, farming.storage_deposit(None, None), deposit = to_yocto("1")).assert_success();
    
    let out_come = call!(
        owner,
        farming.create_simple_farm(HRSimpleFarmTerms{
            seed_id: format!("{}@0", pool.account_id()),
            reward_token: token1.valid_account_id(),
            start_at: 0,
            reward_per_session: to_yocto("1").into(),
            session_interval: 60,
        }, None),
        deposit = to_yocto("1")
    );
    out_come.assert_success();

    let out_come = call!(
        farmer1,
        pool.mft_transfer_call(":0".to_string(), to_va(farming_id()), to_yocto("0.5").into(), None, "".to_string()),
        deposit = 1
    );
    out_come.assert_success();


    let out_come = call!(
        farmer1,
        farming.withdraw_seed(format!("{}@1", pool.account_id()), to_yocto("0.5").into()),
        deposit = 1
    );
    assert!(!out_come.is_ok());
    let ex_status = format!("{:?}", out_come.promise_errors()[0].as_ref().unwrap().status());
    assert!(ex_status.contains("E31: seed not exist"));
}

#[test]
fn failure_e31_stake_seed() {
    let root = init_simulator(None);

    let owner = root.create_user("owner".to_string(), to_yocto("100"));
    let farmer1 = root.create_user("farmer1".to_string(), to_yocto("100"));

    let (pool, _, _) = prepair_pool_and_liquidity(
        &root, &owner, farming_id(), vec![&farmer1]);

    let farming = deploy_farming(&root, farming_id(), owner.account_id());
    call!(farmer1, farming.storage_deposit(None, None), deposit = to_yocto("1")).assert_success();
    

    let out_come = call!(
        farmer1,
        pool.mft_transfer_call(":0".to_string(), to_va(farming_id()), to_yocto("0.5").into(), None, "".to_string()),
        deposit = 1
    );
    out_come.assert_success();
    let ex_status = format!("{:?}", out_come.promise_errors()[0].as_ref().unwrap().status());
    assert!(ex_status.contains("E31: seed not exist"));
}

#[test]
fn failure_e32_unstake_over_balance() {
    let root = init_simulator(None);

    let owner = root.create_user("owner".to_string(), to_yocto("100"));
    let farmer1 = root.create_user("farmer1".to_string(), to_yocto("100"));

    let (pool, token1, _) = prepair_pool_and_liquidity(
        &root, &owner, farming_id(), vec![&farmer1]);

    let farming = deploy_farming(&root, farming_id(), owner.account_id());
    call!(farmer1, farming.storage_deposit(None, None), deposit = to_yocto("1")).assert_success();

    let out_come = call!(
        owner,
        farming.create_simple_farm(HRSimpleFarmTerms{
            seed_id: format!("{}@0", pool.account_id()),
            reward_token: token1.valid_account_id(),
            start_at: 0,
            reward_per_session: to_yocto("1").into(),
            session_interval: 60,
        }, None),
        deposit = to_yocto("1")
    );
    out_come.assert_success();


    let out_come = call!(
        farmer1,
        pool.mft_transfer_call(":0".to_string(), to_va(farming_id()), U128(to_yocto("0.5")), None, "".to_string()),
        deposit = 1
    );
    out_come.assert_success();
    let out_come = call!(
        farmer1,
        farming.withdraw_seed("swap@0".to_string(), to_yocto("0.6").into()),
        deposit = 1
    );
    assert!(!out_come.is_ok());
    let ex_status = format!("{:?}", out_come.promise_errors()[0].as_ref().unwrap().status());
    // println!("ex_status: {}", ex_status);
    assert!(ex_status.contains("E32: not enough amount of seed"));
    let user_seeds = show_userseeds(&farming, farmer1.account_id(), false);
    assert_eq!(user_seeds.get(&String::from("swap@0")).unwrap().0, to_yocto("0.5"));
}

#[test]
fn failure_e33() {
    let root = init_simulator(None);

    let owner = root.create_user("owner".to_string(), to_yocto("100"));
    let farmer1 = root.create_user("farmer1".to_string(), to_yocto("100"));

    let (pool, token1, _) = prepair_pool_and_liquidity(
        &root, &owner, farming_id(), vec![&farmer1]);

    let farming = deploy_farming(&root, farming_id(), owner.account_id());
    call!(farmer1, farming.storage_deposit(None, None), deposit = to_yocto("1")).assert_success();

    let out_come = call!(
        owner,
        farming.create_simple_farm(HRSimpleFarmTerms{
            seed_id: format!("{}@0@3", pool.account_id()),
            reward_token: token1.valid_account_id(),
            start_at: 0,
            reward_per_session: to_yocto("1").into(),
            session_interval: 60,
        }, None),
        deposit = to_yocto("1")
    );
    assert!(!out_come.is_ok());
    let ex_status = format!("{:?}", out_come.promise_errors()[0].as_ref().unwrap().status());
    // println!("ex_status: {}", ex_status);
    assert!(ex_status.contains("E33: invalid seed id"));
}

#[test]
fn failure_e34_stake_below_minimum() {
    let root = init_simulator(None);

    let owner = root.create_user("owner".to_string(), to_yocto("100"));
    let farmer1 = root.create_user("farmer1".to_string(), to_yocto("100"));

    let (pool, token1, _) = prepair_pool_and_liquidity(
        &root, &owner, farming_id(), vec![&farmer1]);

    let farming = deploy_farming(&root, farming_id(), owner.account_id());
    call!(farmer1, farming.storage_deposit(None, None), deposit = to_yocto("1")).assert_success();

    let out_come = call!(
        owner,
        farming.create_simple_farm(HRSimpleFarmTerms{
            seed_id: format!("{}@0", pool.account_id()),
            reward_token: token1.valid_account_id(),
            start_at: 0,
            reward_per_session: to_yocto("1").into(),
            session_interval: 60,
        }, None),
        deposit = to_yocto("1")
    );
    out_come.assert_success();

    let out_come = call!(
        farmer1,
        pool.mft_transfer_call(":0".to_string(), to_va(farming_id()), to_yocto("0.0000001").into(), None, "".to_string()),
        deposit = 1
    );
    out_come.assert_success();
    let ex_status = format!("{:?}", out_come.promise_errors()[0].as_ref().unwrap().status());
    // println!("ex_status: {}", ex_status);
    assert!(ex_status.contains("E34: below min_deposit of this seed"));
    let user_seeds = show_userseeds(&farming, farmer1.account_id(), false);
    assert!(user_seeds.get(&String::from("swap@0")).is_none());
}

#[test]
fn failure_e41_when_deposit_reward_token() {
    let root = init_simulator(None);

    let owner = root.create_user("owner".to_string(), to_yocto("100"));
    let farmer1 = root.create_user("farmer1".to_string(), to_yocto("100"));

    let (pool, token1, _) = prepair_pool_and_liquidity(
        &root, &owner, farming_id(), vec![&farmer1]);

    let farming = deploy_farming(&root, farming_id(), owner.account_id());
    call!(farmer1, farming.storage_deposit(None, None), deposit = to_yocto("1")).assert_success();

    let out_come = call!(
        owner,
        farming.create_simple_farm(HRSimpleFarmTerms{
            seed_id: format!("{}@0", pool.account_id()),
            reward_token: to_va(token1.account_id()),
            start_at: 0,
            reward_per_session: to_yocto("1").into(),
            session_interval: 60,
        }, None),
        deposit = to_yocto("1")
    );
    out_come.assert_success();

    // deposit reward
    call!(
        root,
        token1.storage_deposit(Some(to_va(farming_id())), None),
        deposit = to_yocto("1")
    )
    .assert_success();
    // call with INVALID farm id
    mint_token(&token1, &root, to_yocto("10"));
    let calldata = call!(
        root,
        token1.ft_transfer_call(to_va(farming_id()), U128(to_yocto("1")), None, "swap@0#1".to_string().clone()),
        deposit = 1
    );
    calldata.assert_success();
    let ex_status = format!("{:?}", out_come.promise_errors()[0].as_ref().unwrap().status());
    assert!(ex_status.contains("E41: farm not exist"));
}

#[test]
fn failure_e42_when_force_clean_farm() {
    let root = init_simulator(None);

    let owner = root.create_user("owner".to_string(), to_yocto("100"));
    let farmer1 = root.create_user("farmer1".to_string(), to_yocto("100"));

    let (pool, token1, _) = prepair_pool_and_liquidity(
        &root, &owner, farming_id(), vec![&farmer1]);

    let farm_id = "swap@0#0".to_string();
    let farming = deploy_farming(&root, farming_id(), owner.account_id());
    call!(farmer1, farming.storage_deposit(None, None), deposit = to_yocto("1")).assert_success();

    let out_come = call!(
        owner,
        farming.create_simple_farm(HRSimpleFarmTerms{
            seed_id: format!("{}@0", pool.account_id()),
            reward_token: to_va(token1.account_id()),
            start_at: 0,
            reward_per_session: to_yocto("1").into(),
            session_interval: 60,
        }, None),
        deposit = to_yocto("1")
    );
    out_come.assert_success();

    // deposit reward
    call!(
        root,
        token1.storage_deposit(Some(to_va(farming_id())), None),
        deposit = to_yocto("1")
    )
    .assert_success();
    mint_token(&token1, &root, to_yocto("10"));
    let calldata = call!(
        root,
        token1.ft_transfer_call(to_va(farming_id()), U128(to_yocto("1")), None, farm_id.clone()),
        deposit = 1
    );
    calldata.assert_success();

    // farmer1 staking lpt 
    println!("----->> Farmer1 staking lpt.");
    let out_come = call!(
        farmer1,
        pool.mft_transfer_call(":0".to_string(), to_va(farming_id()), to_yocto("1").into(), None, "".to_string()),
        deposit = 1
    );
    out_come.assert_success();

    // move to 90 seconds later
    assert!(root.borrow_runtime_mut().produce_blocks(90).is_ok());

    let farm_info = show_farminfo(&farming, farm_id.clone(), false);
    assert_farming(&farm_info, "Ended".to_string(), to_yocto("1"), 1, 0, 0, to_yocto("1"), 0);

    let out_come = call!(
        owner,
        farming.force_clean_farm("swap".to_string().clone()),
        deposit = 0
    );
    let ex_status = format!("{:?}", out_come.promise_errors()[0].as_ref().unwrap().status());
    assert!(ex_status.contains("E42: invalid farm id"));
}

#[test]
fn failure_e42_when_claim_reward() {
    let root = init_simulator(None);

    let owner = root.create_user("owner".to_string(), to_yocto("100"));
    let farmer1 = root.create_user("farmer1".to_string(), to_yocto("100"));

    let (pool, token1, _) = prepair_pool_and_liquidity(
        &root, &owner, farming_id(), vec![&farmer1]);

    let farm_id = "swap@0#0".to_string();
    let farming = deploy_farming(&root, farming_id(), owner.account_id());
    call!(farmer1, farming.storage_deposit(None, None), deposit = to_yocto("1")).assert_success();

    let out_come = call!(
        owner,
        farming.create_simple_farm(HRSimpleFarmTerms{
            seed_id: format!("{}@0", pool.account_id()),
            reward_token: to_va(token1.account_id()),
            start_at: 0,
            reward_per_session: to_yocto("1").into(),
            session_interval: 60,
        }, None),
        deposit = to_yocto("1")
    );
    out_come.assert_success();

    // deposit reward
    call!(
        root,
        token1.storage_deposit(Some(to_va(farming_id())), None),
        deposit = to_yocto("1")
    )
    .assert_success();
    mint_token(&token1, &root, to_yocto("10"));
    let calldata = call!(
        root,
        token1.ft_transfer_call(to_va(farming_id()), U128(to_yocto("1")), None, farm_id.clone()),
        deposit = 1
    );
    calldata.assert_success();

    // farmer1 staking lpt 
    println!("----->> Farmer1 staking lpt.");
    let out_come = call!(
        farmer1,
        pool.mft_transfer_call(":0".to_string(), to_va(farming_id()), to_yocto("1").into(), None, "".to_string()),
        deposit = 1
    );
    out_come.assert_success();

    let out_come = call!(
        farmer1,
        farming.claim_reward_by_farm("swap".to_string().clone()),
        deposit = 0
    );
    let ex_status = format!("{:?}", out_come.promise_errors()[0].as_ref().unwrap().status());
    assert!(ex_status.contains("E42: invalid farm id"));
}

#[test]
fn failure_e42_when_remove_user_rps_and_view_unclaim_reward() {
    let root = init_simulator(None);

    let owner = root.create_user("owner".to_string(), to_yocto("100"));
    let farmer1 = root.create_user("farmer1".to_string(), to_yocto("100"));

    let (pool, token1, _) = prepair_pool_and_liquidity(
        &root, &owner, farming_id(), vec![&farmer1]);

    let farm_id = "swap@0#0".to_string();
    let farming = deploy_farming(&root, farming_id(), owner.account_id());
    call!(farmer1, farming.storage_deposit(None, None), deposit = to_yocto("1")).assert_success();

    let out_come = call!(
        owner,
        farming.create_simple_farm(HRSimpleFarmTerms{
            seed_id: format!("{}@0", pool.account_id()),
            reward_token: to_va(token1.account_id()),
            start_at: 0,
            reward_per_session: to_yocto("1").into(),
            session_interval: 60,
        }, None),
        deposit = to_yocto("1")
    );
    out_come.assert_success();

    // deposit reward
    call!(
        root,
        token1.storage_deposit(Some(to_va(farming_id())), None),
        deposit = to_yocto("1")
    )
    .assert_success();
    mint_token(&token1, &root, to_yocto("10"));
    let calldata = call!(
        root,
        token1.ft_transfer_call(to_va(farming_id()), U128(to_yocto("1")), None, farm_id.clone()),
        deposit = 1
    );
    calldata.assert_success();

    // farmer1 staking lpt 
    println!("----->> Farmer1 staking lpt.");
    let out_come = call!(
        farmer1,
        pool.mft_transfer_call(":0".to_string(), to_va(farming_id()), to_yocto("1").into(), None, "".to_string()),
        deposit = 1
    );
    out_come.assert_success();

    // should panic when remove_user_rps_by_farm
    let out_come = call!(
        farmer1,
        farming.remove_user_rps_by_farm("swap".to_string().clone()),
        deposit = 0
    );
    let ex_status = format!("{:?}", out_come.promise_errors()[0].as_ref().unwrap().status());
    assert!(ex_status.contains("E42: invalid farm id"));

    // should panic when get_unclaim_reward
    let out_come = call!(
        farmer1,
        farming.get_unclaimed_reward(to_va(farmer1.account_id()), "swap".to_string().clone()),
        deposit = 0
    );
    let ex_status = format!("{:?}", out_come.promise_errors()[0].as_ref().unwrap().status());
    assert!(ex_status.contains("E42: invalid farm id"));
}

#[test]
fn failure_e43() {
    let root = init_simulator(None);

    let owner = root.create_user("owner".to_string(), to_yocto("100"));
    let farmer1 = root.create_user("farmer1".to_string(), to_yocto("100"));

    let (pool, token1, _) = prepair_pool_and_liquidity(
        &root, &owner, farming_id(), vec![&farmer1]);

    let farm_id = "swap@0#0".to_string();
    let farming = deploy_farming(&root, farming_id(), owner.account_id());
    call!(farmer1, farming.storage_deposit(None, None), deposit = to_yocto("1")).assert_success();

    let out_come = call!(
        owner,
        farming.create_simple_farm(HRSimpleFarmTerms{
            seed_id: format!("{}@0", pool.account_id()),
            reward_token: to_va(token1.account_id()),
            start_at: 0,
            reward_per_session: to_yocto("1").into(),
            session_interval: 60,
        }, None),
        deposit = to_yocto("1")
    );
    out_come.assert_success();

    // deposit reward
    call!(
        root,
        token1.storage_deposit(Some(to_va(farming_id())), None),
        deposit = to_yocto("1")
    )
    .assert_success();
    mint_token(&token1, &root, to_yocto("10"));
    let calldata = call!(
        root,
        token1.ft_transfer_call(to_va(farming_id()), U128(to_yocto("1")), None, farm_id.clone()),
        deposit = 1
    );
    calldata.assert_success();

    // farmer1 staking lpt 
    println!("----->> Farmer1 staking lpt.");
    let out_come = call!(
        farmer1,
        pool.mft_transfer_call(":0".to_string(), to_va(farming_id()), to_yocto("1").into(), None, "".to_string()),
        deposit = 1
    );
    out_come.assert_success();

    // move to 90 seconds later
    assert!(root.borrow_runtime_mut().produce_blocks(90).is_ok());

    let farm_info = show_farminfo(&farming, farm_id.clone(), false);
    assert_farming(&farm_info, "Ended".to_string(), to_yocto("1"), 1, 0, 0, to_yocto("1"), 0);

    // should panic when trying to deposit again
    let calldata = call!(
        root,
        token1.ft_transfer_call(to_va(farming_id()), U128(to_yocto("1")), None, farm_id.clone()),
        deposit = 1
    );
    calldata.assert_success();
    let ex_status = format!("{:?}", calldata.promise_errors()[0].as_ref().unwrap().status());
    assert!(ex_status.contains("E43: invalid farm status"));
}

#[test]
fn failure_e44() {
    let root = init_simulator(None);

    let owner = root.create_user("owner".to_string(), to_yocto("100"));
    let farmer1 = root.create_user("farmer1".to_string(), to_yocto("100"));

    let (pool, token1, token2) = prepair_pool_and_liquidity(
        &root, &owner, farming_id(), vec![&farmer1]);

    let farm_id = "swap@0#0".to_string();
    let farming = deploy_farming(&root, farming_id(), owner.account_id());
    call!(farmer1, farming.storage_deposit(None, None), deposit = to_yocto("1")).assert_success();

    let out_come = call!(
        owner,
        farming.create_simple_farm(HRSimpleFarmTerms{
            seed_id: format!("{}@0", pool.account_id()),
            reward_token: to_va(token1.account_id()),
            start_at: 0,
            reward_per_session: to_yocto("1").into(),
            session_interval: 60,
        }, None),
        deposit = to_yocto("1")
    );
    out_come.assert_success();

    // deposit wrong reward
    mint_token(&token2, &root, to_yocto("10"));
    call!(
        root,
        token2.storage_deposit(Some(to_va(farming_id())), None),
        deposit = to_yocto("1")
    )
    .assert_success();
    let calldata = call!(
        root,
        token2.ft_transfer_call(to_va(farming_id()), U128(to_yocto("1")), None, farm_id.clone()),
        deposit = 1
    );
    calldata.assert_success();
    let ex_status = format!("{:?}", calldata.promise_errors()[0].as_ref().unwrap().status());
    assert!(ex_status.contains("E44: invalid reward token for this farm"));
}
