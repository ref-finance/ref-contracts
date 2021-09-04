
use near_sdk_sim::{call, init_simulator, to_yocto, view, 
    ContractAccount, ExecutionResult, UserAccount
};
use near_sdk::json_types::{U128};
use near_sdk::serde_json::Value;

use ref_exchange::{ContractContract as Exchange, PoolInfo, SwapAction};
use test_token::ContractContract as TestToken;

use crate::common::utils::*;
mod common;

fn pack_action(pool_id: u32, token_in: &str, token_out: &str, amount_in: Option<u128>, min_amount_out: u128) -> String {
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

fn direct_swap(user: &UserAccount, 
    contract: &ContractAccount<TestToken>, 
    actions: Vec<String> ,force: u8
) -> ExecutionResult {
    // {{\"pool_id\": 0, \"token_in\": \"dai\", \"token_out\": \"eth\", \"min_amount_out\": \"1\"}}
    let actions_str = actions.join(", ");
    let msg_str = format!("{{\"force\": {}, \"actions\": [{}]}}", force, actions_str);
    // println!("{}", msg_str);
    call!(
        user,
        contract.ft_transfer_call(
            to_va(swap()),
            to_yocto("1").into(),
            None,
            msg_str
        ),
        deposit = 1
    )
}

#[test]
fn instant_swap_scenario_01() {
    let (root, owner, pool, token1, token2, token3) = setup_pool_with_liquidity();
    let new_user = root.create_user("new_user".to_string(), to_yocto("100"));
    call!(
        new_user,
        token1.mint(to_va(new_user.account_id.clone()), U128(to_yocto("10")))
    )
    .assert_success();

    println!("Case 01: wrong msg");
    let out_come = direct_swap(&new_user, &token1, vec!["wrong".to_string()], 0);
    out_come.assert_success();
    assert_eq!(get_error_count(&out_come), 1);
    assert!(get_error_status(&out_come).contains("E28: Illegal msg in ft_transfer_call"));
    assert_eq!(balance_of(&token1, &new_user.account_id), to_yocto("10"));
    assert_eq!(balance_of(&token2, &new_user.account_id), to_yocto("0"));

    println!("Case 02: non-registered user swap with force = 0");
    let action = pack_action(0, &token1.account_id(), &token2.account_id(), None, 1);
    let out_come = direct_swap(&new_user, &token1, vec![action], 0);
    out_come.assert_success();
    assert_eq!(get_error_count(&out_come), 1);
    assert!(get_error_status(&out_come).contains("E10: account not registered"));
    assert!(get_storage_balance(&pool, new_user.valid_account_id()).is_none());
    assert_eq!(balance_of(&token1, &new_user.account_id), to_yocto("10"));
    assert_eq!(balance_of(&token2, &new_user.account_id), to_yocto("0"));

    println!("Case 03: non-registered user swap with force = 1 but not registered in token2");
    let action = pack_action(0, &token1.account_id(), &token2.account_id(), None, 1);
    let out_come = direct_swap(&new_user, &token1, vec![action], 1);
    out_come.assert_success();
    assert_eq!(get_error_count(&out_come), 1);
    assert!(get_error_status(&out_come).contains("Smart contract panicked: The account new_user is not registered"));
    // println!("total logs: {:#?}", get_logs(&out_come));
    assert!(get_logs(&out_come)[2].contains("Account new_user is not registered. Depositing to owner."));
    assert!(get_storage_balance(&pool, new_user.valid_account_id()).is_none());
    assert_eq!(balance_of(&token1, &new_user.account_id), to_yocto("9"));
    assert!(get_deposits(&pool, owner.valid_account_id()).get(&token2.account_id()).unwrap().0 > to_yocto("1.8"));

    call!(
        new_user,
        token2.storage_deposit(None, None),
        deposit = to_yocto("1")
    )
    .assert_success();

    println!("Case 04: non-registered user swap with force = 1");
    let action = pack_action(0, &token1.account_id(), &token2.account_id(), None, 1);
    let out_come = direct_swap(&new_user, &token1, vec![action], 1);
    out_come.assert_success();
    assert_eq!(get_error_count(&out_come), 0);
    // println!("{:#?}", out_come.promise_results());
    // println!("total logs: {:#?}", get_logs(&out_come));  
    assert!(get_storage_balance(&pool, new_user.valid_account_id()).is_none());
    assert_eq!(balance_of(&token1, &new_user.account_id), to_yocto("8"));
    assert!(balance_of(&token2, &new_user.account_id) > to_yocto("1.5"));
}