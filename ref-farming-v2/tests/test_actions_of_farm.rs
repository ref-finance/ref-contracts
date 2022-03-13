use near_sdk_sim::{call, init_simulator, to_yocto};
use near_sdk::json_types::{U128};
use ref_farming_v2::{HRSimpleFarmTerms};

use crate::common::utils::*;
use crate::common::init::deploy_farming;
use crate::common::views::*;
use crate::common::actions::*;

mod common;

#[test]
fn test_create_simple_farm() {
    generate_user_account!(root, owner, farmer1);
    
    let (pool, token1, _) = prepair_pool_and_liquidity(
        &root, &owner, farming_id(), vec![]);

    let (farming, _) = prepair_multi_farms(&root, &owner, &token1, to_yocto("10"), 31);

    assert_eq!(show_farms_by_seed(&farming, format!("{}@0", swap()), false).len(), 31);

    assert_err!(call!(
        farmer1,
        farming.create_simple_farm(HRSimpleFarmTerms{
            seed_id: format!("{}@0", pool.account_id()),
            reward_token: token1.valid_account_id(),
            start_at: 0,
            reward_per_session: to_yocto("1").into(),
            session_interval: 60,
        }, None),
        deposit = to_yocto("1")
    ), "ERR_NOT_ALLOWED");

    assert_err!(call!(
        owner,
        farming.create_simple_farm(HRSimpleFarmTerms{
            seed_id: format!("{}@0@3", pool.account_id()),
            reward_token: token1.valid_account_id(),
            start_at: 0,
            reward_per_session: to_yocto("1").into(),
            session_interval: 60,
        }, None),
        deposit = to_yocto("1")
    ), "E33: invalid seed id");

    call!(
        owner,
        farming.extend_operators(vec![farmer1.valid_account_id()]),
        deposit = 1
    ).assert_success();

    call!(
        farmer1,
        farming.create_simple_farm(HRSimpleFarmTerms{
            seed_id: format!("{}@0", pool.account_id()),
            reward_token: token1.valid_account_id(),
            start_at: 0,
            reward_per_session: to_yocto("1").into(),
            session_interval: 60,
        }, None),
        deposit = to_yocto("1")
    ).assert_success();

    assert_eq!(show_farms_by_seed(&farming, format!("{}@0", swap()), false).len(), 32);

    assert_err!(call!(
        owner,
        farming.create_simple_farm(HRSimpleFarmTerms{
            seed_id: format!("{}@0", pool.account_id()),
            reward_token: token1.valid_account_id(),
            start_at: 0,
            reward_per_session: to_yocto("1").into(),
            session_interval: 60,
        }, None),
        deposit = to_yocto("1")
    ), "E36: the number of farms has reached its limit");
}


#[test]
fn test_force_clean_farm() {
    generate_user_account!(root, owner, farmer1);
    
    let (pool, token1, _) = prepair_pool_and_liquidity(
        &root, &owner, farming_id(), vec![&farmer1]);

    let farming = deploy_farming(&root, farming_id(), owner.account_id());

    let farm_id = format!("{}@0#0", pool.account_id());
    let seed_id = format!("{}@0", pool.account_id());
    call!(
        owner,
        farming.create_simple_farm(HRSimpleFarmTerms{
            seed_id: seed_id.clone(),
            reward_token: token1.valid_account_id(),
            start_at: 0,
            reward_per_session: to_yocto("1").into(),
            session_interval: 60,
        }, None),
        deposit = to_yocto("1")
    ).assert_success();

    assert_err!(call!(
        farmer1,
        farming.force_clean_farm(farm_id.clone())
    ), "ERR_NOT_ALLOWED");

    assert_err!(call!(
        owner,
        farming.force_clean_farm("random".to_string())
    ), "E41: farm not exist");

    assert_err!(call!(
        owner,
        farming.force_clean_farm(farm_id.clone())
    ), "Farm can NOT be removed now");

    //Fast forward to DEFAULT_FARM_EXPIRE_SEC without any reward
    root.borrow_runtime_mut().cur_block.block_timestamp += to_nano(3600 * 24 * 30);
    assert_err!(call!(
        owner,
        farming.force_clean_farm(format!("{}@0#0", pool.account_id()))
    ), "Farm can NOT be removed now");

    //add reward
    mint_token(&token1, &root, to_yocto("10"));
    call!(
        root,
        token1.storage_deposit(Some(to_va(farming_id())), None),
        deposit = to_yocto("1")
    )
    .assert_success();
    call!(
        root,
        token1.ft_transfer_call(to_va(farming_id()), U128(to_yocto("10")), None, generate_reward_msg(farm_id.clone())),
        deposit = 1
    )
    .assert_success();
    
    //The rewards have been handed out, but farm not expire
    root.borrow_runtime_mut().cur_block.block_timestamp += to_nano(60 * 11);
    assert_err!(call!(
        owner,
        farming.force_clean_farm(farm_id.clone())
    ), "Farm can NOT be removed now");

    root.borrow_runtime_mut().cur_block.block_timestamp += to_nano(3600 * 24 * 30);

    assert_eq!(show_farms_by_seed(&farming, seed_id.clone(), false).len(), 1);
    assert_eq!(show_outdated_farms(&farming, false).len(), 0);
    call!(
        owner,
        farming.extend_operators(vec![farmer1.valid_account_id()]),
        deposit = 1
    ).assert_success();
    call!(
        owner,
        farming.force_clean_farm(farm_id.clone())
    ).assert_success();
    assert_eq!(show_farms_by_seed(&farming, seed_id.clone(), false).len(), 0);
    assert_eq!(show_outdated_farms(&farming, false).len(), 1);
}


