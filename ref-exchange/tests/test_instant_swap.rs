use near_sdk::json_types::U128;
use near_sdk_sim::{
    call, to_yocto, ContractAccount, ExecutionResult, UserAccount,
};

use test_token::ContractContract as TestToken;

use crate::common::utils::*;
pub mod common;

fn pack_action(
    pool_id: u32,
    token_in: &str,
    token_out: &str,
    amount_in: Option<u128>,
    min_amount_out: u128,
) -> String {
    if let Some(amount_in) = amount_in {
        format!(
            "{{\"pool_id\": {}, \"token_in\": \"{}\", \"amount_in\": \"{}\", \"token_out\": \"{}\", \"min_amount_out\": \"{}\"}}",
            pool_id, token_in, amount_in, token_out, min_amount_out
        )
    } else {
        format!(
            "{{\"pool_id\": {}, \"token_in\": \"{}\", \"token_out\": \"{}\", \"min_amount_out\": \"{}\"}}",
            pool_id, token_in, token_out, min_amount_out
        )
    }
}

fn direct_swap(
    user: &UserAccount,
    contract: &ContractAccount<TestToken>,
    actions: Vec<String>,
) -> ExecutionResult {
    // {{\"pool_id\": 0, \"token_in\": \"dai\", \"token_out\": \"eth\", \"min_amount_out\": \"1\"}}
    let actions_str = actions.join(", ");
    let msg_str = format!("{{\"actions\": [{}]}}", actions_str);
    // println!("{}", msg_str);
    call!(
        user,
        contract.ft_transfer_call(to_va(swap()), to_yocto("1").into(), None, msg_str),
        deposit = 1
    )
}

#[test]
fn instant_swap_scenario_01() {
    let (root, owner, pool, token1, token2, _) = setup_pool_with_liquidity();
    let new_user = root.create_user("new_user".to_string(), to_yocto("100"));
    call!(
        new_user,
        token1.mint(to_va(new_user.account_id.clone()), U128(to_yocto("10")))
    )
    .assert_success();

    println!("Case 0101: wrong msg");
    let out_come = direct_swap(&new_user, &token1, vec!["wrong".to_string()]);
    out_come.assert_success();
    assert_eq!(get_error_count(&out_come), 1);
    assert!(get_error_status(&out_come).contains("E28: Illegal msg in ft_transfer_call"));
    assert_eq!(balance_of(&token1, &new_user.account_id), to_yocto("10"));
    assert_eq!(balance_of(&token2, &new_user.account_id), to_yocto("0"));

    println!("Case 0102: less then min_amount_out");
    let action = pack_action(
        0,
        &token1.account_id(),
        &token2.account_id(),
        None,
        to_yocto("1.9"),
    );
    let out_come = direct_swap(&new_user, &token1, vec![action]);
    out_come.assert_success();
    // println!("{:#?}", out_come.promise_results());
    assert_eq!(get_error_count(&out_come), 1);
    assert!(get_error_status(&out_come)
        .contains("Smart contract panicked: panicked at 'ERR_MIN_AMOUNT'"));
    assert!(get_storage_balance(&pool, new_user.valid_account_id()).is_none());
    assert_eq!(balance_of(&token1, &new_user.account_id), to_yocto("10"));
    assert_eq!(balance_of(&token2, &new_user.account_id), to_yocto("0"));

    println!("Case 0103: non-registered user swap but not registered in token2");
    let action = pack_action(0, &token1.account_id(), &token2.account_id(), None, 1);
    let out_come = direct_swap(&new_user, &token1, vec![action]);
    out_come.assert_success();
    // println!("{:#?}", out_come.promise_results());
    assert_eq!(get_error_count(&out_come), 1);
    assert!(get_error_status(&out_come)
        .contains("Smart contract panicked: The account new_user is not registered"));
    // println!("total logs: {:#?}", get_logs(&out_come));
    // assert!(get_logs(&out_come)[2].contains("Account new_user is not registered. Depositing to owner."));
    assert!(get_storage_balance(&pool, new_user.valid_account_id()).is_none());
    assert_eq!(balance_of(&token1, &new_user.account_id), to_yocto("9"));
    assert!(
        get_deposits(&pool, owner.valid_account_id())
            .get(&token2.account_id())
            .unwrap()
            .0
            > to_yocto("1.8")
    );

    call!(
        new_user,
        token2.storage_deposit(None, None),
        deposit = to_yocto("1")
    )
    .assert_success();

    println!("Case 0104: non-registered user swap");
    let action = pack_action(0, &token1.account_id(), &token2.account_id(), None, 1);
    let out_come = direct_swap(&new_user, &token1, vec![action]);
    out_come.assert_success();
    assert_eq!(get_error_count(&out_come), 0);
    // println!("{:#?}", out_come.promise_results());
    // println!("total logs: {:#?}", get_logs(&out_come));
    assert!(get_storage_balance(&pool, new_user.valid_account_id()).is_none());
    assert_eq!(balance_of(&token1, &new_user.account_id), to_yocto("8"));
    assert!(balance_of(&token2, &new_user.account_id) > to_yocto("1.5"));
}

