use near_sdk::json_types::{U128};
use near_sdk_sim::{call, to_yocto};

use ref_exchange::SwapAction;
use crate::common::utils::*;
pub mod common;


#[test]
fn high_order_liquidity() {
    let (
        root, 
        owner, 
        pool, 
        token1, 
        _, 
        _
    ) = setup_pool_with_liquidity();
    assert_eq!(mft_balance_of(&pool, ":0", &root.account_id()), to_yocto("1"));
    assert_eq!(mft_balance_of(&pool, ":1", &root.account_id()), to_yocto("1"));
    assert_eq!(mft_balance_of(&pool, ":2", &root.account_id()), to_yocto("1"));

    // create high order pool
    let out_come = call!(
        owner,
        pool.add_high_order_simple_pool(vec![token1.account_id(), ":0".to_string()], 25),
        deposit = to_yocto("1")
    );
    out_come.assert_success();
    // println!("{:#?}", out_come.promise_results());
    
    // add liquidity
    let out_come = call!(
        root,
        pool.add_liquidity(3, vec![U128(to_yocto("10")), U128(to_yocto("0.2"))], None),
        deposit = to_yocto("0.0015")
    );
    out_come.assert_success();
    assert_eq!(mft_balance_of(&pool, ":0", &root.account_id()), to_yocto("0.8"));
    assert_eq!(mft_balance_of(&pool, ":3", &root.account_id()), to_yocto("1"));
    assert_eq!(get_deposits(&pool, root.valid_account_id()).get(&dai()).unwrap().0, to_yocto("75"));

    // remove liqudity
    let out_come = call!(
        root,
        pool.remove_liquidity(3, U128(to_yocto("0.5")), vec![U128(1), U128(1)]),
        deposit = 1
    );
    out_come.assert_success();
    assert_eq!(mft_balance_of(&pool, ":0", &root.account_id()), to_yocto("0.9"));
    assert_eq!(mft_balance_of(&pool, ":3", &root.account_id()), to_yocto("0.5"));
    assert_eq!(get_deposits(&pool, root.valid_account_id()).get(&token1.account_id()).unwrap().0, to_yocto("80"));
}

#[test]
fn high_order_ordinary_swap() {
    let (
        root, 
        owner, 
        pool, 
        token1, 
        _, 
        _
    ) = setup_pool_with_liquidity();

    // create high order pool
    let out_come = call!(
        owner,
        pool.add_high_order_simple_pool(vec![token1.account_id(), ":0".to_string()], 25),
        deposit = to_yocto("1")
    );
    out_come.assert_success();
    
    // add liquidity
    let out_come = call!(
        root,
        pool.add_liquidity(3, vec![U128(to_yocto("10")), U128(to_yocto("1"))], None),
        deposit = to_yocto("0.0015")
    );
    out_come.assert_success();

    // ordinary swap from old lp
    let out_come = call!(
        root,
        pool.swap(
            vec![SwapAction {
                pool_id: 3,
                token_in: token1.account_id(),
                amount_in: Some(U128(to_yocto("1"))),
                token_out: String::from(":0"),
                min_amount_out: U128(1)
            }],
            None
        ),
        deposit = 1
    );
    out_come.assert_success();
    println!("swap logs: {:#?}", get_logs(&out_come));

    let out_come = call!(
        root,
        pool.swap(
            vec![SwapAction {
                pool_id: 3,
                token_in: String::from(":0"),
                amount_in: Some(U128(90702432370993407592634)),
                token_out: token1.account_id(),
                min_amount_out: U128(1)
            }],
            None
        ),
        deposit = 1
    );
    out_come.assert_success();
    println!("swap logs: {:#?}", get_logs(&out_come));

    // oridnary swap from a non lp
    // prepare non lp
    let new_user = root.create_user("new_user".to_string(), to_yocto("100"));
    call!(
        new_user,
        token1.mint(to_va(new_user.account_id.clone()), U128(to_yocto("10")))
    )
    .assert_success();
    call!(
        new_user,
        pool.storage_deposit(None, Some(false)),
        deposit = to_yocto("1")
    )
    .assert_success();
    call!(
        new_user,
        token1.ft_transfer_call(to_va(swap()), to_yocto("10").into(), None, "".to_string()),
        deposit = 1
    )
    .assert_success();
    // swap would fail cause not register on mft token
    let out_come = call!(
        new_user,
        pool.swap(
            vec![SwapAction {
                pool_id: 3,
                token_in: token1.account_id(),
                amount_in: Some(U128(to_yocto("1"))),
                token_out: String::from(":0"),
                min_amount_out: U128(1)
            }],
            None
        ),
        deposit = 1
    );
    assert_eq!(get_error_count(&out_come), 1);
    assert!(get_error_status(&out_come)
        .contains("E13: LP not registered"));
    assert_eq!(mft_balance_of(&pool, ":0", &new_user.account_id()), to_yocto("0"));
    assert_eq!(mft_balance_of(&pool, ":3", &new_user.account_id()), to_yocto("0"));
    assert_eq!(get_deposits(&pool, new_user.valid_account_id()).get(&token1.account_id()).unwrap().0, to_yocto("10"));
    // register LP and then swap would succeed
    assert_eq!(get_mft_is_registered(&pool, String::from(":0"), new_user.valid_account_id()), false);
    let out_come = call!(
        new_user,
        pool.mft_register(String::from(":0"), new_user.valid_account_id()),
        deposit = to_yocto("0.0008")
    );
    out_come.assert_success();
    assert_eq!(get_mft_is_registered(&pool, String::from(":0"), new_user.valid_account_id()), true);
    let out_come = call!(
        new_user,
        pool.swap(
            vec![SwapAction {
                pool_id: 3,
                token_in: token1.account_id(),
                amount_in: Some(U128(to_yocto("1"))),
                token_out: String::from(":0"),
                min_amount_out: U128(1)
            }],
            None
        ),
        deposit = 1
    );
    out_come.assert_success();
    println!("swap logs: {:#?}", get_logs(&out_come));
    assert_eq!(mft_balance_of(&pool, ":0", &new_user.account_id()), 90664988826115278572728);
    assert_eq!(mft_balance_of(&pool, ":3", &new_user.account_id()), to_yocto("0"));
    assert_eq!(get_deposits(&pool, new_user.valid_account_id()).get(&token1.account_id()).unwrap().0, to_yocto("9"));

}

