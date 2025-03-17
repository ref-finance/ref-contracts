use std::collections::HashMap;

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
const ONE_CUSD: u128 = 1000000000000000000000000;
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
            amp: 10000,
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
            None,
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
            None,
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
            amp: 10000,
            token_account_ids: tokens.into_iter().map(|x| x.account_id()).collect(),
            amounts: vec![U128(100002*ONE_DAI), U128(99999*ONE_USDT+2500), U128(99999*ONE_USDC+2500)],
            total_fee: 25,
            shares_total_supply: U128(300000*ONE_LPT + 499999996666583 + 499999993277742),
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
    let last_share_price = pool_share_price(&pool, 0);
    let last_lpt_supply = mft_total_supply(&pool, ":0");

    // add more liquidity with balanced tokens
    let user1 = root.create_user("user1".to_string(), to_yocto("100"));
    mint_and_deposit_token(&user1, &tokens[0], &pool, 500*ONE_DAI);
    mint_and_deposit_token(&user1, &tokens[1], &pool, 500*ONE_USDT);
    mint_and_deposit_token(&user1, &tokens[2], &pool, 500*ONE_USDC);
    let out_come = call!(
        user1,
        pool.add_stable_liquidity(0, vec![U128(500*ONE_DAI), U128(500*ONE_USDT), U128(500*ONE_USDC)], U128(1)),
        deposit = to_yocto("0.0007")
    );
    out_come.assert_success();
    println!("{:#?}", get_logs(&out_come));
    assert_eq!(pool_share_price(&pool, 0), last_share_price);
    assert_eq!(mft_total_supply(&pool, ":0"), last_lpt_supply + 1500*ONE_LPT);
    let last_lpt_supply = last_lpt_supply + 1500*ONE_LPT;

    // remove by shares
    let out_come = call!(
        user1,
        pool.remove_liquidity(0, U128(300*ONE_LPT), vec![U128(1*ONE_DAI), U128(1*ONE_USDT), U128(1*ONE_USDC)]),
        deposit = 1 
    );
    out_come.assert_success();
    println!("{:#?}", get_logs(&out_come));
    assert_eq!(mft_balance_of(&pool, ":0", &user1.account_id()), 1200*ONE_LPT);
    let balances = view!(pool.get_deposits(user1.valid_account_id()))
        .unwrap_json::<HashMap<AccountId, U128>>();
    assert_eq!(balances[&dai()].0, 100*ONE_DAI);
    assert_eq!(balances[&usdt()].0, 100*ONE_USDT);
    assert_eq!(balances[&usdc()].0, 100*ONE_USDC);
    assert_eq!(pool_share_price(&pool, 0), last_share_price);
    assert_eq!(mft_total_supply(&pool, ":0"), last_lpt_supply - 300*ONE_LPT);
    let last_lpt_supply = last_lpt_supply - 300*ONE_LPT;

    // add more liquidity with imba tokens
    let user2 = root.create_user("user2".to_string(), to_yocto("100"));
    mint_and_deposit_token(&user2, &tokens[0], &pool, 100*ONE_DAI);
    mint_and_deposit_token(&user2, &tokens[1], &pool, 200*ONE_USDT);
    mint_and_deposit_token(&user2, &tokens[2], &pool, 400*ONE_USDC);
    let out_come = call!(
        user2,
        pool.add_stable_liquidity(0, vec![U128(100*ONE_DAI), U128(200*ONE_USDT), U128(400*ONE_USDC)], U128(1)),
        deposit = to_yocto("0.0014")  // 0.0007 for one lp and double it for admin fee
    );
    out_come.assert_success();
    println!("{:#?}", get_logs(&out_come));
    // "Mint 699699997426210330025 shares for user2, fee is 299999998348895348 shares",
    // "Exchange swap got 59999999669779069 shares, No referral fee, from add_liquidity",

    assert_eq!(
        view!(pool.get_pool(0)).unwrap_json::<PoolInfo>(),
        PoolInfo {
            pool_kind: "STABLE_SWAP".to_string(),
            amp: 10000,
            token_account_ids: tokens.into_iter().map(|x| x.account_id()).collect(),
            amounts: vec![U128(100500*ONE_DAI), U128(100600*ONE_USDT), U128(100800*ONE_USDC)],
            total_fee: 25,
            shares_total_supply: U128(301200*ONE_LPT + 699699997426210330025 + 59999999669779069),
        }
    );
    assert_eq!(mft_balance_of(&pool, ":0", &user1.account_id()), 1200*ONE_LPT);
    assert_eq!(mft_balance_of(&pool, ":0", &user2.account_id()), 699699997426210330025);
    assert!(pool_share_price(&pool, 0) > last_share_price);
    let last_share_price = pool_share_price(&pool, 0);
    assert_eq!(mft_total_supply(&pool, ":0"), last_lpt_supply + 699699997426210330025 + 59999999669779069);
    let last_lpt_supply = last_lpt_supply + 699699997426210330025 + 59999999669779069;

    // remove by tokens
    let out_come = call!(
        user1,
        pool.remove_liquidity_by_tokens(0, vec![U128(1*ONE_DAI), U128(500*ONE_USDT), U128(1*ONE_USDC)], U128(550*ONE_LPT)),
        deposit = 1 
    );
    out_come.assert_success();
    println!("{:#?}", get_logs(&out_come));
    // "LP user1 removed 502598511257512352631 shares by given tokens, and fee is 598899301432400519 shares",
    // "Exchange swap got 119779860286480103 shares, No referral fee, from remove_liquidity_by_tokens",

    assert_eq!(mft_balance_of(&pool, ":0", &user1.account_id()), 1200*ONE_LPT - 502598511257512352631);
    let balances = view!(pool.get_deposits(user1.valid_account_id()))
        .unwrap_json::<HashMap<AccountId, U128>>();
    assert_eq!(balances[&dai()].0, 101*ONE_DAI);
    assert_eq!(balances[&usdt()].0, 600*ONE_USDT);
    assert_eq!(balances[&usdc()].0, 101*ONE_USDC);
    assert_eq!(
        view!(pool.get_pool(0)).unwrap_json::<PoolInfo>(),
        PoolInfo {
            pool_kind: "STABLE_SWAP".to_string(),
            amp: 10000,
            token_account_ids: tokens.into_iter().map(|x| x.account_id()).collect(),
            amounts: vec![U128(100499*ONE_DAI), U128(100100*ONE_USDT), U128(100799*ONE_USDC)],
            total_fee: 25,
            shares_total_supply: U128(last_lpt_supply - 502598511257512352631 + 119779860286480103),
        }
    );
    assert_eq!(mft_balance_of(&pool, ":0", &user1.account_id()), 1200*ONE_LPT - 502598511257512352631);
    assert_eq!(mft_balance_of(&pool, ":0", &user2.account_id()), 699699997426210330025);
    assert!(pool_share_price(&pool, 0) > last_share_price);
    let last_share_price = pool_share_price(&pool, 0);
    let last_lpt_supply = last_lpt_supply - 502598511257512352631 + 119779860286480103;

    // tansfer some to other
    let out_come = call!(
        user1,
        pool.mft_transfer(":0".to_string(), user2.valid_account_id(), U128(100*ONE_LPT), None),
        deposit = 1 
    );
    out_come.assert_success();
    println!("{:#?}", get_logs(&out_come));
    assert_eq!(mft_balance_of(&pool, ":0", &user1.account_id()), 1100*ONE_LPT - 502598511257512352631);
    assert_eq!(mft_balance_of(&pool, ":0", &user2.account_id()), 699699997426210330025 + 100*ONE_LPT);
    assert_eq!(pool_share_price(&pool, 0), last_share_price);
    assert_eq!(mft_total_supply(&pool, ":0"), last_lpt_supply);

    // other remove by shares trigger slippage
    let out_come = call!(
        user2,
        pool.remove_liquidity(0, U128(300*ONE_LPT), vec![U128(1*ONE_DAI), U128(298*ONE_USDT), U128(1*ONE_USDC)]),
        deposit = 1 
    );
    assert!(!out_come.is_ok());
    let ex_status = format!("{:?}", out_come.promise_errors()[0].as_ref().unwrap().status());
    // println!("ex_status: {}", ex_status);
    assert!(ex_status.contains("E68: slippage error"));
    assert_eq!(pool_share_price(&pool, 0), last_share_price);
    assert_eq!(mft_total_supply(&pool, ":0"), last_lpt_supply);

    // other remove by tokens trigger slippage
    let out_come = call!(
        user2,
        pool.remove_liquidity_by_tokens(0, vec![U128(1*ONE_DAI), U128(298*ONE_USDT), U128(1*ONE_USDC)], U128(300*ONE_LPT)),
        deposit = 1 
    );
    assert!(!out_come.is_ok());
    let ex_status = format!("{:?}", out_come.promise_errors()[0].as_ref().unwrap().status());
    assert!(ex_status.contains("E68: slippage error"));
    assert_eq!(pool_share_price(&pool, 0), last_share_price);
    assert_eq!(mft_total_supply(&pool, ":0"), last_lpt_supply);

    // user2 remove by share
    assert_eq!(mft_balance_of(&pool, ":0", &user1.account_id()), 1100*ONE_LPT - 502598511257512352631);
    assert_eq!(mft_balance_of(&pool, ":0", &user2.account_id()), 699699997426210330025 + 100*ONE_LPT);
    let out_come = call!(
        user2,
        pool.remove_liquidity(0, U128(300*ONE_LPT), vec![U128(1*ONE_DAI), U128(1*ONE_USDT), U128(1*ONE_USDC)]),
        deposit = 1 
    );
    out_come.assert_success();
    println!("{:#?}", get_logs(&out_come));
    assert_eq!(mft_balance_of(&pool, ":0", &user1.account_id()), 1100*ONE_LPT - 502598511257512352631);
    assert_eq!(mft_balance_of(&pool, ":0", &user2.account_id()), 699699997426210330025 - 200*ONE_LPT);
    assert_eq!(pool_share_price(&pool, 0), last_share_price);
    assert_eq!(mft_total_supply(&pool, ":0"), last_lpt_supply-300*ONE_LPT);
    let last_lpt_supply = last_lpt_supply - 300*ONE_LPT;
    
    // user2 remove by tokens
    let out_come = call!(
        user2,
        pool.remove_liquidity_by_tokens(0, vec![U128(498*ONE_DAI), U128(0*ONE_USDT), U128(0*ONE_USDC)], U128(499*ONE_LPT)),
        deposit = 1 
    );
    out_come.assert_success();
    println!("{:#?}", get_logs(&out_come));
    // "LP user2 removed 498596320225563082252 shares by given tokens, and fee is 597500435701476809 shares",
    // "Exchange swap got 119500087140295361 shares, No referral fee, from remove_liquidity_by_tokens",

    assert_eq!(mft_balance_of(&pool, ":0", &user1.account_id()), 1100*ONE_LPT - 502598511257512352631);
    // previous lpt - removed lpt
    assert_eq!(mft_balance_of(&pool, ":0", &user2.account_id()), 699699997426210330025 - 200*ONE_LPT - 498596320225563082252);
    // last_lpt_supply - removed lpt + admin_fee_to_lpt
    let last_lpt_supply = last_lpt_supply - 498596320225563082252 + 119500087140295361;
    assert_eq!(mft_total_supply(&pool, ":0"), last_lpt_supply);
    assert!(pool_share_price(&pool, 0) > last_share_price);
    let last_share_price = pool_share_price(&pool, 0);
    println!("share_price: {}", last_share_price);

    // add massive liquidity (100 billion)
    let user3 = root.create_user("user3".to_string(), to_yocto("100"));
    mint_and_deposit_token(&user3, &tokens[0], &pool, 100_000_000_000*ONE_DAI);
    mint_and_deposit_token(&user3, &tokens[1], &pool, 100_000_000_000*ONE_USDT);
    mint_and_deposit_token(&user3, &tokens[2], &pool, 100_000_000_000*ONE_USDC);
    let out_come = call!(
        user3,
        pool.add_stable_liquidity(0, vec![U128(100_000_000_000*ONE_DAI), U128(100_000_000_000*ONE_USDT), U128(100_000_000_000*ONE_USDC)], U128(1)),
        deposit = to_yocto("0.0007") 
    );
    out_come.assert_success();
    println!("{:#?}", get_logs(&out_come));
    // "Mint 299997911758886758506069372942 shares for user3, fee is 895808190595468286848457 shares",
    // "Exchange swap got 179161638119093657369691 shares, No referral fee, from add_liquidity",

    // minted_user_lpt
    assert_eq!(mft_balance_of(&pool, ":0", &user3.account_id()), 299997911758886758506069372942);
    // last_lpt_supply + minted_user_lpt + admin_fee_to_lpt
    let last_lpt_supply = last_lpt_supply + 299997911758886758506069372942 + 179161638119093657369691;
    assert_eq!(mft_total_supply(&pool, ":0"), last_lpt_supply);
    let last_share_price = pool_share_price(&pool, 0);
    println!("share_price: {}", last_share_price);
}