#[test]
fn instant_swap_scenario_02() {
    let (root, owner, pool, token1, token2, token3) = setup_pool_with_liquidity();
    let new_user = root.create_user("new_user".to_string(), to_yocto("100"));
    call!(
        new_user,
        token1.mint(to_va(new_user.account_id.clone()), U128(to_yocto("10")))
    )
    .assert_success();

    println!("Case 0201: registered user without any deposits and non-registered to token2");
    call!(
        new_user,
        pool.storage_deposit(None, Some(true)),
        deposit = to_yocto("1")
    )
    .assert_success();
    assert_eq!(
        get_storage_balance(&pool, new_user.valid_account_id())
            .unwrap()
            .available
            .0,
        to_yocto("0")
    );
    assert_eq!(
        get_storage_balance(&pool, new_user.valid_account_id())
            .unwrap()
            .total
            .0,
        to_yocto("0.00102")
    );
    // println!("{:#?}", get_storage_balance(&pool, new_user.valid_account_id()).unwrap());
    let action = pack_action(0, &token1.account_id(), &token2.account_id(), None, 1);
    let out_come = direct_swap(&new_user, &token1, vec![action]);
    out_come.assert_success();
    // println!("swap one logs: {:#?}", get_logs(&out_come));
    // println!("{:#?}", out_come.promise_results());
    assert_eq!(get_error_count(&out_come), 1);
    assert!(get_error_status(&out_come)
        .contains("Smart contract panicked: The account new_user is not registered"));
    // println!("total logs: {:#?}", get_logs(&out_come));
    assert!(get_logs(&out_come)[2].contains("Account new_user has not enough storage. Depositing to owner."));
    assert_eq!(
        get_storage_balance(&pool, new_user.valid_account_id())
            .unwrap()
            .available
            .0,
        to_yocto("0")
    );
    assert_eq!(
        get_storage_balance(&pool, new_user.valid_account_id())
            .unwrap()
            .total
            .0,
        to_yocto("0.00102")
    );
    assert_eq!(balance_of(&token1, &new_user.account_id), to_yocto("9"));
    assert!(
        get_deposits(&pool, owner.valid_account_id())
            .get(&token2.account_id())
            .unwrap()
            .0
            > to_yocto("1.8")
    );
    assert!(get_deposits(&pool, new_user.valid_account_id())
        .get(&token1.account_id())
        .is_none());
    assert!(get_deposits(&pool, new_user.valid_account_id())
        .get(&token2.account_id())
        .is_none());

    println!("Case 0202: registered user without any deposits");
    call!(
        new_user,
        token2.mint(to_va(new_user.account_id.clone()), U128(to_yocto("10")))
    )
    .assert_success();
    assert_eq!(balance_of(&token1, &new_user.account_id), to_yocto("9"));
    assert_eq!(balance_of(&token2, &new_user.account_id), to_yocto("10"));
    let action = pack_action(0, &token1.account_id(), &token2.account_id(), None, 1);
    let out_come = direct_swap(&new_user, &token1, vec![action]);
    out_come.assert_success();
    // println!("{:#?}", out_come.promise_results());
    // println!("total logs: {:#?}", get_logs(&out_come));
    assert_eq!(get_error_count(&out_come), 0);
    assert_eq!(
        get_storage_balance(&pool, new_user.valid_account_id())
            .unwrap()
            .available
            .0,
        0
    );
    assert_eq!(
        get_storage_balance(&pool, new_user.valid_account_id())
            .unwrap()
            .total
            .0,
        to_yocto("0.00102")
    );
    assert_eq!(balance_of(&token1, &new_user.account_id), to_yocto("8"));
    assert!(balance_of(&token2, &new_user.account_id) > to_yocto("11.5"));

    println!("Case 0203: registered user with token already deposited");
    call!(
        new_user,
        pool.storage_deposit(None, None),
        deposit = to_yocto("1")
    )
    .assert_success();
    call!(
        new_user,
        token1.ft_transfer_call(to_va(swap()), to_yocto("5").into(), None, "".to_string()),
        deposit = 1
    )
    .assert_success();
    call!(
        new_user,
        token2.ft_transfer_call(to_va(swap()), to_yocto("5").into(), None, "".to_string()),
        deposit = 1
    )
    .assert_success();
    assert_eq!(
        get_deposits(&pool, new_user.valid_account_id())
            .get(&token1.account_id())
            .unwrap()
            .0,
        to_yocto("5")
    );
    assert_eq!(
        get_deposits(&pool, new_user.valid_account_id())
            .get(&token2.account_id())
            .unwrap()
            .0,
        to_yocto("5")
    );
    let action = pack_action(0, &token1.account_id(), &token2.account_id(), None, 1);
    let out_come = direct_swap(&new_user, &token1, vec![action]);
    out_come.assert_success();
    assert_eq!(get_error_count(&out_come), 0);
    assert_eq!(
        get_deposits(&pool, new_user.valid_account_id())
            .get(&token1.account_id())
            .unwrap()
            .0,
        to_yocto("5")
    );
    assert_eq!(
        get_deposits(&pool, new_user.valid_account_id())
            .get(&token2.account_id())
            .unwrap()
            .0,
        to_yocto("5")
    );
    assert_eq!(balance_of(&token1, &new_user.account_id), to_yocto("2"));
    assert!(balance_of(&token2, &new_user.account_id) > to_yocto("7.7"));

    println!("Case 0204: deposit token is not in action");
    call!(
        new_user,
        token3.mint(to_va(new_user.account_id.clone()), U128(to_yocto("10")))
    )
    .assert_success();
    let action = pack_action(0, &token1.account_id(), &token2.account_id(), None, 1);
    let out_come = direct_swap(&new_user, &token3, vec![action]);
    out_come.assert_success();
    assert_eq!(get_error_count(&out_come), 1);
    // println!("{}", get_error_status(&out_come));
    assert!(get_error_status(&out_come).contains("E21: token not registered"));
    assert_eq!(
        get_deposits(&pool, new_user.valid_account_id())
            .get(&token1.account_id())
            .unwrap()
            .0,
        to_yocto("5")
    );
    assert_eq!(
        get_deposits(&pool, new_user.valid_account_id())
            .get(&token2.account_id())
            .unwrap()
            .0,
        to_yocto("5")
    );
}

