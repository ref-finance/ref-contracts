use near_sdk_sim::{call, init_simulator, to_yocto};

use crate::common::utils::*;
use crate::common::init::deploy_farming;


mod common;

#[test]
fn storage_stake() {
    generate_user_account!(root, owner, farmer1, farmer2);

    let farming = deploy_farming(&root, farming_id(), owner.account_id());

    // farmer1 register
    assert_err!(call!(farmer1, farming.storage_deposit(None, None), deposit = to_yocto("0.01")),
        "E11: insufficient $NEAR storage deposit");

    let orig_user_balance = farmer1.account().unwrap().amount;
    call!(farmer1, farming.storage_deposit(None, None), deposit = to_yocto("0.1")).assert_success();
    assert!(orig_user_balance - farmer1.account().unwrap().amount > to_yocto("0.1"));
    assert!(orig_user_balance - farmer1.account().unwrap().amount < to_yocto("0.11"));

    // farmer1 repeat register
    let orig_user_balance = farmer1.account().unwrap().amount;
    call!(farmer1, farming.storage_deposit(None, None), deposit = to_yocto("0.1")).assert_success();
    assert!(orig_user_balance - farmer1.account().unwrap().amount < to_yocto("0.001"));

    // farmer1 withdraw storage
    let out_come = call!(farmer1, farming.storage_withdraw(None), deposit = 1);
    let ex_status = format!("{:?}", out_come.promise_errors()[0].as_ref().unwrap().status());
    assert!(ex_status.contains("E14: no storage can withdraw"));

    // farmer1 unregister storage
    let orig_user_balance = farmer1.account().unwrap().amount;
    call!(farmer1, farming.storage_unregister(None), deposit = 1).assert_success();
    assert!(farmer1.account().unwrap().amount - orig_user_balance > to_yocto("0.09"));
    assert!(farmer1.account().unwrap().amount - orig_user_balance < to_yocto("0.1"));

    // farmer1 help farmer2 register
    let orig_user_balance = farmer1.account().unwrap().amount;
    let orig_user_balance_famer2 = farmer2.account().unwrap().amount;
    let out_come = call!(farmer1, farming.storage_deposit(Some(to_va(farmer2.account_id())), None), deposit = to_yocto("1"));
    out_come.assert_success();
    assert!(orig_user_balance - farmer1.account().unwrap().amount > to_yocto("0.1"));
    assert!(orig_user_balance - farmer1.account().unwrap().amount < to_yocto("0.11"));
    assert!(orig_user_balance_famer2 - farmer2.account().unwrap().amount == 0);
}