#[test]
fn sim_stable_max_liquidity() {
    let (root, _owner, pool, tokens) = 
        setup_stable_pool_with_liquidity(
            vec![dai(), usdt(), usdc(), 
                "dai1".to_string(), "usdt1".to_string(), "usdc1".to_string(), 
                "dai2".to_string(), "usdt2".to_string(), "usdc2".to_string(),
                ],
            vec![
                100000*ONE_DAI, 100000*ONE_USDT, 100000*ONE_USDC, 
                100000*ONE_DAI, 100000*ONE_USDT, 100000*ONE_USDC, 
                100000*ONE_DAI, 100000*ONE_USDT, 100000*ONE_USDC
            ],
            vec![18, 6, 6, 18, 6, 6, 18, 6, 6],
            25,
            10000,
        );
    let tokens = &tokens;

    // add massive liquidity (100 billion)
    let user = root.create_user("user".to_string(), to_yocto("100"));
    mint_and_deposit_token(&user, &tokens[0], &pool, 100_000_000_000*ONE_DAI);
    mint_and_deposit_token(&user, &tokens[1], &pool, 100_000_000_000*ONE_USDT);
    mint_and_deposit_token(&user, &tokens[2], &pool, 100_000_000_000*ONE_USDC);
    mint_and_deposit_token(&user, &tokens[3], &pool, 100_000_000_000*ONE_DAI);
    mint_and_deposit_token(&user, &tokens[4], &pool, 100_000_000_000*ONE_USDT);
    mint_and_deposit_token(&user, &tokens[5], &pool, 100_000_000_000*ONE_USDC);
    mint_and_deposit_token(&user, &tokens[6], &pool, 100_000_000_000*ONE_DAI);
    mint_and_deposit_token(&user, &tokens[7], &pool, 100_000_000_000*ONE_USDT);
    mint_and_deposit_token(&user, &tokens[8], &pool, 100_000_000_000*ONE_USDC);
    let out_come = call!(
        user,
        pool.add_stable_liquidity(0, vec![
            U128(100_000_000_000*ONE_DAI), U128(100_000_000_000*ONE_USDT), U128(100_000_000_000*ONE_USDC),
            U128(100_000_000_000*ONE_DAI), U128(100_000_000_000*ONE_USDT), U128(100_000_000_000*ONE_USDC),
            U128(100_000_000_000*ONE_DAI), U128(100_000_000_000*ONE_USDT), U128(100_000_000_000*ONE_USDC)
            ], U128(1)),
        deposit = to_yocto("0.0007") 
    );
    out_come.assert_success();
    println!("{:#?}", get_logs(&out_come));
    assert_eq!(mft_balance_of(&pool, ":0", &user.account_id()), 900000000000000000000000000000);
    assert_eq!(mft_total_supply(&pool, ":0"), 900000900000000000000000000000);
    let last_share_price = pool_share_price(&pool, 0);
    println!("share_price: {}", last_share_price);
}