#[test]
fn instant_swap_scenario_03() {
    let (root, owner, pool, token1, token2, token3) = setup_pool_with_liquidity();
    let new_user = root.create_user("new_user".to_string(), to_yocto("100"));
    call!(
        new_user,
        token1.mint(to_va(new_user.account_id.clone()), U128(to_yocto("5")))
    )
    .assert_success();
    call!(
        new_user,
        token2.mint(to_va(new_user.account_id.clone()), U128(to_yocto("5")))
    )
    .assert_success();
    call!(
        new_user,
        token3.mint(to_va(new_user.account_id.clone()), U128(to_yocto("10")))
    )
    .assert_success();
    call!(
        new_user,
        pool.storage_deposit(None, None),
        deposit = to_yocto("1")
    )
    .assert_success();
    call!(
        new_user,
        token3.ft_transfer_call(to_va(swap()), to_yocto("5").into(), None, "".to_string()),
        deposit = 1
    )
    .assert_success();
    call!(
        new_user,
        pool.storage_withdraw(None),
        deposit = 1
    )
    .assert_success();


    println!("Case 0301: two actions with one output token");
    let action1 = pack_action(0, &token1.account_id(), &token2.account_id(), None, 1);
    let action2 = pack_action(1, &token2.account_id(), &token3.account_id(), None, 1);
    let out_come = direct_swap(&new_user, &token1, vec![action1, action2]);
    out_come.assert_success();
    // println!("{:#?}", out_come.promise_results());
    assert_eq!(get_error_count(&out_come), 0);

    assert_eq!(balance_of(&token1, &new_user.account_id), to_yocto("4"));
    assert_eq!(balance_of(&token2, &new_user.account_id), to_yocto("5"));
    // println!("token3 {}", balance_of(&token3, &new_user.account_id));
    assert!(balance_of(&token3, &new_user.account_id) > to_yocto("5.8"));

    println!("Case 0302: two actions with tow output token");
    let action1 = pack_action(0, &token1.account_id(), &token2.account_id(), None, 1);
    let action2 = pack_action(1, &token2.account_id(), &token3.account_id(), Some(to_yocto("1")), 1);
    let out_come = direct_swap(&new_user, &token1, vec![action1, action2]);
    out_come.assert_success();
    // println!("{:#?}", out_come.promise_results());
    assert_eq!(get_error_count(&out_come), 0);

    assert_eq!(balance_of(&token1, &new_user.account_id), to_yocto("3"));
    // println!("token2 {}", balance_of(&token2, &new_user.account_id));
    // println!("token3 {}", balance_of(&token3, &new_user.account_id));
    assert!(balance_of(&token2, &new_user.account_id) > to_yocto("5.5"));
    assert!(balance_of(&token3, &new_user.account_id) > to_yocto("6.2"));

    println!("Case 0303: two actions with two output token and send back token#2 fail");
    call!(new_user, token2.storage_unregister(Some(true)), deposit = 1).assert_success();
    assert!(!is_register_to_token(&token2, new_user.valid_account_id()));
    let action1 = pack_action(0, &token1.account_id(), &token2.account_id(), None, 1);
    let action2 = pack_action(1, &token2.account_id(), &token3.account_id(), Some(to_yocto("1")), 1);
    let out_come = direct_swap(&new_user, &token1, vec![action1, action2]);
    out_come.assert_success();
    // println!("{:#?}", out_come.promise_results());
    assert_eq!(get_error_count(&out_come), 1);
    assert!(get_error_status(&out_come)
        .contains("Smart contract panicked: The account new_user is not registered"));
    
    assert_eq!(balance_of(&token1, &new_user.account_id), to_yocto("2"));
    assert!(
        get_deposits(&pool, owner.valid_account_id())
            .get(&token2.account_id())
            .unwrap()
            .0 
            > to_yocto("0.27")
    );
    // println!("token3 {}", balance_of(&token3, &new_user.account_id));
    assert!(balance_of(&token3, &new_user.account_id) > to_yocto("6.6"));

    println!("Case 0304: two actions with two output token and send back token#3 fail");
    call!(
        new_user,
        token2.mint(to_va(new_user.account_id.clone()), U128(to_yocto("10")))
    )
    .assert_success();
    call!(new_user, token3.storage_unregister(Some(true)), deposit = 1).assert_success();
    assert!(!is_register_to_token(&token3, new_user.valid_account_id()));
    let action1 = pack_action(0, &token1.account_id(), &token2.account_id(), None, 1);
    let action2 = pack_action(
        1,
        &token2.account_id(),
        &token3.account_id(),
        Some(to_yocto("1")),
        1,
    );
    let out_come = direct_swap(&new_user, &token1, vec![action1, action2]);
    out_come.assert_success();
    // println!("{:#?}", out_come.promise_results());
    assert_eq!(get_error_count(&out_come), 1);
    assert!(get_error_status(&out_come)
        .contains("Smart contract panicked: The account new_user is not registered"));

    assert!(
        get_deposits(&pool, new_user.valid_account_id())
            .get(&token3.account_id())
            .unwrap()
            .0
            > to_yocto("5.38")
    );
    assert_eq!(balance_of(&token1, &new_user.account_id), to_yocto("1"));
    assert!(balance_of(&token2, &new_user.account_id) > to_yocto("10.09"));

    println!("Case 0305: two actions with the second one insurfficent");
    let action1 = pack_action(0, &token1.account_id(), &token2.account_id(), None, 1);
    let action2 = pack_action(
        1,
        &token2.account_id(),
        &token3.account_id(),
        Some(to_yocto("1.2")),
        1,
    );
    let out_come = direct_swap(&new_user, &token1, vec![action1, action2]);
    out_come.assert_success();
    assert_eq!(get_error_count(&out_come), 1);
    // println!("{}", get_error_status(&out_come));
    assert!(get_error_status(&out_come).contains("E22: not enough tokens in deposit"));
    assert_eq!(balance_of(&token1, &new_user.account_id), to_yocto("1"));
}