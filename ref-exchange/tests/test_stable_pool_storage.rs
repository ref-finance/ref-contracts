use std::convert::TryFrom;

use near_contract_standards::fungible_token::metadata::FungibleTokenMetadata;
use near_sdk::json_types::{U128};
use near_sdk::AccountId;
use near_sdk_sim::{
    call, view, to_yocto
};

use ref_exchange::{PoolInfo, SwapAction};
use crate::common::utils::*;
pub mod common;


const ONE_LPT: u128 = 1000000000000000000;
const ONE_DAI: u128 = 1000000000000000000;
const ONE_USDT: u128 = 1000000;
const ONE_USDC: u128 = 1000000;

#[test]
fn sim_stable_storage() {
    let (root, _owner, pool, tokens) = 
        setup_stable_pool_with_liquidity(
            vec![dai(), usdt(), usdc()],
            vec![100000*ONE_DAI, 100000*ONE_USDT, 100000*ONE_USDC],
            vec![18, 6, 6],
            25,
            10000,
        );
    let tokens = &tokens;

    // prepare a new user with 3 tokens storage 102 + 3 * 148 = 102 + 444 = 546
    let new_user = root.create_user("new_user1".to_string(), to_yocto("100"));
    call!(
        new_user,
        pool.storage_deposit(None, None),
        deposit = to_yocto("1")
    )
    .assert_success();
    let ss = get_storage_state(&pool, new_user.valid_account_id()).unwrap();
    // println!("{:?}", ss);
    assert_eq!(ss.deposit.0, to_yocto("1"));
    assert_eq!(ss.usage.0, to_yocto("0.00102"));

    for c in tokens.into_iter() {
        call!(
            new_user,
            c.mint(new_user.valid_account_id(), to_yocto("100").into())
        )
        .assert_success();
        call!(
            new_user,
            c.ft_transfer_call(pool.valid_account_id(), to_yocto("100").into(), None, "".to_string()),
            deposit = 1
        ).assert_success();
    }
    let ss = get_storage_state(&pool, new_user.valid_account_id()).unwrap();
    assert_eq!(ss.usage.0, to_yocto("0.00546"));

    // appending balanced liqudity with basic lp register storage fee
    // call!(
    //     new_user,
    //     pool.add_liquidity(0, vec![U128(to_yocto("10")), U128(to_yocto("10")), U128(to_yocto("10"))], None),
    //     deposit = to_yocto("0.0007")
    // )
    // .assert_success();
    
    // ERR_STORAGE_DEPOSIT need 730000000000000000000, attatched 700000000000000000000
    // call!(
    //     new_user,
    //     pool.add_liquidity(0, vec![U128(10*ONE_DAI), U128(10*ONE_USDT), U128(10*ONE_USDC)], None),
    //     deposit = to_yocto("0.00073")
    // )
    // .assert_success();
    // let ss = get_storage_state(&pool, new_user.valid_account_id()).unwrap();
    // assert_eq!(ss.usage.0, to_yocto("0.00546"));

    // appending imba liqudity with extra storage fee for exchange share

    let out_come = call!(
        new_user,
        pool.add_liquidity(0, vec![U128(5*ONE_DAI), U128(10*ONE_USDT), U128(15*ONE_USDC)], None),
        deposit = to_yocto("0.00074")
    );
    out_come.assert_success();
    // assert!(!out_come.is_ok());
    // let ex_status = format!("{:?}", out_come.promise_errors()[0].as_ref().unwrap().status());
    // println!("{}", ex_status);
    // assert!(ex_status.contains("ERR_STORAGE_DEPOSIT"));

    let ss = get_storage_state(&pool, new_user.valid_account_id()).unwrap();
    assert_eq!(ss.usage.0, to_yocto("0.00546"));

}