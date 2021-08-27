use near_sdk_sim::{view, call, init_simulator, to_yocto};
// use near_sdk::json_types::{U128};
use near_sdk::serde_json::Value;
use ref_farming::{HRSimpleFarmTerms};

use crate::common::utils::*;
use crate::common::init::deploy_farming;
use crate::common::views::*;
use crate::common::actions::*;

mod common;

#[test]
fn storage_stake() {
    let root = init_simulator(None);

    let owner = root.create_user("owner".to_string(), to_yocto("100"));
    let farmer1 = root.create_user("farmer1".to_string(), to_yocto("100"));
    let farmer2 = root.create_user("farmer2".to_string(), to_yocto("100"));

    let farming = deploy_farming(&root, farming_id(), owner.account_id());

    // farmer1 register with only_register set to false
    let out_come = call!(farmer1, farming.storage_deposit(None, None), deposit = to_yocto("1"));
    out_come.assert_success();
    let sb = out_come.unwrap_json::<StorageBalance>();
    assert_eq!(sb.total.0, to_yocto("1"));
    assert_eq!(sb.available.0, to_yocto("0.99908"));
    assert!(farmer1.account().unwrap().amount < to_yocto("99"));

    // farmer1 withdraw storage
    let out_come = call!(farmer1, farming.storage_withdraw(None), deposit = 1);
    out_come.assert_success();
    let sb = out_come.unwrap_json::<StorageBalance>();
    assert_eq!(sb.total.0, to_yocto("0.00092"));
    assert_eq!(sb.available.0, to_yocto("0"));
    assert!(farmer1.account().unwrap().amount > to_yocto("99.9"));

    // farmer1 unregister storage
    let out_come = call!(farmer1, farming.storage_unregister(None), deposit = 1);
    out_come.assert_success();
    let ret = out_come.unwrap_json_value();
    assert_eq!(ret, Value::Bool(true));
    let ret = view!(farming.storage_balance_of(farmer1.valid_account_id())).unwrap_json_value();
    assert_eq!(ret, Value::Null);
    assert!(farmer1.account().unwrap().amount > to_yocto("99.99"));

    // farmer1 register with only_register set to true
    let out_come = call!(farmer1, farming.storage_deposit(None, Some(true)), deposit = to_yocto("1"));
    out_come.assert_success();
    let sb = out_come.unwrap_json::<StorageBalance>();
    // println!("{:#?}", sb);
    assert_eq!(sb.total.0, to_yocto("0.01852"));
    assert_eq!(sb.available.0, to_yocto("0.01760"));

    // farmer1 help farmer2 register with only_register set to false
    let out_come = call!(farmer1, farming.storage_deposit(Some(to_va(farmer2.account_id())), Some(false)), deposit = to_yocto("1"));
    out_come.assert_success();
    let sb = out_come.unwrap_json::<StorageBalance>();
    assert_eq!(sb.total.0, to_yocto("1"));
    assert_eq!(sb.available.0, to_yocto("0.99908"));
    let sb = show_storage_balance(&farming, farmer2.account_id(), false);
    assert_eq!(sb.total.0, to_yocto("1"));
    assert_eq!(sb.available.0, to_yocto("0.99908"));
    let sb = show_storage_balance(&farming, farmer1.account_id(), false);
    assert_eq!(sb.total.0, to_yocto("0.01852"));
    assert_eq!(sb.available.0, to_yocto("0.01760"));
    assert!(farmer1.account().unwrap().amount < to_yocto("99"));
    assert_eq!(farmer2.account().unwrap().amount, to_yocto("100"));
    
    // insurfficent storage and deposit more
    let out_come = call!(farmer1, farming.storage_withdraw(None), deposit = 1);
    out_come.assert_success();
    let sb = out_come.unwrap_json::<StorageBalance>();
    assert_eq!(sb.total.0, to_yocto("0.00092"));
    assert_eq!(sb.available.0, to_yocto("0"));

    let (pool, token1, _) = prepair_pool_and_liquidity(&root, &owner, farming_id(), vec![&farmer1]);
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

    let out_come = call!(farmer1, farming.storage_deposit(None, Some(false)), deposit = to_yocto("1"));
    out_come.assert_success();
    let sb = out_come.unwrap_json::<StorageBalance>();
    assert_eq!(sb.total.0, to_yocto("1.00092"));
    assert_eq!(sb.available.0, to_yocto("1"));

    let out_come = call!(
        farmer1,
        pool.mft_transfer_call(":0".to_string(), to_va(farming_id()), to_yocto("0.5").into(), None, "".to_string()),
        deposit = 1
    );
    out_come.assert_success();
    let user_seeds = show_userseeds(&farming, farmer1.account_id(), false);
    assert_eq!(user_seeds.get(&String::from("swap@0")).unwrap().0, to_yocto("0.5"));
}