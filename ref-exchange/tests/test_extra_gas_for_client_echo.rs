use crate::common::utils::*;
pub mod common;

use test_token::ContractContract as TestToken;
use mock_boost_farming::{ContractContract as MockBoostFarming};

use near_sdk::{json_types::U128, serde_json::Value, AccountId};
use near_sdk_sim::{call, deploy, view, to_yocto, ContractAccount, ExecutionResult, UserAccount};

near_sdk_sim::lazy_static_include::lazy_static_include_bytes! {
    MOCK_BOOST_FARMING_WASM_BYTES => "../res/mock_boost_farming.wasm",
}

fn do_swap(
    user: &UserAccount,
    contract: &ContractAccount<TestToken>,
    actions: Vec<String>,
    amount: u128,
    client_echo: Option<String>,
    swap_out_recipient: Option<AccountId>,
    extra_tgas: Option<u32>,
) -> ExecutionResult {
    let client_echo = if let Some(client_echo) = client_echo {
        format!(",\"client_echo\":\"{}\"", client_echo)
    } else {
        "".to_string()
    };
    let swap_out_recipient = if let Some(swap_out_recipient) = swap_out_recipient {
        format!(",\"swap_out_recipient\":\"{}\"", swap_out_recipient)
    } else {
        "".to_string()
    };
    let extra_tgas = if let Some(extra_tgas) = extra_tgas {
        format!(",\"extra_tgas_for_client_echo\":{}", extra_tgas)
    } else {
        "".to_string()
    };
    let actions_str = actions.join(", ");
    let msg_str = format!(
        "{{\"actions\": [{}]{}{}{}}}",
        actions_str, client_echo, swap_out_recipient, extra_tgas
    );
    call!(
        user,
        contract.ft_transfer_call(to_va(swap()), amount.into(), None, msg_str),
        deposit = 1
    )
}

fn pack_action(
    pool_id: u32,
    token_in: &str,
    token_out: &str,
    min_amount_out: u128,
) -> String {

    format!(
        "{{\"pool_id\": {}, \"token_in\": \"{}\", \"token_out\": \"{}\", \"min_amount_out\": \"{}\"}}",
        pool_id, token_in, token_out, min_amount_out
    )
    
}

fn boost_farming() -> AccountId {
    "boost_farming".to_string()
}

#[test]
fn test_extra_gas() {
    let (root, owner, pool, token1, token2, _) = setup_pool_with_liquidity();
    let new_user = root.create_user("new_user".to_string(), to_yocto("100"));
    let mock_boost_farming = deploy!(
        contract: MockBoostFarming,
        contract_id: boost_farming(),
        bytes: &MOCK_BOOST_FARMING_WASM_BYTES,
        signer_account: root,
        init_method: new(root.account_id())
    );
    let outcome = call!(
        root,
        mock_boost_farming.create_seed(token2.account_id(),24, Some(U128(0)), Some(0)),
        deposit = 1
    );
    outcome.assert_success();
    call!(
        pool.user_account,
        mock_boost_farming.storage_deposit(None, None),
        deposit = to_yocto("1")
    )
    .assert_success();

    call!(
        mock_boost_farming.user_account,
        token1.mint(to_va(mock_boost_farming.user_account.account_id.clone()), U128(to_yocto("10")))
    )
    .assert_success();
    call!(
        mock_boost_farming.user_account,
        token2.storage_deposit(None, None),
        deposit = to_yocto("1")
    )
    .assert_success();
    call!(
        new_user,
        token2.storage_deposit(None, None),
        deposit = to_yocto("1")
    )
    .assert_success();
    assert_eq!(balance_of(&token1, &mock_boost_farming.user_account.account_id), to_yocto("10"));

    let action = pack_action(0, &token1.account_id(), &token2.account_id(), 0);

    call!(
        owner,
        pool.extend_client_echo_token_id_whitelist(vec![token1.account_id()]),
        deposit = 1
    )
    .assert_success();

    assert_eq!(balance_of(&token1, &mock_boost_farming.user_account.account_id), to_yocto("10"));
    assert_eq!(balance_of(&token2, &mock_boost_farming.user_account.account_id), to_yocto("0"));

    call!(
        owner,
        pool.extend_client_echo_sender_id_whitelist(vec![mock_boost_farming.account_id()]),
        deposit = 1
    )
    .assert_success();

    // use default 15T as extra gas by not passing extra_tgas_for_client_echo field
    let out_come = do_swap(
        &mock_boost_farming.user_account,
        &token1,
        vec![action.clone()],
        to_yocto("1"),
        Some("\\\"Free\\\"".to_string()),
        None,
        None,
    );
    out_come.assert_success();
    // println!("final outcome: {:#?}", out_come);
    // println!("promise_results: {:#?}", out_come.promise_results());
    // println!("promise_errors: {:#?}", out_come.promise_errors());
    // println!("logs: {:#?}", get_logs(&out_come));
    assert_eq!(*get_logs(&out_come).get(4).unwrap(), "prepaid gas: 15000000000000".to_string());

    // mannually set extra gas to 10T
    let out_come = do_swap(
        &mock_boost_farming.user_account,
        &token1,
        vec![action.clone()],
        to_yocto("1"),
        Some("\\\"Free\\\"".to_string()),
        None,
        Some(10_u32),
    );
    out_come.assert_success();
    assert_eq!(*get_logs(&out_come).get(4).unwrap(), "prepaid gas: 10000000000000".to_string());

    // extra gas is less than needed, cause an error.
    let out_come = do_swap(
        &mock_boost_farming.user_account,
        &token1,
        vec![action.clone()],
        to_yocto("1"),
        Some("\\\"Free\\\"".to_string()),
        None,
        Some(5_u32),
    );
    out_come.assert_success();
    println!("logs: {:#?}", get_logs(&out_come));
    assert_eq!(get_error_count(&out_come), 1);
    assert_eq!(*get_logs(&out_come).get(4).unwrap(), "prepaid gas: 5000000000000".to_string());
    
}
