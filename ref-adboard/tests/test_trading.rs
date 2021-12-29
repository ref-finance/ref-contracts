use near_sdk_sim::{call, init_simulator, to_yocto, view};
use near_sdk::json_types::{U128};
use near_sdk_sim::transaction::ExecutionStatus;
use crate::common::*;

mod common;



#[test]
fn test_trading() {
    let root = init_simulator(None);

    // prepair users
    let owner = root.create_user("owner".to_string(), to_yocto("100"));
    let alice = root.create_user("alice".to_string(), to_yocto("100"));
    let bob = root.create_user("bob".to_string(), to_yocto("100"));
    println!("----->> owner lp and user accounts prepaired.");

    // prepair pool and tokens
    let(pool, token1, token2) = prepair_pool(&root, &owner);
    println!("----->> The pool prepaired.");

    // deposit dai and eth to swap
    swap_deposit(&alice, &pool, &token1, &token2);
    assert_eq!(
        view!(pool.get_deposit(to_va(alice.account_id.clone()), to_va(dai())))
            .unwrap_json::<U128>()
            .0,
        to_yocto("100")
    );
    assert_eq!(
        view!(pool.get_deposit(to_va(alice.account_id.clone()), to_va(eth())))
            .unwrap_json::<U128>()
            .0,
        to_yocto("100")
    );
    println!("----->> token deposited to swap by alice.");
    swap_deposit(&bob, &pool, &token1, &token2);
    assert_eq!(
        view!(pool.get_deposit(to_va(bob.account_id.clone()), to_va(dai())))
            .unwrap_json::<U128>()
            .0,
        to_yocto("100")
    );
    assert_eq!(
        view!(pool.get_deposit(to_va(bob.account_id.clone()), to_va(eth())))
            .unwrap_json::<U128>()
            .0,
        to_yocto("100")
    );
    println!("----->> token deposited to swap by bob.");

    // create adboard
    let adboard = deploy_adboard(&root, adboard_id(), owner.account_id());
    println!("Deploying adboard ... OK.");
    // register to swap
    call!(
        root,
        pool.storage_deposit(Some(to_va(adboard_id())), None),
        deposit = to_yocto("1")
    )
    .assert_success();
    println!("Registering adboard to swap ... OK.");
    println!("----->> Adboard is ready.");

    // add_token_to_whitelist
    call!(
        owner,
        adboard.add_token_to_whitelist(to_va(dai())),
        deposit = 0
    ).assert_success();
    call!(
        owner,
        adboard.add_token_to_whitelist(to_va(eth())),
        deposit = 0
    ).assert_success();
    let token_whitelist = view!(adboard.get_whitelist()).unwrap_json::<Vec<String>>();
    assert_eq!(token_whitelist.len(), 2);
    println!("----->> Adboard token whitelisted.");

    //********************
    // start the real test
    //********************

    // Alice buy frame0 using dai and new sell price 1.5, 
    // but payment would fail and recorded into failed_payment,
    // cause owner of frame dosn't register storage in pool
    chain_move_and_show(&root, 0); 
    let out_come = call!(
        alice,
        pool.mft_transfer_call(
            dai(), to_va(adboard_id()), to_yocto("1").into(), 
            None, "0||dai||1500000000000000000000000||0".to_string()),
        deposit = 1
    );
    out_come.assert_success();
    let ret = get_user_token(&pool, owner.account_id(), dai());
    assert_eq!(ret.0, 0);
    let ret = get_failed_payment(&adboard);
    assert_eq!(ret.len(), 1);  // cause owner haven't register storage in pool
    let ret = get_frame_metadata(&adboard, 0);
    println!("{:?}", ret.unwrap());
    println!("----->> Alice buy frame0 using dai and new sell price 1.5, but payment failed as expected.");

    // Bob buy frame0 using eth and new sell price 2, 
    // but the frame is in pretection, so would fail,
    chain_move_and_show(&root, 0);  // 101_000_000_000
    let out_come = call!(
        bob,
        pool.mft_transfer_call(
            dai(), to_va(adboard_id()), to_yocto("1.5").into(), 
            None, "0||eth||2000000000000000000000000||0".to_string()),
        deposit = 1
    );
    // println!("{:#?}", out_come.promise_results());
    assert_eq!(out_come.promise_errors().len(), 1);
    if let ExecutionStatus::Failure(execution_error) = 
        &out_come.promise_errors().remove(0).unwrap().outcome().status {
            // println!("{}", execution_error);
            assert!(execution_error.to_string().contains("Frame is currently protected"));
        } else {
            unreachable!();
        }
    println!("----->> Bob buy frame0 failed as expected.");

    // Bob buy frame0 using eth and new sell price 2
    let ret = get_user_token(&pool, alice.account_id(), dai());
    let alice_balance = ret.0;
    let ret = get_frame_metadata(&adboard, 0);
    println!("{:?}", ret.unwrap());
    chain_move_and_show(&root, 60);
    let out_come = call!(
        bob,
        pool.mft_transfer_call(
            dai(), to_va(adboard_id()), to_yocto("1.5").into(), 
            None, "0||eth||2000000000000000000000000||0".to_string()),
        deposit = 1
    );
    out_come.assert_success();
    let ret = get_user_token(&pool, alice.account_id(), dai());
    assert_eq!(ret.0 - alice_balance, to_yocto("1.485"));
    let ret = get_frame_metadata(&adboard, 0);
    println!("{:?}", ret.unwrap());
    println!("----->> Bob buy frame0 succeeded.");

    // repay failed_payment
    // register to swap
    call!(
        root,
        pool.storage_deposit(Some(to_va(owner.account_id())), None),
        deposit = to_yocto("1")
    )
    .assert_success();
    let out_come = call!(
        owner,
        adboard.repay_failure_payment(),
        deposit = 0
    );
    out_come.assert_success();
    let ret = get_user_token(&pool, owner.account_id(), dai());
    assert_eq!(ret.0, to_yocto("0.99"));
    let ret = get_failed_payment(&adboard);
    assert_eq!(ret.len(), 0);
    println!("----->> Repay failed_payment succeeded.");
}