use near_sdk_sim::{call, init_simulator, to_yocto, view};
use near_sdk::json_types::{U128};
use near_sdk::serde_json::Value;

use ref_farming::{HRSimpleFarmTerms};

use crate::common::utils::*;
use crate::common::init::{deploy_farming, deploy_token};
use crate::common::views::*;
use crate::common::actions::*;

mod common;

#[test]
/// reward: 10.pow(33), seed: 10.pow(0)
/// rps: 10.pow(57)
fn seed_amount_little() {
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
            reward_per_session: to_yocto("1000000000").into(),
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
    mint_token(&token1, &root, to_yocto("10000000000"));
    call!(
        root,
        token1.ft_transfer_call(to_va(farming_id()), U128(to_yocto("10000000000")), None, farm_id.clone()),
        deposit = 1
    )
    .assert_success();
    let farm_info = show_farminfo(&farming, farm_id.clone(), false);

    let out_come = call!(
        farmer1,
        pool.mft_transfer_call(":0".to_string(), to_va(farming_id()), to_yocto("1").into(), None, "".to_string()),
        deposit = 1
    );
    out_come.assert_success();
    let out_come = call!(
        farmer1,
        farming.withdraw_seed(farm_info.seed_id.clone(), to_yocto("0.999999999999999999999999").into()),
        deposit = 1
    );
    out_come.assert_success();

    assert!(root.borrow_runtime_mut().produce_blocks(60).is_ok());
    let farm_info = show_farminfo(&farming, farm_id.clone(), false);
    assert_farming(&farm_info, "Running".to_string(), to_yocto("10000000000"), 1, 0, 0, to_yocto("1000000000"), 0);
    let user_seeds = show_userseeds(&farming, farmer1.account_id(), false);
    assert_eq!(user_seeds.get(&String::from("swap@0")).unwrap().0, 1);
    let unclaim = show_unclaim(&farming, farmer1.account_id(), farm_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("1000000000"));

    assert!(root.borrow_runtime_mut().produce_blocks(60).is_ok());
    let farm_info = show_farminfo(&farming, farm_id.clone(), false);
    assert_farming(&farm_info, "Running".to_string(), to_yocto("10000000000"), 2, 0, to_yocto("0"), to_yocto("2000000000"), to_yocto("0"));
    let unclaim = show_unclaim(&farming, farmer1.account_id(), farm_id.clone(), false);
    assert_eq!(unclaim.0, to_yocto("2000000000"));
    let out_come = call!(
        farmer1,
        farming.claim_reward_by_farm(farm_id.clone()),
        deposit = 0
    );
    out_come.assert_success();
    // println!("{:#?}", out_come.promise_results());
    let farm_info = show_farminfo(&farming, farm_id.clone(), false);
    assert_farming(&farm_info, "Running".to_string(), to_yocto("10000000000"), 2, 2, to_yocto("2000000000"), to_yocto("0"), to_yocto("0"));
    let unclaim = show_unclaim(&farming, farmer1.account_id(), farm_id.clone(), false);
    assert_eq!(unclaim.0, 0_u128);
    let reward = show_reward(&farming, farmer1.account_id(), token1.account_id(), false);
    assert_eq!(reward.0, to_yocto("2000000000"));
    let user_rps = view!(farming.get_user_rps(farmer1.valid_account_id(), farm_id)).unwrap_json::<String>();
    assert_eq!(user_rps, String::from("2000000000000000000000000000000000000000000000000000000000"));
}

