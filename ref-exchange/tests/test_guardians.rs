use near_sdk::json_types::{U128};
use near_sdk_sim::{call, to_yocto};

use ref_exchange::{RunningState, SwapAction};
use crate::common::utils::*;
pub mod common;

#[test]
fn guardians_scenario_01() {
    let (root, owner, pool, token1, _, _) = setup_pool_with_liquidity();
    let guard1 = root.create_user("guard1".to_string(), to_yocto("100"));
    let guard2 = root.create_user("guard2".to_string(), to_yocto("100"));
    let new_user = root.create_user("new_user".to_string(), to_yocto("100"));

    println!("Guardians Case 0101: only owner can add guardians");
    let out_come = call!(
        root,
        pool.extend_guardians(vec![guard1.valid_account_id(), guard2.valid_account_id()])
    );
    assert!(!out_come.is_ok());
    assert_eq!(get_error_count(&out_come), 1);
    assert!(get_error_status(&out_come).contains("ERR_NOT_ALLOWED"));
    let metadata = get_metadata(&pool);
    assert_eq!(metadata.guardians.len(), 0);

    let out_come = call!(
        owner,
        pool.extend_guardians(vec![guard1.valid_account_id(), guard2.valid_account_id()])
    );
    out_come.assert_success();
    assert_eq!(get_error_count(&out_come), 0);
    let metadata = get_metadata(&pool);
    assert_eq!(metadata.guardians.len(), 2);
    assert_eq!(metadata.guardians.get(0).unwrap().clone(), guard1.account_id());
    assert_eq!(metadata.guardians.get(1).unwrap().clone(), guard2.account_id());

    println!("Guardians Case 0102: only owner and guardians can manage global whitelists");
    let out_come = call!(
        root,
        pool.remove_whitelisted_tokens(vec![to_va(eth()), to_va(dai())])
    );
    assert!(!out_come.is_ok());
    assert_eq!(get_error_count(&out_come), 1);
    assert!(get_error_status(&out_come).contains("ERR_NOT_ALLOWED"));
    let wl = get_whitelist(&pool);
    assert_eq!(wl.len(), 3);
    assert_eq!(wl.get(0).unwrap().clone(), dai());
    assert_eq!(wl.get(1).unwrap().clone(), eth());
    assert_eq!(wl.get(2).unwrap().clone(), usdt());

    let out_come = call!(
        owner,
        pool.remove_whitelisted_tokens(vec![to_va(usdt()), to_va(eth()), to_va(dai())])
    );
    out_come.assert_success();
    assert_eq!(get_error_count(&out_come), 0);
    let wl = get_whitelist(&pool);
    assert_eq!(wl.len(), 0);

    let out_come = call!(
        owner,
        pool.extend_whitelisted_tokens(vec![to_va(dai())])
    );
    out_come.assert_success();
    assert_eq!(get_error_count(&out_come), 0);
    let wl = get_whitelist(&pool);
    assert_eq!(wl.len(), 1);
    assert_eq!(wl.get(0).unwrap().clone(), dai());

    let out_come = call!(
        guard1,
        pool.extend_whitelisted_tokens(vec![to_va(eth())])
    );
    out_come.assert_success();
    assert_eq!(get_error_count(&out_come), 0);
    let wl = get_whitelist(&pool);
    assert_eq!(wl.len(), 2);
    assert_eq!(wl.get(0).unwrap().clone(), dai());
    assert_eq!(wl.get(1).unwrap().clone(), eth());

    let out_come = call!(
        guard2,
        pool.extend_whitelisted_tokens(vec![to_va(usdt())])
    );
    out_come.assert_success();
    assert_eq!(get_error_count(&out_come), 0);
    let wl = get_whitelist(&pool);
    assert_eq!(wl.len(), 3);
    assert_eq!(wl.get(0).unwrap().clone(), dai());
    assert_eq!(wl.get(1).unwrap().clone(), eth());
    assert_eq!(wl.get(2).unwrap().clone(), usdt());

    println!("Guardians Case 0103: only owner and guardians can pause the contract");
    let out_come = call!(
        root,
        pool.change_state(RunningState::Paused),
        deposit = 1
    );
    assert!(!out_come.is_ok());
    assert_eq!(get_error_count(&out_come), 1);
    assert!(get_error_status(&out_come).contains("ERR_NOT_ALLOWED"));
    let metadata = get_metadata(&pool);
    assert_eq!(metadata.state, RunningState::Running);

    let out_come = call!(
        guard1,
        pool.change_state(RunningState::Paused),
        deposit = 1
    );
    out_come.assert_success();
    assert_eq!(get_error_count(&out_come), 0);
    let metadata = get_metadata(&pool);
    assert_eq!(metadata.state, RunningState::Paused);

    // register user would fail
    let out_come = call!(
        new_user,
        pool.storage_deposit(None, None),
        deposit = to_yocto("1")
    );
    assert!(!out_come.is_ok());
    assert_eq!(get_error_count(&out_come), 1);
    assert!(get_error_status(&out_come).contains("E51: contract paused"));

    // add pool would fail
    let out_come = call!(
        root,
        pool.add_simple_pool(vec![to_va(dai()), to_va(eth())], 25),
        deposit = to_yocto("1")
    );
    assert!(!out_come.is_ok());
    assert_eq!(get_error_count(&out_come), 1);
    assert!(get_error_status(&out_come).contains("E51: contract paused"));

    // deposit token would fail
    let out_come = call!(
        root,
        token1.ft_transfer_call(to_va(swap()), to_yocto("5").into(), None, "".to_string()),
        deposit = 1
    );
    out_come.assert_success();
    assert_eq!(get_error_count(&out_come), 1);
    assert!(get_error_status(&out_come).contains("E51: contract paused"));

    // add liqudity would fail
    let out_come = call!(
        root,
        pool.add_liquidity(0, vec![U128(to_yocto("10")), U128(to_yocto("20"))], None),
        deposit = to_yocto("0.0007")
    );
    assert!(!out_come.is_ok());
    assert_eq!(get_error_count(&out_come), 1);
    assert!(get_error_status(&out_come).contains("E51: contract paused"));

    // swap would fail
    let out_come = call!(
        root,
        pool.swap(
            vec![SwapAction {
                pool_id: 0,
                token_in: dai(),
                amount_in: Some(U128(to_yocto("1"))),
                token_out: eth(),
                min_amount_out: U128(1)
            }],
            None
        ),
        deposit = 1
    );
    assert!(!out_come.is_ok());
    assert_eq!(get_error_count(&out_come), 1);
    assert!(get_error_status(&out_come).contains("E51: contract paused"));

    // // instant swap would fail
    // call!(
    //     new_user,
    //     token2.mint(to_va(new_user.account_id.clone()), U128(to_yocto("10")))
    // )
    // .assert_success();
    // let msg = format!(
    //     "{{\"pool_id\": {}, \"token_in\": \"{}\", \"token_out\": \"{}\", \"min_amount_out\": \"{}\"}}",
    //     0, token2.account_id(), token1.account_id(), 1
    // );
    // let msg_str = format!("{{\"force\": 0, \"actions\": [{}]}}", msg);
    // let out_come = call!(
    //     new_user,
    //     token2.ft_transfer_call(to_va(swap()), to_yocto("1").into(), None, msg_str.clone()),
    //     deposit = 1
    // );
    // out_come.assert_success();
    // assert_eq!(get_error_count(&out_come), 1);
    // assert!(get_error_status(&out_come).contains("E51: contract paused"));

    // withdraw token would fail
    let out_come = call!(
        root,
        pool.withdraw(to_va(eth()), U128(to_yocto("1")), None),
        deposit = 1
    );
    assert!(!out_come.is_ok());
    assert_eq!(get_error_count(&out_come), 1);
    assert!(get_error_status(&out_come).contains("E51: contract paused"));

    println!("Guardians Case 0104: only owner can resume the contract");
    let out_come = call!(
        root,
        pool.change_state(RunningState::Running),
        deposit = 1
    );
    assert!(!out_come.is_ok());
    assert_eq!(get_error_count(&out_come), 1);
    assert!(get_error_status(&out_come).contains("ERR_NOT_ALLOWED"));
    let metadata = get_metadata(&pool);
    assert_eq!(metadata.state, RunningState::Paused);

    let out_come = call!(
        guard2,
        pool.change_state(RunningState::Running),
        deposit = 1
    );
    assert!(!out_come.is_ok());
    assert_eq!(get_error_count(&out_come), 1);
    assert!(get_error_status(&out_come).contains("ERR_NOT_ALLOWED"));
    let metadata = get_metadata(&pool);
    assert_eq!(metadata.state, RunningState::Paused);

    let out_come = call!(
        owner,
        pool.change_state(RunningState::Running),
        deposit = 1
    );
    out_come.assert_success();
    assert_eq!(get_error_count(&out_come), 0);
    let metadata = get_metadata(&pool);
    assert_eq!(metadata.state, RunningState::Running);

    let out_come = call!(
        new_user,
        pool.storage_deposit(None, None),
        deposit = to_yocto("1")
    );
    out_come.assert_success();
    assert_eq!(get_error_count(&out_come), 0);
} 
