use near_sdk_sim::{call, init_simulator, to_yocto};
use near_sdk::serde_json::Value;

use ref_farming::{HRSimpleFarmTerms};

use crate::common::utils::*;
use crate::common::init::deploy_farming;
use crate::common::views::*;
use crate::common::actions::*;

mod common;

#[test]
fn test_min_deposit() {
    let root = init_simulator(None);

    println!("----->> Prepare accounts.");
    let owner = root.create_user("owner".to_string(), to_yocto("100"));
    let farmer = root.create_user("farmer1".to_string(), to_yocto("100"));
    println!("<<----- owner and 1 farmers prepared.");

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

    let seeds_info = show_seedsinfo(&farming, true);
    assert_eq!(seeds_info.get(&seed_id).unwrap().min_deposit.0, to_yocto("0.000001"));

    let out_come = call!(
        owner,
        farming.modify_seed_min_deposit(seed_id.clone(), to_yocto("2").into())
    );
    out_come.assert_success();

    let seeds_info = show_seedsinfo(&farming, true);
    assert_eq!(seeds_info.get(&seed_id).unwrap().min_deposit.0, to_yocto("2"));

    let out_come = call!(
        farmer,
        pool.mft_transfer_call(":0".to_string(), to_va(farming_id()), to_yocto("1").into(), None, "".to_string()),
        deposit = 1
    );
    assert!(format!("{:?}", out_come.promise_errors()[0].as_ref().unwrap().status()).contains("E34: below min_deposit of this seed"));
}