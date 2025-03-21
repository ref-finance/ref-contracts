use near_sdk::json_types::{U128};
use near_sdk_sim::{call, to_yocto};

use ref_exchange::{RunningState, SwapAction};
use crate::common::utils::*;
pub mod common;

#[test]
fn guardians_scenario_01() {
    let (root, owner, pool, token1, token2, _) = setup_pool_with_liquidity();
    let guard1 = root.create_user("guard1".to_string(), to_yocto("100"));
    let guard2 = root.create_user("guard2".to_string(), to_yocto("100"));
    let new_user = root.create_user("new_user".to_string(), to_yocto("100"));

    println!("Guardians Case 0101: only owner can add guardians");
    let out_come = call!(
        root,
        pool.extend_guardians(vec![guard1.valid_account_id(), guard2.valid_account_id()]),
        deposit=1
    );
    assert!(!out_come.is_ok());
    assert_eq!(get_error_count(&out_come), 1);
    assert!(get_error_status(&out_come).contains("E100: no permission to invoke this"));
    let metadata = get_metadata(&pool);
    assert_eq!(metadata.guardians.len(), 0);

    let out_come = call!(
        owner,
        pool.remove_guardians(vec![guard2.valid_account_id()]),
        deposit=1
    );
    assert!(!out_come.is_ok());
    assert_eq!(get_error_count(&out_come), 1);
    assert!(get_error_status(&out_come).contains("E104: guardian not in list"));
    let metadata = get_metadata(&pool);
    assert_eq!(metadata.guardians.len(), 0);

    let out_come = call!(
        owner,
        pool.extend_guardians(vec![guard1.valid_account_id(), guard2.valid_account_id()]),
        deposit=1
    );
    out_come.assert_success();
    assert_eq!(get_error_count(&out_come), 0);
    let metadata = get_metadata(&pool);
    assert_eq!(metadata.guardians.len(), 2);
    assert_eq!(metadata.guardians.get(0).unwrap().clone(), guard1.account_id());
    assert_eq!(metadata.guardians.get(1).unwrap().clone(), guard2.account_id());

    let out_come = call!(
        owner,
        pool.remove_guardians(vec![guard2.valid_account_id()]),
        deposit=1
    );
    out_come.assert_success();
    assert_eq!(get_error_count(&out_come), 0);
    let metadata = get_metadata(&pool);
    assert_eq!(metadata.guardians.len(), 1);
    assert_eq!(metadata.guardians.get(0).unwrap().clone(), guard1.account_id());

    let out_come = call!(
        owner,
        pool.extend_guardians(vec![guard1.valid_account_id(), guard2.valid_account_id()]),
        deposit=1
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
        pool.remove_whitelisted_tokens(vec![to_va(eth()), to_va(dai())]),
        deposit=1
    );
    assert!(!out_come.is_ok());
    assert_eq!(get_error_count(&out_come), 1);
    assert!(get_error_status(&out_come).contains("E100: no permission to invoke this"));
    let wl = get_whitelist(&pool);
    assert_eq!(wl.len(), 3);
    assert_eq!(wl.get(0).unwrap().clone(), dai());
    assert_eq!(wl.get(1).unwrap().clone(), eth());
    assert_eq!(wl.get(2).unwrap().clone(), usdt());

    let out_come = call!(
        owner,
        pool.remove_whitelisted_tokens(vec![to_va(usdt()), to_va(eth()), to_va(dai())]),
        deposit=1
    );
    out_come.assert_success();
    assert_eq!(get_error_count(&out_come), 0);
    let wl = get_whitelist(&pool);
    assert_eq!(wl.len(), 0);

    let out_come = call!(
        owner,
        pool.remove_whitelisted_tokens(vec![to_va(usdt())]),
        deposit=1
    );
    assert!(!out_come.is_ok());
    assert_eq!(get_error_count(&out_come), 1);
    assert!(get_error_status(&out_come).contains("E53: token not in list"));

    let out_come = call!(
        owner,
        pool.extend_whitelisted_tokens(vec![to_va(dai())]),
        deposit=1
    );
    out_come.assert_success();
    assert_eq!(get_error_count(&out_come), 0);
    let wl = get_whitelist(&pool);
    assert_eq!(wl.len(), 1);
    assert_eq!(wl.get(0).unwrap().clone(), dai());

    let out_come = call!(
        guard1,
        pool.extend_whitelisted_tokens(vec![to_va(eth())]),
        deposit=1
    );
    out_come.assert_success();
    assert_eq!(get_error_count(&out_come), 0);
    let wl = get_whitelist(&pool);
    assert_eq!(wl.len(), 2);
    assert_eq!(wl.get(0).unwrap().clone(), dai());
    assert_eq!(wl.get(1).unwrap().clone(), eth());

    let out_come = call!(
        guard2,
        pool.extend_whitelisted_tokens(vec![to_va(usdt())]),
        deposit=1
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
    assert!(get_error_status(&out_come).contains("E100: no permission to invoke this"));
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

    // add liquidity would fail
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
            None,
            None
        ),
        deposit = 1
    );
    assert!(!out_come.is_ok());
    assert_eq!(get_error_count(&out_come), 1);
    assert!(get_error_status(&out_come).contains("E51: contract paused"));

    // instant swap would fail
    call!(
        new_user,
        token2.mint(to_va(new_user.account_id.clone()), U128(to_yocto("10")))
    )
    .assert_success();
    let msg = format!(
        "{{\"pool_id\": {}, \"token_in\": \"{}\", \"token_out\": \"{}\", \"min_amount_out\": \"{}\"}}",
        0, token2.account_id(), token1.account_id(), 1
    );
    let msg_str = format!("{{\"force\": 0, \"actions\": [{}]}}", msg);
    let out_come = call!(
        new_user,
        token2.ft_transfer_call(to_va(swap()), to_yocto("1").into(), None, msg_str.clone()),
        deposit = 1
    );
    out_come.assert_success();
    assert_eq!(get_error_count(&out_come), 1);
    assert!(get_error_status(&out_come).contains("E51: contract paused"));

    // withdraw token would fail
    let out_come = call!(
        root,
        pool.withdraw(to_va(eth()), U128(to_yocto("1")), None, None),
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
    assert!(get_error_status(&out_come).contains("E100: no permission to invoke this"));
    let metadata = get_metadata(&pool);
    assert_eq!(metadata.state, RunningState::Paused);

    let out_come = call!(
        guard2,
        pool.change_state(RunningState::Running),
        deposit = 1
    );
    assert!(!out_come.is_ok());
    assert_eq!(get_error_count(&out_come), 1);
    assert!(get_error_status(&out_come).contains("E100: no permission to invoke this"));
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

#[test]
fn guardians_scenario_02() {
    let (root, old_owner, pool, _, token2, token3) = setup_pool_with_liquidity();
    let guard1 = root.create_user("guard1".to_string(), to_yocto("100"));
    let owner = root.create_user("owner2".to_string(), to_yocto("100"));
    call!(
        old_owner,
        pool.set_owner(owner.valid_account_id()),
        deposit=1
    ).assert_success();
    call!(
        owner,
        pool.extend_guardians(vec![guard1.valid_account_id()]),
        deposit=1
    ).assert_success();
    call!(
        owner,
        pool.modify_admin_fee(2000),
        deposit=1
    ).assert_success();
    call!(
        root,
        pool.swap(
            vec![SwapAction {
                pool_id: 1,
                token_in: usdt(),
                amount_in: Some(U128(to_yocto("1"))),
                token_out: eth(),
                min_amount_out: U128(1)
            }],
            None,
            None
        ),
        deposit = 1
    ).assert_success();
    assert_eq!(mft_balance_of(&pool, ":1", &pool.account_id()), 22731147428066673554);
    
    // guardians remove liquidity but owner account not ready
    println!("Guardians Case 0201: remove liquidity fail if owner account is not ready");
    let out_come = call!(
        guard1,
        pool.remove_exchange_fee_liquidity(1, U128(22731147428066673554), vec![U128(1), U128(1)]),
        deposit = 1
    );
    // println!("{:#?}", out_come.promise_results());
    assert!(!out_come.is_ok());
    assert_eq!(get_error_count(&out_come), 1);
    assert!(get_error_status(&out_come).contains("E10: account not registered"));
    assert_eq!(mft_balance_of(&pool, ":1", &pool.account_id()), 22731147428066673554);

    // guardians remove liquidity
    println!("Guardians Case 0202: remove liquidity success");
    call!(
        owner,
        pool.storage_deposit(None, None),
        deposit = to_yocto("1")
    )
    .assert_success();
    let out_come = call!(
        guard1,
        pool.remove_exchange_fee_liquidity(1, U128(22731147428066673554), vec![U128(1), U128(1)]),
        deposit = 1
    );
    out_come.assert_success();
    assert_eq!(mft_balance_of(&pool, ":1", &pool.account_id()), 0);
    let owner_deposits = get_deposits(&pool, owner.valid_account_id());
    assert_eq!(owner_deposits.get(&token2.account_id()).unwrap().0, 413378144755595527105);
    assert_eq!(owner_deposits.get(&token3.account_id()).unwrap().0, 250036938082231399513);

    // guardians withdraw owner token but owner not registered on token
    println!("Guardians Case 0203: withdraw owner token fail if owner not registered on token");
    let out_come = call!(
        guard1,
        pool.withdraw_owner_token(token2.valid_account_id(), U128(413378144755595527105), None),
        deposit = 1
    );
    out_come.assert_success();
    assert_eq!(get_error_count(&out_come), 1);
    assert!(get_error_status(&out_come).contains("The account owner2 is not registered"));
    let owner_deposits = get_deposits(&pool, owner.valid_account_id());
    assert_eq!(owner_deposits.get(&token2.account_id()).unwrap().0, 413378144755595527105);
    assert_eq!(balance_of(&token2, &owner.account_id()), 0);

    // guardians withdraw owner token
    println!("Guardians Case 0204: withdraw owner token success");
    call!(
        owner,
        token2.storage_deposit(None, Some(true)),
        deposit = to_yocto("1")
    )
    .assert_success();
    let out_come = call!(
        guard1,
        pool.withdraw_owner_token(token2.valid_account_id(), U128(413378144755595527105), None),
        deposit = 1
    );
    out_come.assert_success();
    assert_eq!(get_error_count(&out_come), 0);
    let owner_deposits = get_deposits(&pool, owner.valid_account_id());
    assert_eq!(owner_deposits.get(&token2.account_id()).unwrap().0, 0);
    assert_eq!(balance_of(&token2, &owner.account_id()), 413378144755595527105);
}

#[test]
fn guardians_scenario_03() {
    let (root, owner, pool, _, _, _) = setup_pool_with_liquidity();
    let guard1 = root.create_user("guard1".to_string(), to_yocto("100"));
    let referral1 = root.create_user("referral1".to_string(), to_yocto("100"));
    let referral2 = root.create_user("referral2".to_string(), to_yocto("100"));

    call!(
        owner,
        pool.extend_guardians(vec![guard1.valid_account_id()]),
        deposit=1
    ).assert_success();

    println!("Guardians Case 0301: only owner and guardians can manage referrals");
    let out_come = call!(
        root,
        pool.insert_referral(referral1.valid_account_id(), 2000),
        deposit=1
    );
    assert!(!out_come.is_ok());
    assert_eq!(get_error_count(&out_come), 1);
    assert!(get_error_status(&out_come).contains("E100: no permission to invoke this"));

    let out_come = call!(
        guard1,
        pool.insert_referral(referral1.valid_account_id(), 0),
        deposit=1
    );
    assert!(!out_come.is_ok());
    assert_eq!(get_error_count(&out_come), 1);
    assert!(get_error_status(&out_come).contains("E132: Illegal referral fee"));

    let out_come = call!(
        guard1,
        pool.insert_referral(referral1.valid_account_id(), 10000),
        deposit=1
    );
    assert!(!out_come.is_ok());
    assert_eq!(get_error_count(&out_come), 1);
    assert!(get_error_status(&out_come).contains("E132: Illegal referral fee"));

    let referrals = list_referrals(&pool);
    assert_eq!(referrals.len(), 0);

    call!(
        guard1,
        pool.insert_referral(referral1.valid_account_id(), 2000),
        deposit=1
    ).assert_success();
    let referrals = list_referrals(&pool);
    assert_eq!(referrals.len(), 1);
    assert_eq!(referrals.get(&referral1.account_id()).unwrap(), &2000_u32);

    let out_come = call!(
        guard1,
        pool.insert_referral(referral1.valid_account_id(), 1000),
        deposit=1
    );
    assert!(!out_come.is_ok());
    assert_eq!(get_error_count(&out_come), 1);
    assert!(get_error_status(&out_come).contains("E130: Referral already exist"));
    let referrals = list_referrals(&pool);
    assert_eq!(referrals.len(), 1);

    let out_come = call!(
        guard1,
        pool.update_referral(referral2.valid_account_id(), 3000),
        deposit=1
    );
    assert!(!out_come.is_ok());
    assert_eq!(get_error_count(&out_come), 1);
    assert!(get_error_status(&out_come).contains("E131: Referral not exist"));
    let referrals = list_referrals(&pool);
    assert_eq!(referrals.len(), 1);

    call!(
        guard1,
        pool.update_referral(referral1.valid_account_id(), 3000),
        deposit=1
    ).assert_success();
    let referrals = list_referrals(&pool);
    assert_eq!(referrals.len(), 1);
    assert_eq!(referrals.get(&referral1.account_id()).unwrap(), &3000_u32);

    let out_come = call!(
        guard1,
        pool.remove_referral(referral2.valid_account_id()),
        deposit=1
    );
    assert!(!out_come.is_ok());
    assert_eq!(get_error_count(&out_come), 1);
    assert!(get_error_status(&out_come).contains("E131: Referral not exist"));
    let referrals = list_referrals(&pool);
    assert_eq!(referrals.len(), 1);

    call!(
        guard1,
        pool.insert_referral(referral2.valid_account_id(), 2000),
        deposit=1
    ).assert_success();
    call!(
        guard1,
        pool.remove_referral(referral1.valid_account_id()),
        deposit=1
    ).assert_success();
    let referrals = list_referrals(&pool);
    assert_eq!(referrals.len(), 1);
    assert_eq!(referrals.get(&referral2.account_id()).unwrap(), &2000_u32);
}