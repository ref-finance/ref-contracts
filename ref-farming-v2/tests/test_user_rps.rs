use near_sdk_sim::{call, init_simulator, to_yocto};
use near_sdk::json_types::{U128};

use ref_farming_v2::HRSimpleFarmTerms;

use crate::common::utils::*;
use crate::common::init::deploy_farming;
use crate::common::views::*;
use crate::common::actions::*;

mod common;

#[test]
fn test_user_rps() {
    generate_user_account!(root, owner, farmer1, farmer2);

    let (pool, token1, _) = prepair_pool_and_liquidity(
        &root, &owner, farming_id(), vec![&farmer1, &farmer2]);

    let farm_id1 = "swap@0#0".to_string();
    let farm_id2 = "swap@0#1".to_string();
    let seed_id = format!("{}@0", pool.account_id());
    let farming = deploy_farming(&root, farming_id(), owner.account_id());
    call!(farmer1, farming.storage_deposit(None, None), deposit = to_yocto("1")).assert_success();
    call!(farmer2, farming.storage_deposit(None, None), deposit = to_yocto("1")).assert_success();

    let current_timestamp = root.borrow_runtime().cur_block.block_timestamp;

    call!(
        owner,
        farming.create_simple_farm(HRSimpleFarmTerms{
            seed_id: seed_id.clone(),
            reward_token: to_va(token1.account_id()),
            start_at: to_sec(current_timestamp + to_nano(100)),
            reward_per_session: to_yocto("1").into(),
            session_interval: 60,
        }, None),
        deposit = to_yocto("1")
    ).assert_success();

    // deposit reward to farm1
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

    // farmer1 staking lpt 
    println!("----->> Farmer1 staking lpt.");
    call!(
        farmer1,
        pool.mft_transfer_call(":0".to_string(), to_va(farming_id()), to_yocto("1").into(), None, "".to_string()),
        deposit = 1
    ).assert_success();
    assert_eq!(get_user_rps(&farming, farmer1.account_id(),farm_id1.clone()).unwrap(), "0");

    //add farm2 after farmer1 staking 
    call!(
        owner,
        farming.create_simple_farm(HRSimpleFarmTerms{
            seed_id: seed_id.clone(),
            reward_token: to_va(token1.account_id()),
            start_at: to_sec(current_timestamp + to_nano(100)),
            reward_per_session: to_yocto("1").into(),
            session_interval: 60,
        }, None),
        deposit = to_yocto("1")
    ).assert_success();

    // deposit reward to farm2
    call!(
        root,
        token1.ft_transfer_call(to_va(farming_id()), U128(to_yocto("10")), None, generate_reward_msg(farm_id2.clone())),
        deposit = 1
    ).assert_success();

    assert!(get_user_rps(&farming, farmer1.account_id(), farm_id2.clone()).is_none());

    root.borrow_runtime_mut().cur_block.block_timestamp = current_timestamp + to_nano(60);

    call!(
        farmer1,
        farming.claim_reward_by_seed(seed_id.clone()),
        deposit = 0
    ).assert_success();
    assert_eq!(get_user_rps(&farming, farmer1.account_id(), farm_id1.clone()).unwrap(), "0");
    assert_eq!(get_user_rps(&farming, farmer1.account_id(), farm_id2.clone()).unwrap(), "0");

    root.borrow_runtime_mut().cur_block.block_timestamp = current_timestamp + to_nano(100 + 60);

    call!(
        farmer1,
        farming.claim_reward_by_seed(seed_id.clone()),
        deposit = 0
    ).assert_success();
    assert_eq!(get_user_rps(&farming, farmer1.account_id(), farm_id1.clone()).unwrap(), to_yocto("1").to_string());
    assert_eq!(get_user_rps(&farming, farmer1.account_id(), farm_id2.clone()).unwrap(), to_yocto("1").to_string());

    // farmer2 staking lpt 
    println!("----->> Farmer2 staking lpt.");
    call!(
        farmer2,
        pool.mft_transfer_call(":0".to_string(), to_va(farming_id()), to_yocto("1").into(), None, "".to_string()),
        deposit = 1
    ).assert_success();
    assert_eq!(get_user_rps(&farming, farmer2.account_id(),farm_id1.clone()).unwrap(), to_yocto("1").to_string());
    assert_eq!(get_user_rps(&farming, farmer2.account_id(),farm_id2.clone()).unwrap(), to_yocto("1").to_string());

    root.borrow_runtime_mut().cur_block.block_timestamp = current_timestamp + to_nano(100 + 60 * 2);

    call!(
        farmer1,
        farming.claim_reward_by_seed(seed_id.clone()),
        deposit = 0
    ).assert_success();
    assert_eq!(get_user_rps(&farming, farmer1.account_id(), farm_id1.clone()).unwrap(), to_yocto("1.5").to_string());
    assert_eq!(get_user_rps(&farming, farmer1.account_id(), farm_id2.clone()).unwrap(), to_yocto("1.5").to_string());
    call!(
        farmer2,
        farming.claim_reward_by_seed(seed_id.clone()),
        deposit = 0
    ).assert_success();
    assert_eq!(get_user_rps(&farming, farmer2.account_id(), farm_id1.clone()).unwrap(), to_yocto("1.5").to_string());
    assert_eq!(get_user_rps(&farming, farmer2.account_id(), farm_id2.clone()).unwrap(), to_yocto("1.5").to_string());


    //withdraw all seed 
    call!(
        farmer1,
        farming.withdraw_seed(seed_id.clone(), to_yocto("1").into()),
        deposit = 1
    ).assert_success();
    assert!(get_user_rps(&farming, farmer1.account_id(),farm_id1.clone()).is_none());
    assert!(get_user_rps(&farming, farmer1.account_id(),farm_id2.clone()).is_none());
    call!(
        farmer2,
        farming.withdraw_seed(seed_id.clone(), to_yocto("1").into()),
        deposit = 1
    ).assert_success();
    assert!(get_user_rps(&farming, farmer2.account_id(),farm_id1.clone()).is_none());
    assert!(get_user_rps(&farming, farmer2.account_id(),farm_id2.clone()).is_none());

    //staking in the same session_interval
    call!(
        farmer1,
        pool.mft_transfer_call(":0".to_string(), to_va(farming_id()), to_yocto("1").into(), None, "".to_string()),
        deposit = 1
    ).assert_success();
    assert_eq!(get_user_rps(&farming, farmer1.account_id(),farm_id1.clone()).unwrap(), to_yocto("1.5").to_string());
    assert_eq!(get_user_rps(&farming, farmer1.account_id(),farm_id2.clone()).unwrap(), to_yocto("1.5").to_string());

    call!(
        farmer2,
        pool.mft_transfer_call(":0".to_string(), to_va(farming_id()), to_yocto("1").into(), None, "".to_string()),
        deposit = 1
    ).assert_success();
    assert_eq!(get_user_rps(&farming, farmer2.account_id(),farm_id1.clone()).unwrap(), to_yocto("1.5").to_string());
    assert_eq!(get_user_rps(&farming, farmer2.account_id(),farm_id2.clone()).unwrap(), to_yocto("1.5").to_string());

    //withdraw all seed again
    call!(
        farmer1,
        farming.withdraw_seed(seed_id.clone(), to_yocto("1").into()),
        deposit = 1
    ).assert_success();
    call!(
        farmer2,
        farming.withdraw_seed(seed_id.clone(), to_yocto("1").into()),
        deposit = 1
    ).assert_success();

    root.borrow_runtime_mut().cur_block.block_timestamp = current_timestamp + to_nano(100 + 60 * 3);

    //staking in the next session_interval
    call!(
        farmer1,
        pool.mft_transfer_call(":0".to_string(), to_va(farming_id()), to_yocto("1").into(), None, "".to_string()),
        deposit = 1
    ).assert_success();
    assert_eq!(get_user_rps(&farming, farmer1.account_id(),farm_id1.clone()).unwrap(), "0");
    assert_eq!(get_user_rps(&farming, farmer1.account_id(),farm_id2.clone()).unwrap(), "0");

    call!(
        farmer2,
        pool.mft_transfer_call(":0".to_string(), to_va(farming_id()), to_yocto("1").into(), None, "".to_string()),
        deposit = 1
    ).assert_success();
    assert_eq!(get_user_rps(&farming, farmer2.account_id(),farm_id1.clone()).unwrap(), "0");
    assert_eq!(get_user_rps(&farming, farmer2.account_id(),farm_id2.clone()).unwrap(), "0");
}