#[test]
fn high_order_instant_swap() {
    let (
        root, 
        owner, 
        pool, 
        token1, 
        _, 
        _
    ) = setup_pool_with_liquidity();

    // create high order pool
    let out_come = call!(
        owner,
        pool.add_high_order_simple_pool(vec![token1.account_id(), ":0".to_string()], 25),
        deposit = to_yocto("1")
    );
    out_come.assert_success();
    
    // add liquidity
    let out_come = call!(
        root,
        pool.add_liquidity(3, vec![U128(to_yocto("10")), U128(to_yocto("0.9"))], None),
        deposit = to_yocto("0.0015")
    );
    out_come.assert_success();


    let new_user = root.create_user("new_user".to_string(), to_yocto("100"));
    call!(
        new_user,
        token1.mint(to_va(new_user.account_id.clone()), U128(to_yocto("10")))
    )
    .assert_success();
    // ft -> mft and without mft_register
    // expected: ft_transfer_call revert
    println!("Case 0101: ft swap mft but mft not registered");
    let action = pack_action(3, &token1.account_id(), &String::from(":0"), None, 1);
    let out_come = direct_swap(&new_user, &token1, vec![action]);
    out_come.assert_success();
    assert_eq!(get_error_count(&out_come), 1);
    assert!(get_error_status(&out_come).contains("E13: LP not registered"));
    assert_eq!(mft_balance_of(&pool, ":0", &new_user.account_id()), to_yocto("0"));
    assert_eq!(mft_balance_of(&pool, ":3", &new_user.account_id()), to_yocto("0"));
    assert_eq!(balance_of(&token1, &new_user.account_id()), to_yocto("10"));

    // register LP and then swap would succeed
    println!("Case 0102: ft swap mft and mft has registered");
    assert_eq!(get_mft_is_registered(&pool, String::from(":0"), new_user.valid_account_id()), false);
    let out_come = call!(
        new_user,
        pool.mft_register(String::from(":0"), new_user.valid_account_id()),
        deposit = to_yocto("0.0008")
    );
    out_come.assert_success();
    assert_eq!(get_mft_is_registered(&pool, String::from(":0"), new_user.valid_account_id()), true);
    let action = pack_action(3, &token1.account_id(), &String::from(":0"), None, 1);
    let out_come = direct_swap(&new_user, &token1, vec![action]);
    out_come.assert_success();
    // println!("{:#?}", out_come.promise_results());
    assert_eq!(get_error_count(&out_come), 0);
    assert_eq!(mft_balance_of(&pool, ":0", &new_user.account_id()), 81632189133894066833371);
    assert_eq!(mft_balance_of(&pool, ":3", &new_user.account_id()), to_yocto("0"));
    assert_eq!(balance_of(&token1, &new_user.account_id()), to_yocto("9"));
    
    println!("Case 0103: mft swap ft and ft has registered");
    let action = pack_action(3, &String::from(":0"), &token1.account_id(), None, 1);
    let msg_str = format!("{{\"actions\": [{}]}}", action);
    // println!("{}", msg_str);
    let out_come = call!(
        new_user,
        pool.mft_transfer_call(String::from(":0"), pool.valid_account_id(), U128(81632189133894066833371), None, msg_str),
        deposit = 1
    );
    out_come.assert_success();
    // println!("{:#?}", out_come.promise_results());
    assert_eq!(get_error_count(&out_come), 0);
    assert_eq!(mft_balance_of(&pool, ":0", &new_user.account_id()), 0);
    assert_eq!(mft_balance_of(&pool, ":3", &new_user.account_id()), to_yocto("0"));
    assert_eq!(balance_of(&token1, &new_user.account_id()), to_yocto("9") + 995458165383034684495970);

    println!("Case 0104: mft swap ft and ft not registered");
    let new_user2 = root.create_user("new_user2".to_string(), to_yocto("10"));
    call!(
        new_user2,
        pool.mft_register(String::from(":0"), new_user2.valid_account_id()),
        deposit = to_yocto("0.0008")
    ).assert_success();
    call!(
        root,
        pool.mft_transfer(String::from(":0"), new_user2.valid_account_id(), U128(to_yocto("0.1")), None),
        deposit = 1
    ).assert_success();
    let action = pack_action(3, &String::from(":0"), &token1.account_id(), None, 1);
    let msg_str = format!("{{\"actions\": [{}]}}", action);
    // println!("{}", msg_str);
    let out_come = call!(
        new_user2,
        pool.mft_transfer_call(String::from(":0"), pool.valid_account_id(), U128(to_yocto("0.01")), None, msg_str),
        deposit = 1
    );
    out_come.assert_success();
    // println!("{:#?}", out_come.promise_results());
    assert_eq!(get_error_count(&out_come), 1);
    assert!(get_error_status(&out_come).contains("The account new_user2 is not registered"));
    assert_eq!(mft_balance_of(&pool, ":0", &new_user2.account_id()), to_yocto("0.09"));
    assert_eq!(get_deposits(&pool, owner.valid_account_id()).get(&token1.account_id()).unwrap().0, 109668182972393998760573);
}