#[test]
/// reward 10.pow(17), seed: 10.pow(38) 
/// rps 10.pow(3) 
fn seed_amount_huge() {
    // println!("{}", u128::MAX);
    // 340282366920938.463463374607431768211455

    let root = init_simulator(None);

    let owner = root.create_user("owner".to_string(), to_yocto("100"));
    let farmer1 = root.create_user("farmer1".to_string(), to_yocto("100"));

    let (pool, token1, token2) = prepair_pool(&root, &owner);
    call!(root, pool.mft_register(":0".to_string(), to_va(farming_id())), deposit = to_yocto("1"))
    .assert_success();

    mint_token(&token1, &farmer1, u128::MAX);
    mint_token(&token2, &farmer1, u128::MAX);
    call!(
        farmer1,
        pool.storage_deposit(None, None),
        deposit = to_yocto("1")
    )
    .assert_success();
    call!(
        farmer1,
        token1.ft_transfer_call(to_va(swap()), U128(to_yocto("1")), None, "".to_string()),
        deposit = 1
    )
    .assert_success();
    call!(
        farmer1,
        token2.ft_transfer_call(to_va(swap()), U128(to_yocto("1")), None, "".to_string()),
        deposit = 1
    )
    .assert_success();
    call!(
        farmer1,
        pool.add_liquidity(0, vec![U128(to_yocto("1")), U128(to_yocto("1"))], None),
        deposit = to_yocto("0.01")
    )
    .assert_success();

    call!(
        farmer1,
        token1.ft_transfer_call(to_va(swap()), U128(to_yocto("340282366920937")), None, "".to_string()),
        deposit = 1
    )
    .assert_success();
    call!(
        farmer1,
        token2.ft_transfer_call(to_va(swap()), U128(to_yocto("340282366920937")), None, "".to_string()),
        deposit = 1
    )
    .assert_success();
    call!(
        farmer1,
        pool.add_liquidity(0, vec![U128(to_yocto("340282366920937")), U128(to_yocto("340282366920937"))], None),
        deposit = to_yocto("0.01")
    )
    .assert_success();


    let farming = deploy_farming(&root, farming_id(), owner.account_id());
    call!(farmer1, farming.storage_deposit(None, None), deposit = to_yocto("1")).assert_success();

    let token3 = deploy_token(&root, String::from("rft"), vec![farming_id()]);
    mint_token(&token3, &farmer1, to_yocto("10"));

    let farm_id = "swap@0#0".to_string();
    let single_reward = 100000000000000000_u128;
    let out_come = call!(
        owner,
        farming.create_simple_farm(HRSimpleFarmTerms{
            seed_id: format!("{}@0", pool.account_id()),
            reward_token: token3.valid_account_id(),
            start_at: 0,
            reward_per_session: U128(single_reward),
            session_interval: 60,
        }, None),
        deposit = to_yocto("1")
    );
    out_come.assert_success();

    call!(
        farmer1,
        token3.ft_transfer_call(to_va(farming_id()), U128(to_yocto("10")), None, farm_id.clone()),
        deposit = 1
    )
    .assert_success();
    let farm_info = show_farminfo(&farming, farm_id.clone(), false);

    let out_come = call!(
        farmer1,
        pool.mft_transfer_call(":0".to_string(), to_va(farming_id()), U128(to_yocto("340282366920938")), None, "".to_string()),
        deposit = 1
    );
    out_come.assert_success();
    

    assert!(root.borrow_runtime_mut().produce_blocks(60).is_ok());
    let farm_info = show_farminfo(&farming, farm_id.clone(), false);
    assert_farming(&farm_info, "Running".to_string(), to_yocto("10"), 1, 0, 0, single_reward, 0);
    let user_seeds = show_userseeds(&farming, farmer1.account_id(), false);
    assert_eq!(user_seeds.get(&String::from("swap@0")).unwrap().0, to_yocto("340282366920938"));
    let unclaim = show_unclaim(&farming, farmer1.account_id(), farm_id.clone(), false);
    // println!("unclaim.0 {}", unclaim.0);
    assert!(unclaim.0 > 99000000000000000);

    assert!(root.borrow_runtime_mut().produce_blocks(60).is_ok());
    let farm_info = show_farminfo(&farming, farm_id.clone(), false);
    assert_farming(&farm_info, "Running".to_string(), to_yocto("10"), 2, 0, to_yocto("0"), 2 * single_reward, to_yocto("0"));
    let unclaim = show_unclaim(&farming, farmer1.account_id(), farm_id.clone(), false);
    // println!("unclaim.0 {}", unclaim.0);
    assert!(unclaim.0 > 199000000000000000);
    let out_come = call!(
        farmer1,
        farming.claim_reward_by_farm(farm_id.clone()),
        deposit = 0
    );
    out_come.assert_success();
    // println!("{:#?}", out_come.promise_results());
    // show_farminfo(&farming, farm_id.clone(), false);
    // assert_farming(&farm_info, "Running".to_string(), to_yocto("10"), 2, 2, to_yocto("2"), to_yocto("0"), to_yocto("0"));
    let unclaim = show_unclaim(&farming, farmer1.account_id(), farm_id.clone(), false);
    assert_eq!(unclaim.0, 0_u128);
    let reward = show_reward(&farming, farmer1.account_id(), token3.account_id(), false);
    // println!("reward.0 {}", reward.0);
    assert!(reward.0 > 199000000000000000);
    let user_rps = view!(farming.get_user_rps(farmer1.valid_account_id(), farm_id)).unwrap_json::<String>();
    // println!("user_rps: {}", user_rps);
    assert_eq!(user_rps, String::from("587"));
}