#[test]
fn test_cancel_farm() {
    generate_user_account!(root, owner, farmer1);
    
    let (pool, token1, _) = prepair_pool_and_liquidity(
        &root, &owner, farming_id(), vec![&farmer1]);

    let farming = deploy_farming(&root, farming_id(), owner.account_id());

    let farm_id = format!("{}@0#0", pool.account_id());
    let seed_id = format!("{}@0", pool.account_id());
    call!(
        owner,
        farming.create_simple_farm(HRSimpleFarmTerms{
            seed_id: seed_id.clone(),
            reward_token: token1.valid_account_id(),
            start_at: 0,
            reward_per_session: to_yocto("1").into(),
            session_interval: 60,
        }, None),
        deposit = to_yocto("1")
    ).assert_success();

    assert_err!(call!(
        farmer1,
        farming.cancel_farm(farm_id.clone())
    ), "ERR_NOT_ALLOWED");

    assert_err!(call!(
        owner,
        farming.cancel_farm("random".to_string())
    ), "E41: farm not exist");

    //add reward
    mint_token(&token1, &root, to_yocto("10"));
    call!(
        root,
        token1.storage_deposit(Some(to_va(farming_id())), None),
        deposit = to_yocto("1")
    )
    .assert_success();
    call!(
        root,
        token1.ft_transfer_call(to_va(farming_id()), U128(to_yocto("10")), None, generate_reward_msg(farm_id.clone())),
        deposit = 1
    )
    .assert_success();

    //The rewards have been handed out, but farm not expire
    root.borrow_runtime_mut().cur_block.block_timestamp += to_nano(60 * 11);
    assert_err!(call!(
        owner,
        farming.cancel_farm(farm_id.clone())
    ), "This farm can NOT be cancelled");

    root.borrow_runtime_mut().cur_block.block_timestamp += to_nano(3600 * 24 * 30);
    assert_err!(call!(
        owner,
        farming.cancel_farm(farm_id.clone())
    ), "This farm can NOT be cancelled");

    call!(
        owner,
        farming.create_simple_farm(HRSimpleFarmTerms{
            seed_id: seed_id.clone(),
            reward_token: token1.valid_account_id(),
            start_at: 0,
            reward_per_session: to_yocto("1").into(),
            session_interval: 60,
        }, None),
        deposit = to_yocto("1")
    ).assert_success();

    assert_eq!(show_farms_by_seed(&farming, seed_id.clone(), false).len(), 2);
    assert_eq!(show_outdated_farms(&farming, false).len(), 0);
    call!(
        owner,
        farming.extend_operators(vec![farmer1.valid_account_id()]),
        deposit = 1
    ).assert_success();
    call!(
        farmer1,
        farming.cancel_farm(format!("{}@0#1", pool.account_id()))
    ).assert_success();
    assert_eq!(show_farms_by_seed(&farming, seed_id.clone(), false).len(), 1);
    assert_eq!(show_outdated_farms(&farming, false).len(), 0);
}