#[test]
fn sim_stable_lp_storage() {
    let (root, _owner, pool, tokens) = 
        setup_stable_pool_with_liquidity(
            vec![
                "dai1234567890123456789012345678901234567890123456789012345678901".to_string(), 
                usdt(), 
                "cusd123456789012345678901234567890123456789012345678901234567890".to_string()
                ],
            vec![100000*ONE_DAI, 100000*ONE_USDT, 100000*ONE_CUSD],
            vec![18, 6, 24],
            5,
            240,
        );
    let tokens = &tokens;

    // user add liquidity with mono token
    let usdt_token = &tokens[1];
    let user = root.create_user("user".to_string(), to_yocto("100"));
    call!(
        user,
        usdt_token.mint(user.valid_account_id(), U128(500*ONE_USDT))
    )
    .assert_success();

    call!(
        user,
        pool.storage_deposit(None, None),
        deposit = to_yocto("0.0025")
    )
    .assert_success();

    let sb = get_storage_balance(&pool, user.valid_account_id()).unwrap();
    assert_eq!(sb.total.0, to_yocto("0.0025"));
    assert_eq!(sb.total.0 - sb.available.0, to_yocto("0.00102"));

    call!(
        user,
        usdt_token.ft_transfer_call(
            pool.valid_account_id(), 
            U128(500*ONE_USDT), 
            None, 
            "".to_string()
        ),
        deposit = 1
    )
    .assert_success();

    let sb = get_storage_balance(&pool, user.valid_account_id()).unwrap();
    assert_eq!(sb.total.0, to_yocto("0.0025"));
    assert_eq!(sb.available.0, 0);

    let out_come = call!(
        user,
        pool.add_stable_liquidity(0, vec![U128(0), U128(500*ONE_USDT), U128(0)], U128(1)),
        deposit = to_yocto("0.0007")
    );
    out_come.assert_success();
    let sb = get_storage_balance(&pool, user.valid_account_id()).unwrap();
    assert_eq!(sb.total.0, to_yocto("0.0025"));
    assert_eq!(sb.available.0, 0);

    // remove by shares
    let out_come = call!(
        user,
        pool.remove_liquidity(0, U128(300*ONE_LPT), vec![U128(1*ONE_DAI), U128(1*ONE_USDT), U128(1*ONE_CUSD)]),
        deposit = 1 
    );
    assert!(!out_come.is_ok());

    call!(
        user,
        pool.storage_deposit(None, None),
        deposit = to_yocto("0.00296")
    )
    .assert_success();
    let sb = get_storage_balance(&pool, user.valid_account_id()).unwrap();
    assert_eq!(sb.total.0, to_yocto("0.00546"));
    assert_eq!(sb.available.0, to_yocto("0.00296"));

    // remove by shares
    let out_come = call!(
        user,
        pool.remove_liquidity(0, U128(300*ONE_LPT), vec![U128(1*ONE_DAI), U128(1*ONE_USDT), U128(1*ONE_CUSD)]),
        deposit = 1 
    );
    out_come.assert_success();
    let sb = get_storage_balance(&pool, user.valid_account_id()).unwrap();
    assert_eq!(sb.total.0, to_yocto("0.00546"));
    assert_eq!(sb.available.0, 0);
}