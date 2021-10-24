use std::collections::HashMap;
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
fn sim_stable_swap() {
    let (root, _owner, pool, tokens) = 
        setup_stable_pool_with_liquidity(
            vec![dai(), usdt(), usdc()],
            vec![100000*ONE_DAI, 100000*ONE_USDT, 100000*ONE_USDC],
            vec![18, 6, 6],
            25,
            10000,
        );
    let tokens = &tokens;
    assert_eq!(
        view!(pool.get_pool(0)).unwrap_json::<PoolInfo>(),
        PoolInfo {
            pool_kind: "STABLE_SWAP".to_string(),
            token_account_ids: tokens.into_iter().map(|x| x.account_id()).collect(),
            amounts: vec![U128(100000*ONE_DAI), U128(100000*ONE_USDT), U128(100000*ONE_USDC)],
            total_fee: 25,
            shares_total_supply: U128(300000*ONE_LPT),
        }
    );
    assert_eq!(
        view!(pool.mft_metadata(":0".to_string()))
            .unwrap_json::<FungibleTokenMetadata>()
            .name,
        "ref-pool-0"
    );
    assert_eq!(
        view!(pool.mft_balance_of(":0".to_string(), to_va(root.account_id.clone())))
            .unwrap_json::<U128>()
            .0,
        300000*ONE_LPT
    );
    let balances = view!(pool.get_deposits(root.valid_account_id()))
        .unwrap_json::<HashMap<AccountId, U128>>();
    let balances = balances.values().cloned().collect::<Vec<_>>();
    assert_eq!(balances, vec![U128(0), U128(0), U128(0)]);

    let c = tokens.get(0).unwrap();
    call!(
        root,
        c.ft_transfer_call(pool.valid_account_id(), U128(2 * ONE_DAI), None, "".to_string()),
        deposit = 1
    )
    .assert_success();

    let out_come = call!(
        root,
        pool.swap(
            vec![SwapAction {
                pool_id: 0,
                token_in: dai(),
                amount_in: Some(U128(ONE_DAI)),
                token_out: usdc(),
                min_amount_out: U128(1)
            }],
            None
        ),
        deposit = 1
    );
    out_come.assert_success();
    println!("{:#?}", get_logs(&out_come));

    let out_come = call!(
        root,
        pool.swap(
            vec![SwapAction {
                pool_id: 0,
                token_in: dai(),
                amount_in: Some(U128(ONE_DAI)),
                token_out: usdt(),
                min_amount_out: U128(1)
            }],
            None
        ),
        deposit = 1
    );
    out_come.assert_success();
    println!("{:#?}", get_logs(&out_come));

    let balances = view!(pool.get_deposits(root.valid_account_id()))
        .unwrap_json::<HashMap<AccountId, U128>>();
    assert_eq!(balances[&dai()].0, 0);
    assert_eq!(balances[&usdt()].0, 997499);
    assert_eq!(balances[&usdc()].0, 997499);

    assert_eq!(
        view!(pool.get_pool(0)).unwrap_json::<PoolInfo>(),
        PoolInfo {
            pool_kind: "STABLE_SWAP".to_string(),
            token_account_ids: tokens.into_iter().map(|x| x.account_id()).collect(),
            amounts: vec![U128(100002*ONE_DAI), U128(99999*ONE_USDT+2500), U128(99999*ONE_USDC+2500)],
            total_fee: 25,
            shares_total_supply: U128(300000*ONE_LPT+997999990125778),
        }
    );
}

#[test]
fn sim_stable_lp() {
    let (root, _owner, pool, tokens) = 
        setup_stable_pool_with_liquidity(
            vec![dai(), usdt(), usdc()],
            vec![100000*ONE_DAI, 100000*ONE_USDT, 100000*ONE_USDC],
            vec![18, 6, 6],
            25,
            10000,
        );
    let tokens = &tokens;

    // add more liquidity with balanced tokens
    let user1 = root.create_user("user1".to_string(), to_yocto("100"));
    mint_and_deposit_token(&user1, &tokens[0], &pool, 500*ONE_DAI);
    mint_and_deposit_token(&user1, &tokens[1], &pool, 500*ONE_USDT);
    mint_and_deposit_token(&user1, &tokens[2], &pool, 500*ONE_USDC);
    let out_come = call!(
        user1,
        pool.add_liquidity(0, vec![U128(500*ONE_DAI), U128(500*ONE_USDT), U128(500*ONE_USDC)], None),
        deposit = to_yocto("0.0007")
    );
    out_come.assert_success();
    println!("{:#?}", get_logs(&out_come));

    // add more liquidity with imba tokens
    let user2 = root.create_user("user2".to_string(), to_yocto("100"));
    mint_and_deposit_token(&user2, &tokens[0], &pool, 100*ONE_DAI);
    mint_and_deposit_token(&user2, &tokens[1], &pool, 200*ONE_USDT);
    mint_and_deposit_token(&user2, &tokens[2], &pool, 400*ONE_USDC);
    let out_come = call!(
        user2,
        pool.add_liquidity(0, vec![U128(100*ONE_DAI), U128(200*ONE_USDT), U128(400*ONE_USDC)], None),
        deposit = to_yocto("0.0014")  // 0.0007 for one lp and double it for admin fee
    );
    out_come.assert_success();
    println!("{:#?}", get_logs(&out_come));

    assert_eq!(
        view!(pool.get_pool(0)).unwrap_json::<PoolInfo>(),
        PoolInfo {
            pool_kind: "STABLE_SWAP".to_string(),
            token_account_ids: tokens.into_iter().map(|x| x.account_id()).collect(),
            amounts: vec![U128(100600*ONE_DAI), U128(100700*ONE_USDT), U128(100900*ONE_USDC)],
            total_fee: 25,
            shares_total_supply: U128(302200*ONE_LPT-252002571499322238),
        }
    );
    assert_eq!(mft_balance_of(&pool, ":0", &user1.account_id()), 1500*ONE_LPT);
    // user2 lp tokens = token_sum - fee_parts = token_sum - reduced_lp_token - admin_fee
    assert_eq!(mft_balance_of(&pool, ":0", &user2.account_id()), 700*ONE_LPT-252002571499322238-47999999736084776);


    // remove by shares

    // remove by tokens

    // tansfer some to other

    // other remove by shares

    // other remove by tokens
    
}
