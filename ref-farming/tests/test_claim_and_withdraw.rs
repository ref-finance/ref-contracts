use near_sdk_sim::{call, init_simulator, to_yocto, view};
use near_sdk::json_types::{U128};
use near_sdk::serde_json::Value;

use ref_farming::{HRSimpleFarmTerms};

use crate::common::utils::*;
use crate::common::init::deploy_farming;
use crate::common::views::*;
use crate::common::actions::*;

mod common;

#[test]
fn claim_and_withdraw_0() {
    let root = init_simulator(None);

    let owner = root.create_user("owner".to_string(), to_yocto("100"));
    let farmer1 = root.create_user("farmer1".to_string(), to_yocto("100"));
    let farmer2 = root.create_user("farmer2".to_string(), to_yocto("100"));

    let (pool, token1, token2) = prepair_pool_and_liquidity(
        &root, &owner, farming_id(), vec![&farmer1, &farmer2]);

    let farming = deploy_farming(&root, farming_id(), owner.account_id());
    call!(farmer1, farming.storage_deposit(None, None), deposit = to_yocto("1")).assert_success();
    call!(farmer2, farming.storage_deposit(None, None), deposit = to_yocto("1")).assert_success();

    let farm1_id = "swap@0#0".to_string();
    let farm2_id = "swap@0#1".to_string();
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
        owner,
        farming.create_simple_farm(HRSimpleFarmTerms{
            seed_id: format!("{}@0", pool.account_id()),
            reward_token: token2.valid_account_id(),
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
        token1.ft_transfer_call(to_va(farming_id()), U128(to_yocto("10")), None, farm1_id.clone()),
        deposit = 1
    )
    .assert_success();

    call!(
        root,
        token2.storage_deposit(Some(to_va(farming_id())), None),
        deposit = to_yocto("1")
    )
    .assert_success();
    mint_token(&token2, &root, to_yocto("10"));
    call!(
        root,
        token2.ft_transfer_call(to_va(farming_id()), U128(to_yocto("10")), None, farm2_id.clone()),
        deposit = 1
    )
    .assert_success();

    let out_come = call!(
        farmer1,
        pool.mft_transfer_call(":0".to_string(), to_va(farming_id()), to_yocto("1").into(), None, "".to_string()),
        deposit = 1
    );
    out_come.assert_success();
    assert_eq!(out_come.promise_errors().len(), 0);
    let out_come = call!(
        farmer2,
        pool.mft_transfer_call(":0".to_string(), to_va(farming_id()), to_yocto("1").into(), None, "".to_string()),
        deposit = 1
    );
    out_come.assert_success();
    assert_eq!(out_come.promise_errors().len(), 0);

    assert!(root.borrow_runtime_mut().produce_blocks(60).is_ok());
    let farm_info = show_farminfo(&farming, farm1_id.clone(), false);
    assert_farming(&farm_info, "Running".to_string(), to_yocto("10"), 1, 0, 0, to_yocto("1"), 0);
    let farm_info = show_farminfo(&farming, farm2_id.clone(), false);
    assert_farming(&farm_info, "Running".to_string(), to_yocto("10"), 1, 0, 0, to_yocto("1"), 0);
    let unclaim = show_unclaim(&farming, farmer1.account_id(), farm1_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("0.5"));
    let unclaim = show_unclaim(&farming, farmer1.account_id(), farm2_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("0.5"));
    let unclaim = show_unclaim(&farming, farmer2.account_id(), farm1_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("0.5"));
    let unclaim = show_unclaim(&farming, farmer2.account_id(), farm2_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("0.5"));

    println!("Case0301 claim_and_withdraw_by_farm");
    let out_come = call!(
        farmer1,
        farming.claim_and_withdraw_by_farm(farm1_id.clone(), false),
        deposit = 0
    );
    out_come.assert_success();
    assert_eq!(out_come.promise_errors().len(), 0);
    let farm_info = show_farminfo(&farming, farm1_id.clone(), false);
    assert_farming(&farm_info, "Running".to_string(), to_yocto("10"), 1, 1, to_yocto("0.5"), to_yocto("0.5"), 0);
    let unclaim = show_unclaim(&farming, farmer1.account_id(), farm1_id.clone(), false);
    assert_eq!(unclaim.0, 0_u128);
    let unclaim = show_unclaim(&farming, farmer1.account_id(), farm2_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("0.5"));
    let reward = show_reward(&farming, farmer1.account_id(), token1.account_id(), false);
    assert_eq!(reward.0, to_yocto("0"));
    let balance = balance_of(&token1, farmer1.account_id());
    assert_eq!(balance, to_yocto("5.5"));

    println!("Case0302 claim_and_withdraw_by_seed.");
    assert!(root.borrow_runtime_mut().produce_blocks(60).is_ok());
    let out_come = call!(
        farmer2,
        farming.claim_and_withdraw_by_seed(farm_info.seed_id.clone()),
        deposit = 0
    );
    out_come.assert_success();
    assert_eq!(out_come.promise_errors().len(), 0);
    let farm_info = show_farminfo(&farming, farm1_id.clone(), false);
    assert_farming(&farm_info, "Running".to_string(), to_yocto("10"), 2, 2, to_yocto("1.5"), to_yocto("0.5"), 0);
    let farm_info = show_farminfo(&farming, farm2_id.clone(), false);
    assert_farming(&farm_info, "Running".to_string(), to_yocto("10"), 2, 2, to_yocto("1"), to_yocto("1"), 0);
    let reward = show_reward(&farming, farmer2.account_id(), token1.account_id(), false);
    assert_eq!(reward.0, to_yocto("0"));
    let reward = show_reward(&farming, farmer2.account_id(), token2.account_id(), false);
    assert_eq!(reward.0, to_yocto("0"));
    let balance = balance_of(&token1, farmer2.account_id());
    assert_eq!(balance, to_yocto("6"));
    let balance = balance_of(&token2, farmer2.account_id());
    assert_eq!(balance, to_yocto("6"));

    assert!(root.borrow_runtime_mut().produce_blocks(60).is_ok());
    let out_come = call!(
        farmer1,
        farming.withdraw_seed(farm_info.seed_id.clone(), to_yocto("0.5").into()),
        deposit = 1
    );
    out_come.assert_success();
    assert_eq!(out_come.promise_errors().len(), 0);
    let out_come = call!(
        farmer2,
        farming.withdraw_seed(farm_info.seed_id.clone(), to_yocto("0.5").into()),
        deposit = 1
    );
    out_come.assert_success();
    assert_eq!(out_come.promise_errors().len(), 0);
    let reward = show_reward(&farming, farmer1.account_id(), token1.account_id(), false);
    assert_eq!(reward.0, to_yocto("1"));
    let reward = show_reward(&farming, farmer1.account_id(), token2.account_id(), false);
    assert_eq!(reward.0, to_yocto("1.5"));

    println!("Case0303 claim_and_withdraw_by_seed after seed change.");
    let out_come = call!(
        farmer1,
        farming.claim_and_withdraw_by_seed(farm_info.seed_id.clone()),
        deposit = 0
    );
    out_come.assert_success();
    assert_eq!(out_come.promise_errors().len(), 0);
    let reward = show_reward(&farming, farmer1.account_id(), token1.account_id(), false);
    assert_eq!(reward.0, to_yocto("0"));
    let reward = show_reward(&farming, farmer1.account_id(), token2.account_id(), false);
    assert_eq!(reward.0, to_yocto("0"));
    let balance = balance_of(&token1, farmer1.account_id());
    assert_eq!(balance, to_yocto("6.5"));
    let balance = balance_of(&token2, farmer1.account_id());
    assert_eq!(balance, to_yocto("6.5"));
    
    println!("Case0304 claim_and_withdraw_by_farm after seed change.");
    assert!(root.borrow_runtime_mut().produce_blocks(60).is_ok());
    let out_come = call!(
        farmer2,
        farming.claim_and_withdraw_by_farm(farm1_id.clone(), true),
        deposit = 0
    );
    out_come.assert_success();
    assert_eq!(out_come.promise_errors().len(), 0);
    let unclaim = show_unclaim(&farming, farmer2.account_id(), farm2_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("0.5"));
    let reward = show_reward(&farming, farmer2.account_id(), token1.account_id(), false);
    assert_eq!(reward.0, to_yocto("0"));
    let reward = show_reward(&farming, farmer2.account_id(), token2.account_id(), false);
    assert_eq!(reward.0, to_yocto("0"));
    let balance = balance_of(&token1, farmer2.account_id());
    assert_eq!(balance, to_yocto("7"));
    let balance = balance_of(&token2, farmer2.account_id());
    assert_eq!(balance, to_yocto("6.5"));

    // send token failure
    println!("Case0305 claim_and_withdraw_by_seed with tokens unregstered.");
    assert!(root.borrow_runtime_mut().produce_blocks(80).is_ok());
    let farm_info = show_farminfo(&farming, farm1_id.clone(), false);
    assert_farming(&farm_info, "Running".to_string(), to_yocto("10"), 6, 4, to_yocto("3.5"), to_yocto("2.5"), 0);
    let farm_info = show_farminfo(&farming, farm2_id.clone(), false);
    assert_farming(&farm_info, "Running".to_string(), to_yocto("10"), 6, 3, to_yocto("3"), to_yocto("3"), 0);
    let reward = show_reward(&farming, farmer1.account_id(), token1.account_id(), false);
    assert_eq!(reward.0, to_yocto("0"));
    let reward = show_reward(&farming, farmer1.account_id(), token2.account_id(), false);
    assert_eq!(reward.0, to_yocto("0"));
    let unclaim = show_unclaim(&farming, farmer1.account_id(), farm1_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("1.5"));
    let unclaim = show_unclaim(&farming, farmer1.account_id(), farm2_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("1.5"));
    call!(farmer1, token1.storage_unregister(Some(true)), deposit = 1).assert_success();
    call!(farmer1, token2.storage_unregister(Some(true)), deposit = 1).assert_success();
    let out_come = call!(
        farmer1,
        farming.claim_and_withdraw_by_seed(farm_info.seed_id.clone()),
        deposit = 0
    );
    out_come.assert_success();
    assert_eq!(out_come.promise_errors().len(), 2);
    let reward = show_reward(&farming, farmer1.account_id(), token1.account_id(), false);
    assert_eq!(reward.0, to_yocto("1.5"));
    let reward = show_reward(&farming, farmer1.account_id(), token2.account_id(), false);
    assert_eq!(reward.0, to_yocto("1.5"));

    // token1 registered
    call!(farmer1, token1.storage_deposit(None, None), deposit = to_yocto("1")).assert_success();
    let out_come = call!(
        farmer1,
        farming.claim_and_withdraw_by_seed(farm_info.seed_id.clone()),
        deposit = 0
    );
    out_come.assert_success();
    assert_eq!(out_come.promise_errors().len(), 1);
    let reward = show_reward(&farming, farmer1.account_id(), token1.account_id(), false);
    assert_eq!(reward.0, to_yocto("0"));
    let reward = show_reward(&farming, farmer1.account_id(), token2.account_id(), false);
    assert_eq!(reward.0, to_yocto("1.5"));
    let balance = balance_of(&token1, farmer1.account_id());
    assert_eq!(balance, to_yocto("1.5"));

    // token2 registered
    call!(farmer1, token2.storage_deposit(None, None), deposit = to_yocto("1")).assert_success();
    let out_come = call!(
        farmer1,
        farming.claim_and_withdraw_by_seed(farm_info.seed_id.clone()),
        deposit = 0
    );
    out_come.assert_success();
    assert_eq!(out_come.promise_errors().len(), 0);
    let reward = show_reward(&farming, farmer1.account_id(), token1.account_id(), false);
    assert_eq!(reward.0, to_yocto("0"));
    let reward = show_reward(&farming, farmer1.account_id(), token2.account_id(), false);
    assert_eq!(reward.0, to_yocto("0"));
    let balance = balance_of(&token1, farmer1.account_id());
    assert_eq!(balance, to_yocto("1.5"));
    let balance = balance_of(&token2, farmer1.account_id());
    assert_eq!(balance, to_yocto("1.5"));

    // normal withdraw
    println!("Case0306 normal withdraw.");
    assert!(root.borrow_runtime_mut().produce_blocks(60).is_ok());
    let farm_info = show_farminfo(&farming, farm1_id.clone(), false);
    assert_farming(&farm_info, "Running".to_string(), to_yocto("10"), 7, 6, to_yocto("5"), to_yocto("2"), 0);
    let farm_info = show_farminfo(&farming, farm2_id.clone(), false);
    assert_farming(&farm_info, "Running".to_string(), to_yocto("10"), 7, 6, to_yocto("4.5"), to_yocto("2.5"), 0);
    let out_come = call!(
        farmer1,
        pool.mft_transfer_call(":0".to_string(), to_va(farming_id()), to_yocto("0.5").into(), None, "".to_string()),
        deposit = 1
    );
    out_come.assert_success();
    let out_come = call!(
        farmer2,
        pool.mft_transfer_call(":0".to_string(), to_va(farming_id()), to_yocto("0.5").into(), None, "".to_string()),
        deposit = 1
    );
    out_come.assert_success();
    let reward = show_reward(&farming, farmer1.account_id(), token1.account_id(), false);
    assert_eq!(reward.0, to_yocto("0.5"));
    let reward = show_reward(&farming, farmer1.account_id(), token2.account_id(), false);
    assert_eq!(reward.0, to_yocto("0.5"));
    let reward = show_reward(&farming, farmer2.account_id(), token1.account_id(), false);
    assert_eq!(reward.0, to_yocto("1.5"));
    let reward = show_reward(&farming, farmer2.account_id(), token2.account_id(), false);
    assert_eq!(reward.0, to_yocto("2"));
    let out_come = call!(
        farmer2,
        farming.withdraw_reward(token1.valid_account_id(), None),
        deposit = 1
    );
    out_come.assert_success();
    assert_eq!(out_come.promise_errors().len(), 0);
    let reward = show_reward(&farming, farmer2.account_id(), token1.account_id(), false);
    assert_eq!(reward.0, to_yocto("0"));
    let balance = balance_of(&token1, farmer2.account_id());
    assert_eq!(balance, to_yocto("8.5"));
    let out_come = call!(
        farmer2,
        farming.withdraw_reward(token2.valid_account_id(), Some(U128(to_yocto("1.5")))),
        deposit = 1
    );
    out_come.assert_success();
    assert_eq!(out_come.promise_errors().len(), 0);
    let reward = show_reward(&farming, farmer2.account_id(), token2.account_id(), false);
    assert_eq!(reward.0, to_yocto("0.5"));
    let balance = balance_of(&token2, farmer2.account_id());
    assert_eq!(balance, to_yocto("8"));

}