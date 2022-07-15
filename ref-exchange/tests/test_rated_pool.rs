use std::collections::HashMap;

use near_contract_standards::fungible_token::metadata::FungibleTokenMetadata;
use near_sdk::json_types::{U128};
use near_sdk::AccountId;
use near_sdk_sim::{
    call, view, to_yocto
};

use ref_exchange::{PoolInfo, SwapAction, RatedTokenInfo};
use crate::common::utils::*;
pub mod common;

const ONE_LPT: u128 = 10u128.pow(24 as u32);
const ONE_NEAR: u128 = 10u128.pow(24 as u32);
const ONE_STNEAR: u128 = 10u128.pow(24 as u32);
const ONE_LINEAR: u128 = 10u128.pow(24 as u32);
const ONE_NEARX: u128 = 10u128.pow(24 as u32);

#[test]
fn sim_rated_swap_liquidity_two() {
    let (root, owner, pool, tokens, token_rated_contracts) = 
        setup_rated_pool(
            vec![near()],
            vec![stnear()],
            vec![24, 24],
            25,
            10000,
        );

    let stnear_contract = &token_rated_contracts[0];

    call!(
        owner,
        pool.register_rated_token(
            "STNEAR".to_string(),
            token_rated_contracts[0].valid_account_id()
        ),
        deposit = 1
    ).assert_success();

    call!(
        root,
        stnear_contract.set_price(U128(2 * 10u128.pow(24)))
    ).assert_success();

    call!(
        owner,
        pool.update_token_rate(
            stnear_contract.valid_account_id()
        ),
        deposit = 1
    ).assert_success();

    let user = root.create_user("user".to_string(), to_yocto("100"));
    mint_and_deposit_token(&user, &tokens[0], &pool, 100000*ONE_NEAR);
    mint_and_deposit_rated_token(&user, &token_rated_contracts[0], &pool, 100000*ONE_STNEAR);
    let out_come = call!(
        user,
        pool.add_stable_liquidity(0, vec![
            U128(100000*ONE_NEAR), U128(50000*ONE_STNEAR)], U128(1)),
        deposit = to_yocto("0.0007") 
    );
    out_come.assert_success();
    println!("{:#?}", get_logs(&out_come));
    assert_eq!(mft_balance_of(&pool, ":0", &user.account_id()), 200000*ONE_LPT);
    assert_eq!(mft_total_supply(&pool, ":0"), 200000*ONE_LPT);
    let last_share_price = pool_share_price(&pool, 0);
    assert_eq!(100000000, last_share_price);

    let user1 = root.create_user("user1".to_string(), to_yocto("100"));
    mint_and_deposit_token(&user1, &tokens[0], &pool, 100000*ONE_NEAR);
    mint_and_deposit_rated_token(&user1, &token_rated_contracts[0], &pool, 100000*ONE_STNEAR);
    let out_come = call!(
        user1,
        pool.add_stable_liquidity(0, vec![
            U128(100000*ONE_NEAR), U128(50000*ONE_STNEAR)], U128(1)),
        deposit = to_yocto("0.0007") 
    );
    out_come.assert_success();
    println!("{:#?}", get_logs(&out_come));
    assert_eq!(mft_balance_of(&pool, ":0", &user1.account_id()), 200000*ONE_LPT);
    assert_eq!(mft_total_supply(&pool, ":0"), 400000*ONE_LPT);
    let last_share_price = pool_share_price(&pool, 0);
    assert_eq!(100000000, last_share_price);

    let out_come = call!(
        user1,
        pool.remove_liquidity(0, U128(200000*ONE_LPT), vec![U128(1*ONE_NEAR), U128(1*ONE_STNEAR)]),
        deposit = 1 
    );
    out_come.assert_success();
    println!("{:#?}", get_logs(&out_come));
    assert_eq!(mft_balance_of(&pool, ":0", &user1.account_id()), 0);
    assert_eq!(mft_total_supply(&pool, ":0"), 200000*ONE_LPT);
    assert_eq!(100000000, pool_share_price(&pool, 0));

    let out_come = call!(
        user,
        pool.remove_liquidity(0, U128(200000*ONE_LPT), vec![U128(1*ONE_NEAR), U128(1*ONE_STNEAR)]),
        deposit = 1 
    );
    assert_eq!(get_error_count(&out_come), 1);
    assert!(get_error_status(&out_come).contains("E69: pool reserved token balance less than MIN_RESERVE"));

}

#[test]
fn sim_rated_swap_liquidity_three_one_rated() {
    let (root, owner, pool, tokens, token_rated_contracts) = 
        setup_rated_pool(
            vec![near()],
            vec![stnear(), linear()],
            vec![24, 24, 24],
            25,
            10000,
        );

    let stnear_contract = &token_rated_contracts[0];

    call!(
        owner,
        pool.register_rated_token(
            "STNEAR".to_string(),
            token_rated_contracts[0].valid_account_id()
        ),
        deposit = 1
    ).assert_success();

    call!(
        root,
        stnear_contract.set_price(U128(2 * 10u128.pow(24)))
    ).assert_success();

    call!(
        owner,
        pool.update_token_rate(
            stnear_contract.valid_account_id()
        ),
        deposit = 1
    ).assert_success();

    let user = root.create_user("user".to_string(), to_yocto("100"));
    mint_and_deposit_token(&user, &tokens[0], &pool, 100000*ONE_NEAR);
    mint_and_deposit_rated_token(&user, &token_rated_contracts[0], &pool, 100000*ONE_STNEAR);
    mint_and_deposit_rated_token(&user, &token_rated_contracts[1], &pool, 100000*ONE_LINEAR);
    let out_come = call!(
        user,
        pool.add_stable_liquidity(0, vec![
            U128(100000*ONE_NEAR), U128(50000*ONE_STNEAR), U128(100000*ONE_LINEAR)], U128(1)),
        deposit = to_yocto("0.0007") 
    );
    out_come.assert_success();
    println!("{:#?}", get_logs(&out_come));
    assert_eq!(mft_balance_of(&pool, ":0", &user.account_id()), 300000*ONE_LPT);
    assert_eq!(mft_total_supply(&pool, ":0"), 300000*ONE_LPT);
    let last_share_price = pool_share_price(&pool, 0);
    assert_eq!(100000000, last_share_price);

    let user1 = root.create_user("user1".to_string(), to_yocto("100"));
    mint_and_deposit_token(&user1, &tokens[0], &pool, 100000*ONE_NEAR);
    mint_and_deposit_rated_token(&user1, &token_rated_contracts[0], &pool, 100000*ONE_STNEAR);
    mint_and_deposit_rated_token(&user1, &token_rated_contracts[1], &pool, 100000*ONE_LINEAR);
    let out_come = call!(
        user1,
        pool.add_stable_liquidity(0, vec![
            U128(100000*ONE_NEAR), U128(50000*ONE_STNEAR), U128(100000*ONE_LINEAR)], U128(1)),
        deposit = to_yocto("0.0007") 
    );
    out_come.assert_success();
    println!("{:#?}", get_logs(&out_come));
    assert_eq!(mft_balance_of(&pool, ":0", &user1.account_id()), 300000*ONE_LPT);
    assert_eq!(mft_total_supply(&pool, ":0"), 600000*ONE_LPT);
    let last_share_price = pool_share_price(&pool, 0);
    assert_eq!(100000000, last_share_price);

    let out_come = call!(
        user1,
        pool.remove_liquidity(0, U128(300000*ONE_LPT), vec![U128(1*ONE_NEAR), U128(1*ONE_STNEAR), U128(1*ONE_LINEAR)]),
        deposit = 1 
    );
    out_come.assert_success();
    println!("{:#?}", get_logs(&out_come));
    assert_eq!(mft_balance_of(&pool, ":0", &user1.account_id()), 0);
    assert_eq!(mft_total_supply(&pool, ":0"), 300000*ONE_LPT);
    assert_eq!(100000000, pool_share_price(&pool, 0));
}

#[test]
fn sim_rated_swap_liquidity_three_two_rated() {
    let (root, owner, pool, tokens, token_rated_contracts) = 
        setup_rated_pool(
            vec![near()],
            vec![stnear(), linear()],
            vec![24, 24, 24],
            25,
            10000,
        );

    let stnear_contract = &token_rated_contracts[0];
    let linear_contract = &token_rated_contracts[1];

    call!(
        owner,
        pool.register_rated_token(
            "STNEAR".to_string(),
            token_rated_contracts[0].valid_account_id()
        ),
        deposit = 1
    ).assert_success();

    call!(
        owner,
        pool.register_rated_token(
            "LINEAR".to_string(),
            token_rated_contracts[1].valid_account_id()
        ),
        deposit = 1
    ).assert_success();

    call!(
        root,
        stnear_contract.set_price(U128(2 * 10u128.pow(24)))
    ).assert_success();

    call!(
        root,
        linear_contract.set_price(U128(4 * 10u128.pow(24)))
    ).assert_success();

    call!(
        owner,
        pool.update_token_rate(
            stnear_contract.valid_account_id()
        ),
        deposit = 1
    ).assert_success();

    call!(
        owner,
        pool.update_token_rate(
            linear_contract.valid_account_id()
        ),
        deposit = 1
    ).assert_success();


    let user = root.create_user("user".to_string(), to_yocto("100"));
    mint_and_deposit_token(&user, &tokens[0], &pool, 100000*ONE_NEAR);
    mint_and_deposit_rated_token(&user, &token_rated_contracts[0], &pool, 100000*ONE_STNEAR);
    mint_and_deposit_rated_token(&user, &token_rated_contracts[1], &pool, 100000*ONE_LINEAR);
    let out_come = call!(
        user,
        pool.add_stable_liquidity(0, vec![
            U128(100000*ONE_NEAR), U128(50000*ONE_STNEAR), U128(25000*ONE_LINEAR)], U128(1)),
        deposit = to_yocto("0.0007") 
    );
    out_come.assert_success();
    println!("{:#?}", get_logs(&out_come));
    assert_eq!(mft_balance_of(&pool, ":0", &user.account_id()), 300000*ONE_LPT);
    assert_eq!(mft_total_supply(&pool, ":0"), 300000*ONE_LPT);
    assert_eq!(100000000, pool_share_price(&pool, 0));

    let user1 = root.create_user("user1".to_string(), to_yocto("100"));
    mint_and_deposit_token(&user1, &tokens[0], &pool, 100000*ONE_NEAR);
    mint_and_deposit_rated_token(&user1, &token_rated_contracts[0], &pool, 100000*ONE_STNEAR);
    mint_and_deposit_rated_token(&user1, &token_rated_contracts[1], &pool, 100000*ONE_LINEAR);
    let out_come = call!(
        user1,
        pool.add_stable_liquidity(0, vec![
            U128(100000*ONE_NEAR), U128(50000*ONE_STNEAR), U128(25000*ONE_LINEAR)], U128(1)),
        deposit = to_yocto("0.0007") 
    );
    out_come.assert_success();
    println!("{:#?}", get_logs(&out_come));
    assert_eq!(mft_balance_of(&pool, ":0", &user1.account_id()), 300000*ONE_LPT);
    assert_eq!(mft_total_supply(&pool, ":0"), 600000*ONE_LPT);
    assert_eq!(100000000, pool_share_price(&pool, 0));

    let out_come = call!(
        user1,
        pool.remove_liquidity(0, U128(300000*ONE_LPT), vec![U128(1*ONE_NEAR), U128(1*ONE_STNEAR), U128(1*ONE_LINEAR)]),
        deposit = 1 
    );
    out_come.assert_success();
    println!("{:#?}", get_logs(&out_come));
    assert_eq!(mft_balance_of(&pool, ":0", &user1.account_id()), 0);
    assert_eq!(mft_total_supply(&pool, ":0"), 300000*ONE_LPT);
    assert_eq!(100000000, pool_share_price(&pool, 0));
}

#[test]
fn sim_rated_swap_two_no_rated() {
    let (root, _owner, pool, tokens, _token_rated_contracts) = 
        setup_rated_pool_with_liquidity(
            vec![near()],
            vec![stnear()],
            vec![100000*ONE_NEAR],
            vec![100000*ONE_STNEAR],
            vec![24, 24],
            25,
            10000,
        );
    assert_eq!(
        view!(pool.get_pool(0)).unwrap_json::<PoolInfo>(),
        PoolInfo {
            pool_kind: "RATED_SWAP".to_string(),
            amp: 10000,
            token_account_ids: vec![near(), stnear()],
            amounts: vec![U128(100000*ONE_NEAR), U128(100000*ONE_STNEAR)],
            total_fee: 25,
            shares_total_supply: U128(200000*ONE_LPT),
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
        200000*ONE_LPT
    );
    let balances = view!(pool.get_deposits(root.valid_account_id()))
        .unwrap_json::<HashMap<AccountId, U128>>();
    let balances = balances.values().cloned().collect::<Vec<_>>();
    assert_eq!(balances, vec![U128(0), U128(0)]);

    let c = tokens.get(0).unwrap();
    call!(
        root,
        c.ft_transfer_call(pool.valid_account_id(), U128(ONE_NEAR), None, "".to_string()),
        deposit = 1
    )
    .assert_success();

    let out_come = call!(
        root,
        pool.swap(
            vec![SwapAction {
                pool_id: 0,
                token_in: near(),
                amount_in: Some(U128(ONE_NEAR)),
                token_out: stnear(),
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
    assert_eq!(balances[&near()].0, 0);
    assert_eq!(balances[&stnear()].0, 997499999501274936452669);

    assert_eq!(
        view!(pool.get_pool(0)).unwrap_json::<PoolInfo>(),
        PoolInfo {
            pool_kind: "RATED_SWAP".to_string(),
            amp: 10000,
            token_account_ids: vec![near(), stnear()],
            amounts: vec![U128(100001*ONE_NEAR), U128(99999*ONE_STNEAR+2500000498725063547331)],
            total_fee: 25,
            shares_total_supply: U128(200000*ONE_LPT + 499999994999720058346),
        }
    );
}

#[test]
fn sim_rated_swap_rate_one_with_fee() {
    let (root, owner, pool, tokens, token_rated_contracts) = 
        setup_rated_pool_with_liquidity(
            vec![near()],
            vec![stnear()],
            vec![100000*ONE_NEAR],
            vec![100000*ONE_STNEAR],
            vec![24, 24],
            25,
            10000,
        );
    let stnear_contract = &token_rated_contracts[0];
    assert_eq!(
        view!(pool.get_pool(0)).unwrap_json::<PoolInfo>(),
        PoolInfo {
            pool_kind: "RATED_SWAP".to_string(),
            amp: 10000,
            token_account_ids: vec![near(), stnear()],
            amounts: vec![U128(100000*ONE_NEAR), U128(100000*ONE_STNEAR)],
            total_fee: 25,
            shares_total_supply: U128(200000*ONE_LPT),
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
        200000*ONE_LPT
    );
    let balances = view!(pool.get_deposits(root.valid_account_id()))
        .unwrap_json::<HashMap<AccountId, U128>>();
    let balances = balances.values().cloned().collect::<Vec<_>>();
    assert_eq!(balances, vec![U128(0), U128(0)]);

    let c = tokens.get(0).unwrap();
    call!(
        root,
        c.ft_transfer_call(pool.valid_account_id(), U128(ONE_NEAR), None, "".to_string()),
        deposit = 1
    )
    .assert_success();

    call!(
        owner,
        pool.register_rated_token(
            "STNEAR".to_string(),
            token_rated_contracts[0].valid_account_id()
        ),
        deposit = 1
    ).assert_success();

    call!(
        owner,
        pool.update_token_rate(
            stnear_contract.valid_account_id()
        ),
        deposit = 1
    ).assert_success();

    let rated_infos = view!(pool.list_rated_tokens()).unwrap_json::<HashMap<String, RatedTokenInfo>>();

    println!("{:?}", rated_infos);

    let out_come = call!(
        root,
        pool.swap(
            vec![SwapAction {
                pool_id: 0,
                token_in: near(),
                amount_in: Some(U128(ONE_NEAR)),
                token_out: stnear(),
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
    assert_eq!(balances[&near()].0, 0);
    assert_eq!(balances[&stnear()].0, 997499999501274936452669);

    assert_eq!(
        view!(pool.get_pool(0)).unwrap_json::<PoolInfo>(),
        PoolInfo {
            pool_kind: "RATED_SWAP".to_string(),
            amp: 10000,
            token_account_ids: vec![near(), stnear()],
            amounts: vec![U128(100001*ONE_NEAR), U128(99999*ONE_STNEAR+2500000498725063547331)],
            total_fee: 25,
            shares_total_supply: U128(200000*ONE_LPT + 499999994999720058346),
        }
    );
    
}

#[test]
fn sim_rated_swap_rate_one_no_fee() {
    let (root, owner, pool, tokens, token_rated_contracts) = 
        setup_rated_pool_with_liquidity(
            vec![near()],
            vec![stnear()],
            vec![100000*ONE_NEAR],
            vec![100000*ONE_STNEAR],
            vec![24, 24],
            0,
            10000,
        );
    let stnear_contract = &token_rated_contracts[0];
    assert_eq!(
        view!(pool.get_pool(0)).unwrap_json::<PoolInfo>(),
        PoolInfo {
            pool_kind: "RATED_SWAP".to_string(),
            amp: 10000,
            token_account_ids: vec![near(), stnear()],
            amounts: vec![U128(100000*ONE_NEAR), U128(100000*ONE_STNEAR)],
            total_fee: 0,
            shares_total_supply: U128(200000*ONE_LPT),
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
        200000*ONE_LPT
    );
    let balances = view!(pool.get_deposits(root.valid_account_id()))
        .unwrap_json::<HashMap<AccountId, U128>>();
    let balances = balances.values().cloned().collect::<Vec<_>>();
    assert_eq!(balances, vec![U128(0), U128(0)]);

    let c = tokens.get(0).unwrap();
    call!(
        root,
        c.ft_transfer_call(pool.valid_account_id(), U128(ONE_NEAR), None, "".to_string()),
        deposit = 1
    )
    .assert_success();

    call!(
        owner,
        pool.register_rated_token(
            "STNEAR".to_string(),
            token_rated_contracts[0].valid_account_id()
        ),
        deposit = 1
    ).assert_success();

    call!(
        owner,
        pool.update_token_rate(
            stnear_contract.valid_account_id()
        ),
        deposit = 1
    ).assert_success();

    let rated_infos = view!(pool.list_rated_tokens()).unwrap_json::<HashMap<String, RatedTokenInfo>>();

    println!("{:?}", rated_infos);

    let out_come = call!(
        root,
        pool.swap(
            vec![SwapAction {
                pool_id: 0,
                token_in: near(),
                amount_in: Some(U128(ONE_NEAR)),
                token_out: stnear(),
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
    assert_eq!(balances[&near()].0, 0);
    assert_eq!(balances[&stnear()].0, 999999999500024998950044);

    assert_eq!(
        view!(pool.get_pool(0)).unwrap_json::<PoolInfo>(),
        PoolInfo {
            pool_kind: "RATED_SWAP".to_string(),
            amp: 10000,
            token_account_ids: vec![near(), stnear()],
            amounts: vec![U128(100001*ONE_NEAR), U128(99999*ONE_STNEAR+499975001049956)],
            total_fee: 0,
            shares_total_supply: U128(200000*ONE_LPT),
        }
    );
}

#[test]
fn sim_rated_swap_three_no_rated() {
    let (root, _owner, pool, tokens, _token_rated_contracts) = 
        setup_rated_pool_with_liquidity(
            vec![near()],
            vec![stnear(), linear()],
            vec![100000*ONE_NEAR],
            vec![100000*ONE_STNEAR, 100000*ONE_LINEAR],
            vec![24, 24, 24],
            25,
            10000,
        );
    let tokens = &tokens;
    assert_eq!(
        view!(pool.get_pool(0)).unwrap_json::<PoolInfo>(),
        PoolInfo {
            pool_kind: "RATED_SWAP".to_string(),
            amp: 10000,
            token_account_ids: vec![near(), stnear(), linear()],
            amounts: vec![U128(100000*ONE_NEAR), U128(100000*ONE_STNEAR), U128(100000*ONE_LINEAR)],
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
        c.ft_transfer_call(pool.valid_account_id(), U128(2 * ONE_NEAR), None, "".to_string()),
        deposit = 1
    )
    .assert_success();

    let out_come = call!(
        root,
        pool.swap(
            vec![SwapAction {
                pool_id: 0,
                token_in: near(),
                amount_in: Some(U128(ONE_NEAR)),
                token_out: stnear(),
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
                token_in: near(),
                amount_in: Some(U128(ONE_NEAR)),
                token_out: linear(),
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
    assert_eq!(balances[&near()].0, 0);
    assert_eq!(balances[&stnear()].0, 997499999889167898135697);
    assert_eq!(balances[&linear()].0, 997499999778338010999825);

    assert_eq!(
        view!(pool.get_pool(0)).unwrap_json::<PoolInfo>(),
        PoolInfo {
            pool_kind: "RATED_SWAP".to_string(),
            amp: 10000,
            token_account_ids: vec![near(), stnear(), linear()],
            amounts: vec![U128(100002*ONE_NEAR), U128(99999*ONE_STNEAR+2500000110832101864303), U128(99999*ONE_STNEAR+2500000221661989000175)],
            total_fee: 25,
            shares_total_supply: U128(300000*ONE_LPT + 499999996666583725184 + 499999993277742563392),
        }
    );
}

#[test]
fn sim_rated_swap() {
    let (root, owner, pool, tokens, token_rated_contracts) = 
        setup_rated_pool_with_liquidity(
            vec![near()],
            vec![stnear(), linear()],
            vec![100000*ONE_NEAR],
            vec![100000*ONE_STNEAR, 100000*ONE_LINEAR],
            vec![24, 24, 24],
            25,
            10000,
        );

    let stnear_contract = &token_rated_contracts[0];
    let linear_contract = &token_rated_contracts[1];

    assert_eq!(view!(pool.list_rated_tokens()).unwrap_json::<HashMap<String, RatedTokenInfo>>().len(), 0);
    
    call!(
        owner,
        pool.register_rated_token(
            "STNEAR".to_string(),
            token_rated_contracts[0].valid_account_id()
        ),
        deposit = 1
    ).assert_success();

    let rated_infos = view!(pool.list_rated_tokens()).unwrap_json::<HashMap<String, RatedTokenInfo>>();
    assert_eq!(1, rated_infos.len());
    assert_eq!("STNEAR".to_string(), rated_infos.get(&stnear_contract.account_id()).unwrap().rate_type);
    assert_eq!(10u128.pow(24), rated_infos.get(&stnear_contract.account_id()).unwrap().rate_price.0);
    assert_eq!(0, rated_infos.get(&stnear_contract.account_id()).unwrap().last_update_ts.0);
    assert_eq!(false, rated_infos.get(&stnear_contract.account_id()).unwrap().is_valid);

    let out_come = call!(
        owner,
        pool.register_rated_token(
            "STNEAR1".to_string(),
            token_rated_contracts[0].valid_account_id()
        ),
        deposit = 1
    );
    assert_eq!(get_error_count(&out_come), 1);
    assert!(get_error_status(&out_come).contains("E127: Invalid rate type"));

    let out_come = call!(
        owner,
        pool.register_rated_token(
            "STNEAR".to_string(),
            token_rated_contracts[0].valid_account_id()
        ),
        deposit = 1
    );
    assert_eq!(get_error_count(&out_come), 1);
    assert!(get_error_status(&out_come).contains("Rated token stnear already exist"));

    call!(
        owner,
        pool.register_rated_token(
            "LINEAR".to_string(),
            token_rated_contracts[1].valid_account_id()
        ),
        deposit = 1
    ).assert_success();

    let rated_infos = view!(pool.list_rated_tokens()).unwrap_json::<HashMap<String, RatedTokenInfo>>();
    assert_eq!(2, rated_infos.len());
    assert_eq!("LINEAR".to_string(), rated_infos.get(&linear_contract.account_id()).unwrap().rate_type);
    assert_eq!(10u128.pow(24), rated_infos.get(&linear_contract.account_id()).unwrap().rate_price.0);
    assert_eq!(0, rated_infos.get(&linear_contract.account_id()).unwrap().last_update_ts.0);
    assert_eq!(false, rated_infos.get(&linear_contract.account_id()).unwrap().is_valid);

    let c = tokens.get(0).unwrap();
    call!(
        root,
        c.ft_transfer_call(pool.valid_account_id(), U128(2 * ONE_NEAR), None, "".to_string()),
        deposit = 1
    )
    .assert_success();

    // Rates expired
    let out_come = call!(
        root,
        pool.swap(
            vec![SwapAction {
                pool_id: 0,
                token_in: near(),
                amount_in: Some(U128(ONE_NEAR)),
                token_out: linear(),
                min_amount_out: U128(1)
            }],
            None
        ),
        deposit = 1
    );
    assert_eq!(get_error_count(&out_come), 1);
    assert!(get_error_status(&out_come).contains("E120: Rates expired"));
    
    // update stnear price
    call!(
        owner,
        pool.update_token_rate(
            stnear_contract.valid_account_id()
        ),
        deposit = 1
    ).assert_success();

    // Rates expired too, linear expired
    let out_come = call!(
        root,
        pool.swap(
            vec![SwapAction {
                pool_id: 0,
                token_in: near(),
                amount_in: Some(U128(ONE_NEAR)),
                token_out: linear(),
                min_amount_out: U128(1)
            }],
            None
        ),
        deposit = 1
    );
    assert_eq!(get_error_count(&out_come), 1);
    assert!(get_error_status(&out_come).contains("E120: Rates expired"));
    
    // update linear price
    call!(
        owner,
        pool.update_token_rate(
            linear_contract.valid_account_id()
        ),
        deposit = 1
    ).assert_success();
    
    assert_eq!(997499999889167898135697, view!(pool.get_return(0, to_va(near()), U128(ONE_NEAR), to_va(stnear()))).unwrap_json::<U128>().0);

    call!(
        root,
        stnear_contract.set_price(U128(2 * 10u128.pow(24)))
    ).assert_success();

    call!(
        owner,
        pool.update_token_rate(
            stnear_contract.valid_account_id()
        ),
        deposit = 1
    ).assert_success();

    assert_eq!(498754378484693050587240, view!(pool.get_return(0, to_va(near()), U128(ONE_NEAR), to_va(stnear()))).unwrap_json::<U128>().0);
}

#[test]
fn sim_rated_swap_register_unregister() {
    let (_root, owner, pool, _tokens, token_rated_contracts) = 
        setup_rated_pool_with_liquidity(
            vec![near()],
            vec![stnear(), linear()],
            vec![100000*ONE_NEAR],
            vec![100000*ONE_STNEAR, 100000*ONE_LINEAR],
            vec![24, 24, 24],
            25,
            10000,
        );

    let stnear_contract = &token_rated_contracts[0];
    let linear_contract = &token_rated_contracts[1];

    assert_eq!(view!(pool.list_rated_tokens()).unwrap_json::<HashMap<String, RatedTokenInfo>>().len(), 0);
    
    call!(
        owner,
        pool.register_rated_token(
            "STNEAR".to_string(),
            token_rated_contracts[0].valid_account_id()
        ),
        deposit = 1
    ).assert_success();

    let rated_infos = view!(pool.list_rated_tokens()).unwrap_json::<HashMap<String, RatedTokenInfo>>();
    assert_eq!(1, rated_infos.len());
    assert_eq!("STNEAR".to_string(), rated_infos.get(&stnear_contract.account_id()).unwrap().rate_type);
    assert_eq!(10u128.pow(24), rated_infos.get(&stnear_contract.account_id()).unwrap().rate_price.0);
    assert_eq!(0, rated_infos.get(&stnear_contract.account_id()).unwrap().last_update_ts.0);
    assert_eq!(false, rated_infos.get(&stnear_contract.account_id()).unwrap().is_valid);

    let out_come = call!(
        owner,
        pool.register_rated_token(
            "STNEAR1".to_string(),
            token_rated_contracts[0].valid_account_id()
        ),
        deposit = 1
    );
    assert_eq!(get_error_count(&out_come), 1);
    assert!(get_error_status(&out_come).contains("E127: Invalid rate type"));

    let out_come = call!(
        owner,
        pool.register_rated_token(
            "STNEAR".to_string(),
            token_rated_contracts[0].valid_account_id()
        ),
        deposit = 1
    );
    assert_eq!(get_error_count(&out_come), 1);
    assert!(get_error_status(&out_come).contains("Rated token stnear already exist"));

    call!(
        owner,
        pool.register_rated_token(
            "LINEAR".to_string(),
            token_rated_contracts[1].valid_account_id()
        ),
        deposit = 1
    ).assert_success();

    let rated_infos = view!(pool.list_rated_tokens()).unwrap_json::<HashMap<String, RatedTokenInfo>>();
    assert_eq!(2, rated_infos.len());
    assert_eq!("LINEAR".to_string(), rated_infos.get(&linear_contract.account_id()).unwrap().rate_type);
    assert_eq!(10u128.pow(24), rated_infos.get(&linear_contract.account_id()).unwrap().rate_price.0);
    assert_eq!(0, rated_infos.get(&linear_contract.account_id()).unwrap().last_update_ts.0);
    assert_eq!(false, rated_infos.get(&linear_contract.account_id()).unwrap().is_valid);

    let out_come = call!(
        owner,
        pool.unregister_rated_token(
            token_rated_contracts[1].valid_account_id()
        ),
        deposit = 1
    );
    out_come.assert_success();
    assert!(get_logs(&out_come).contains(&"Rated token linear removed.".to_string()));
    let rated_infos = view!(pool.list_rated_tokens()).unwrap_json::<HashMap<String, RatedTokenInfo>>();
    assert_eq!(1, rated_infos.len());

    let out_come = call!(
        owner,
        pool.unregister_rated_token(
            token_rated_contracts[1].valid_account_id()
        ),
        deposit = 1
    );
    out_come.assert_success();
    assert!(get_logs(&out_come).contains(&"Rated token linear not exist in rate list.".to_string()));
    let rated_infos = view!(pool.list_rated_tokens()).unwrap_json::<HashMap<String, RatedTokenInfo>>();
    assert_eq!(1, rated_infos.len());

    let out_come = call!(
        owner,
        pool.unregister_rated_token(
            token_rated_contracts[0].valid_account_id()
        ),
        deposit = 1
    );
    out_come.assert_success();
    assert!(get_logs(&out_come).contains(&"Rated token stnear removed.".to_string()));
    let rated_infos = view!(pool.list_rated_tokens()).unwrap_json::<HashMap<String, RatedTokenInfo>>();
    assert_eq!(0, rated_infos.len());
}

#[test]
fn sim_rated_swap_out_zero() {
    let (root, owner, pool, tokens, token_rated_contracts) = 
        setup_rated_pool_with_liquidity(
            vec![near()],
            vec![stnear()],
            vec![100000*ONE_NEAR],
            vec![100000*ONE_STNEAR],
            vec![24, 24],
            0,
            1000,
        );

    let stnear_contract = &token_rated_contracts[0];

    let c = tokens.get(0).unwrap();
    call!(
        root,
        c.ft_transfer_call(pool.valid_account_id(), U128(ONE_NEAR), None, "".to_string()),
        deposit = 1
    )
    .assert_success();

    call!(
        owner,
        pool.register_rated_token(
            "STNEAR".to_string(),
            token_rated_contracts[0].valid_account_id()
        ),
        deposit = 1
    ).assert_success();

    call!(
        owner,
        pool.update_token_rate(
            stnear_contract.valid_account_id()
        ),
        deposit = 1
    ).assert_success();

    let out_come = call!(
        root,
        pool.swap(
            vec![SwapAction {
                pool_id: 0,
                token_in: near(),
                amount_in: Some(U128(0)),
                token_out: stnear(),
                min_amount_out: U128(0)
            }],
            None
        ),
        deposit = 1
    );

    assert_eq!(out_come.unwrap_json::<U128>().0, 0);

    let out_come = call!(
        root,
        pool.swap(
            vec![SwapAction {
                pool_id: 0,
                token_in: near(),
                amount_in: Some(U128(1)),
                token_out: stnear(),
                min_amount_out: U128(0)
            }],
            None
        ),
        deposit = 1
    );

    assert_eq!(out_come.unwrap_json::<U128>().0, 0);
}

#[test]
fn sim_rated_swap_lp() {
    let (root, _owner, pool, tokens, token_rated_contracts) = 
        setup_rated_pool_with_liquidity(
            vec![near()],
            vec![stnear(), linear()],
            vec![100000*ONE_NEAR],
            vec![100000*ONE_STNEAR, 100000*ONE_LINEAR],
            vec![24, 24, 24],
            25,
            10000,
        );
    
    let last_share_price = pool_share_price(&pool, 0);
    let last_lpt_supply = mft_total_supply(&pool, ":0");

    // add more liquidity with balanced tokens
    let user1 = root.create_user("user1".to_string(), to_yocto("100"));
    mint_and_deposit_token(&user1, &tokens[0], &pool, 500*ONE_NEAR);
    mint_and_deposit_rated_token(&user1, &token_rated_contracts[0], &pool, 500*ONE_STNEAR);
    mint_and_deposit_rated_token(&user1, &token_rated_contracts[1], &pool, 500*ONE_LINEAR);
    let out_come = call!(
        user1,
        pool.add_stable_liquidity(0, vec![U128(500*ONE_NEAR), U128(500*ONE_STNEAR), U128(500*ONE_LINEAR)], U128(1)),
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
        pool.remove_liquidity(0, U128(300*ONE_LPT), vec![U128(1*ONE_NEAR), U128(1*ONE_STNEAR), U128(1*ONE_LINEAR)]),
        deposit = 1 
    );
    out_come.assert_success();
    println!("{:#?}", get_logs(&out_come));
    assert_eq!(mft_balance_of(&pool, ":0", &user1.account_id()), 1200*ONE_LPT);
    let balances = view!(pool.get_deposits(user1.valid_account_id()))
        .unwrap_json::<HashMap<AccountId, U128>>();
    assert_eq!(balances[&near()].0, 100*ONE_NEAR);
    assert_eq!(balances[&stnear()].0, 100*ONE_STNEAR);
    assert_eq!(balances[&linear()].0, 100*ONE_LINEAR);
    assert_eq!(pool_share_price(&pool, 0), last_share_price);
    assert_eq!(mft_total_supply(&pool, ":0"), last_lpt_supply - 300*ONE_LPT);
    let last_lpt_supply = last_lpt_supply - 300*ONE_LPT;

    // add more liquidity with imba tokens
    let user2 = root.create_user("user2".to_string(), to_yocto("100"));
    mint_and_deposit_token(&user2, &tokens[0], &pool, 100*ONE_NEAR);
    mint_and_deposit_rated_token(&user2, &token_rated_contracts[0], &pool, 200*ONE_STNEAR);
    mint_and_deposit_rated_token(&user2, &token_rated_contracts[1], &pool, 400*ONE_LINEAR);
    let out_come = call!(
        user2,
        pool.add_stable_liquidity(0, vec![U128(100*ONE_NEAR), U128(200*ONE_STNEAR), U128(400*ONE_LINEAR)], U128(1)),
        deposit = to_yocto("0.0014")  // 0.0007 for one lp and double it for admin fee
    );
    out_come.assert_success();
    println!("{:#?}", get_logs(&out_come));

    assert_eq!(
        view!(pool.get_pool(0)).unwrap_json::<PoolInfo>(),
        PoolInfo {
            pool_kind: "RATED_SWAP".to_string(),
            amp: 10000,
            token_account_ids: vec![near(), stnear(), linear()],
            amounts: vec![U128(100500*ONE_NEAR), U128(100600*ONE_STNEAR), U128(100800*ONE_LINEAR)],
            total_fee: 25,
            shares_total_supply: U128(301200*ONE_LPT+699699997426210330024139704+47999999735823255901269),
        }
    );
    assert_eq!(mft_balance_of(&pool, ":0", &user1.account_id()), 1200*ONE_LPT);
    assert_eq!(mft_balance_of(&pool, ":0", &user2.account_id()), 699699997426210330024139704);
    assert!(pool_share_price(&pool, 0) > last_share_price);
    let last_share_price = pool_share_price(&pool, 0);
    assert_eq!(mft_total_supply(&pool, ":0"), last_lpt_supply + 699699997426210330024139704 + 47999999735823255901269);
    let last_lpt_supply = last_lpt_supply + 699699997426210330024139704 + 47999999735823255901269;

    // remove by tokens
    let out_come = call!(
        user1,
        pool.remove_liquidity_by_tokens(0, vec![U128(1*ONE_NEAR), U128(500*ONE_STNEAR), U128(1*ONE_LINEAR)], U128(550*ONE_LPT)),
        deposit = 1 
    );
    out_come.assert_success();
    println!("{:#?}", get_logs(&out_come));
    assert_eq!(mft_balance_of(&pool, ":0", &user1.account_id()), 697401508719920229452420705);
    let balances = view!(pool.get_deposits(user1.valid_account_id()))
        .unwrap_json::<HashMap<AccountId, U128>>();
    assert_eq!(balances[&near()].0, 101*ONE_NEAR);
    assert_eq!(balances[&stnear()].0, 600*ONE_STNEAR);
    assert_eq!(balances[&linear()].0, 101*ONE_LINEAR);
    assert_eq!(
        view!(pool.get_pool(0)).unwrap_json::<PoolInfo>(),
        PoolInfo {
            pool_kind: "RATED_SWAP".to_string(),
            amp: 10000,
            token_account_ids: vec![near(), stnear(), linear()],
            amounts: vec![U128(100499*ONE_NEAR), U128(100100*ONE_STNEAR), U128(100799*ONE_LINEAR)],
            total_fee: 25,
            shares_total_supply: U128(last_lpt_supply-502598491280079770547579295+95823884420348155736299),
        }
    );
    assert_eq!(mft_balance_of(&pool, ":0", &user1.account_id()), 1200*ONE_LPT-502598491280079770547579295);
    assert_eq!(mft_balance_of(&pool, ":0", &user2.account_id()), 699699997426210330024139704);
    assert!(pool_share_price(&pool, 0) > last_share_price);
    let last_share_price = pool_share_price(&pool, 0);
    let last_lpt_supply = last_lpt_supply - 502598491280079770547579295 + 95823884420348155736299;

    // tansfer some to other
    let out_come = call!(
        user1,
        pool.mft_transfer(":0".to_string(), user2.valid_account_id(), U128(100*ONE_LPT), None),
        deposit = 1 
    );
    out_come.assert_success();
    println!("{:#?}", get_logs(&out_come));
    assert_eq!(mft_balance_of(&pool, ":0", &user1.account_id()), 1100*ONE_LPT-502598491280079770547579295);
    assert_eq!(mft_balance_of(&pool, ":0", &user2.account_id()), 799699997426210330024139704);
    assert_eq!(pool_share_price(&pool, 0), last_share_price);
    assert_eq!(mft_total_supply(&pool, ":0"), last_lpt_supply);

    // other remove by shares trigger slippage
    let out_come = call!(
        user2,
        pool.remove_liquidity(0, U128(300*ONE_LPT), vec![U128(1*ONE_NEAR), U128(298*ONE_STNEAR), U128(1*ONE_LINEAR)]),
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
        pool.remove_liquidity_by_tokens(0, vec![U128(1*ONE_NEAR), U128(298*ONE_STNEAR), U128(1*ONE_LINEAR)], U128(300*ONE_LPT)),
        deposit = 1 
    );
    assert!(!out_come.is_ok());
    let ex_status = format!("{:?}", out_come.promise_errors()[0].as_ref().unwrap().status());
    assert!(ex_status.contains("E68: slippage error"));
    assert_eq!(pool_share_price(&pool, 0), last_share_price);
    assert_eq!(mft_total_supply(&pool, ":0"), last_lpt_supply);

    // user2 remove by share
    assert_eq!(mft_balance_of(&pool, ":0", &user1.account_id()), 1100*ONE_LPT-502598491280079770547579295);
    assert_eq!(mft_balance_of(&pool, ":0", &user2.account_id()), 799699997426210330024139704);
    let out_come = call!(
        user2,
        pool.remove_liquidity(0, U128(300*ONE_LPT), vec![U128(1*ONE_NEAR), U128(1*ONE_STNEAR), U128(1*ONE_LINEAR)]),
        deposit = 1 
    );
    out_come.assert_success();
    println!("{:#?}", get_logs(&out_come));
    assert_eq!(mft_balance_of(&pool, ":0", &user1.account_id()), 1100*ONE_LPT-502598491280079770547579295);
    assert_eq!(mft_balance_of(&pool, ":0", &user2.account_id()), 499699997426210330024139704);
    assert_eq!(pool_share_price(&pool, 0), last_share_price);
    assert_eq!(mft_total_supply(&pool, ":0"), last_lpt_supply-300*ONE_LPT);
    let last_lpt_supply = last_lpt_supply - 300*ONE_LPT;
    
    // user2 remove by tokens
    let out_come = call!(
        user2,
        pool.remove_liquidity_by_tokens(0, vec![U128(498*ONE_NEAR), U128(0*ONE_STNEAR), U128(0*ONE_LINEAR)], U128(499*ONE_LPT)),
        deposit = 1 
    );
    out_come.assert_success();
    println!("{:#?}", get_logs(&out_come));
    assert_eq!(mft_balance_of(&pool, ":0", &user1.account_id()), 1100*ONE_LPT-502598491280079770547579295);
    // previous lpt - removed lpt
    assert_eq!(mft_balance_of(&pool, ":0", &user2.account_id()), 499699997426210330024139704-498596260777261245962554635);
    // last_lpt_supply - removed lpt + admin_fee_to_lpt
    let last_lpt_supply = last_lpt_supply - 498596260777261245962554635 + 95600058313712936588149;
    assert_eq!(mft_total_supply(&pool, ":0"), last_lpt_supply);
    assert!(pool_share_price(&pool, 0) > last_share_price);
    let last_share_price = pool_share_price(&pool, 0);
    println!("share_price: {}", last_share_price);

    // add massive liquidity (100 billion)
    let user3 = root.create_user("user3".to_string(), to_yocto("100"));
    mint_and_deposit_token(&user3, &tokens[0], &pool, 100_000_000_000*ONE_NEAR);
    mint_and_deposit_rated_token(&user3, &token_rated_contracts[0], &pool, 100_000_000_000*ONE_STNEAR);
    mint_and_deposit_rated_token (&user3, &token_rated_contracts[1], &pool, 100_000_000_000*ONE_LINEAR);
    let out_come = call!(
        user3,
        pool.add_stable_liquidity(0, vec![U128(100_000_000_000*ONE_NEAR), U128(100_000_000_000*ONE_STNEAR), U128(100_000_000_000*ONE_LINEAR)], U128(1)),
        deposit = to_yocto("0.0007") 
    );
    out_come.assert_success();
    println!("{:#?}", get_logs(&out_come));
    // minted_user_lpt
    assert_eq!(mft_balance_of(&pool, ":0", &user3.account_id()), 299997852137498188726148212849465927);
    // assert_eq!(mft_balance_of(&pool, ":0", &user3.account_id()), 299997852137498188726148212849465927);
    // last_lpt_supply + minted_user_lpt + admin_fee_to_lpt
    let last_lpt_supply = last_lpt_supply + 299997852137498188726148212849465927 + 143329282015797902428444724880;
    assert_eq!(mft_total_supply(&pool, ":0"), last_lpt_supply);
    let last_share_price = pool_share_price(&pool, 0);
    println!("share_price: {}", last_share_price);
}

#[test]
fn sim_rated_swap_max_liquidity() {
    let (root, _owner, pool, tokens, token_rated_contracts) = 
        setup_rated_pool_with_liquidity(
            vec![near()],
            vec![stnear(), linear()],
            vec![100000*ONE_NEAR],
            vec![100000*ONE_STNEAR, 100000*ONE_LINEAR],
            vec![24, 24, 24],
            25,
            10000,
        );

    // add massive liquidity (100 billion)
    let user = root.create_user("user".to_string(), to_yocto("100"));
    mint_and_deposit_token(&user, &tokens[0], &pool, 100_000_000_000*ONE_NEAR);
    mint_and_deposit_rated_token(&user, &token_rated_contracts[0], &pool, 100_000_000_000*ONE_STNEAR);
    mint_and_deposit_rated_token(&user, &token_rated_contracts[1], &pool, 100_000_000_000*ONE_LINEAR);
    let out_come = call!(
        user,
        pool.add_stable_liquidity(0, vec![
            U128(100_000_000_000*ONE_NEAR), U128(100_000_000_000*ONE_STNEAR), U128(100_000_000_000*ONE_LINEAR)], U128(1)),
        deposit = to_yocto("0.0007") 
    );
    out_come.assert_success();
    println!("{:#?}", get_logs(&out_come));
    assert_eq!(mft_balance_of(&pool, ":0", &user.account_id()), 300000000000000000000000000000000000);
    assert_eq!(mft_total_supply(&pool, ":0"), 300000300000000000000000000000000000);
    let last_share_price = pool_share_price(&pool, 0);
    println!("share_price: {}", last_share_price);
}

#[test]
fn sim_rated_swap_lp_storage() {
    let (root, _owner, pool, tokens, _token_rated_contracts) = 
        setup_rated_pool_with_liquidity(
            vec![near()],
            vec![stnear(), linear()],
            vec![100000*ONE_NEAR],
            vec![100000*ONE_STNEAR, 100000*ONE_LINEAR],
            vec![24, 24, 24],
            25,
            10000,
        );

    let near_token = &tokens[0];
    let user = root.create_user("user".to_string(), to_yocto("100"));
    call!(
        user,
        near_token.mint(user.valid_account_id(), U128(500*ONE_NEAR))
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
        near_token.ft_transfer_call(
            pool.valid_account_id(), 
            U128(500*ONE_NEAR), 
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
        pool.add_stable_liquidity(0, vec![U128(500*ONE_NEAR), U128(0), U128(0)], U128(1)),
        deposit = to_yocto("0.0007")
    );
    out_come.assert_success();
    let sb = get_storage_balance(&pool, user.valid_account_id()).unwrap();
    assert_eq!(sb.total.0, to_yocto("0.0025"));
    assert_eq!(sb.available.0, 0);

    // remove by shares
    let out_come = call!(
        user,
        pool.remove_liquidity(0, U128(300*ONE_LPT), vec![U128(1*ONE_NEAR), U128(1*ONE_STNEAR), U128(1*ONE_LINEAR)]),
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
        pool.remove_liquidity(0, U128(300*ONE_LPT), vec![U128(1*ONE_NEAR), U128(1*ONE_STNEAR), U128(1*ONE_LINEAR)]),
        deposit = 1 
    );
    out_come.assert_success();
    let sb = get_storage_balance(&pool, user.valid_account_id()).unwrap();
    assert_eq!(sb.total.0, to_yocto("0.00546"));
    assert_eq!(sb.available.0, 0);
}

#[test]
fn sim_rated_swap_liquidity_two_with_nearx() {
    let (root, owner, pool, tokens, token_rated_contracts) = 
        setup_rated_pool(
            vec![near()],
            vec![nearx()],
            vec![24, 24],
            25,
            10000,
        );

    let nearx_contract = &token_rated_contracts[0];

    call!(
        owner,
        pool.register_rated_token(
            "NEARX".to_string(),
            token_rated_contracts[0].valid_account_id()
        ),
        deposit = 1
    ).assert_success();

    call!(
        root,
        nearx_contract.set_price(U128(2 * 10u128.pow(24)))
    ).assert_success();

    call!(
        owner,
        pool.update_token_rate(
            nearx_contract.valid_account_id()
        ),
        deposit = 1
    ).assert_success();

    let user = root.create_user("user".to_string(), to_yocto("100"));
    mint_and_deposit_token(&user, &tokens[0], &pool, 100000*ONE_NEAR);
    mint_and_deposit_rated_token(&user, &token_rated_contracts[0], &pool, 100000*ONE_NEARX);
    let out_come = call!(
        user,
        pool.add_stable_liquidity(0, vec![
            U128(100000*ONE_NEAR), U128(50000*ONE_NEARX)], U128(1)),
        deposit = to_yocto("0.0007") 
    );
    out_come.assert_success();
    println!("{:#?}", get_logs(&out_come));
    assert_eq!(mft_balance_of(&pool, ":0", &user.account_id()), 200000*ONE_LPT);
    assert_eq!(mft_total_supply(&pool, ":0"), 200000*ONE_LPT);
    let last_share_price = pool_share_price(&pool, 0);
    assert_eq!(100000000, last_share_price);

    let user1 = root.create_user("user1".to_string(), to_yocto("100"));
    mint_and_deposit_token(&user1, &tokens[0], &pool, 100000*ONE_NEAR);
    mint_and_deposit_rated_token(&user1, &token_rated_contracts[0], &pool, 100000*ONE_NEARX);
    let out_come = call!(
        user1,
        pool.add_stable_liquidity(0, vec![
            U128(100000*ONE_NEAR), U128(50000*ONE_NEARX)], U128(1)),
        deposit = to_yocto("0.0007") 
    );
    out_come.assert_success();
    println!("{:#?}", get_logs(&out_come));
    assert_eq!(mft_balance_of(&pool, ":0", &user1.account_id()), 200000*ONE_LPT);
    assert_eq!(mft_total_supply(&pool, ":0"), 400000*ONE_LPT);
    let last_share_price = pool_share_price(&pool, 0);
    assert_eq!(100000000, last_share_price);

    let out_come = call!(
        user1,
        pool.remove_liquidity(0, U128(200000*ONE_LPT), vec![U128(1*ONE_NEAR), U128(1*ONE_NEARX)]),
        deposit = 1 
    );
    out_come.assert_success();
    println!("{:#?}", get_logs(&out_come));
    assert_eq!(mft_balance_of(&pool, ":0", &user1.account_id()), 0);
    assert_eq!(mft_total_supply(&pool, ":0"), 200000*ONE_LPT);
    assert_eq!(100000000, pool_share_price(&pool, 0));

    let out_come = call!(
        user,
        pool.remove_liquidity(0, U128(200000*ONE_LPT), vec![U128(1*ONE_NEAR), U128(1*ONE_NEARX)]),
        deposit = 1 
    );
    assert_eq!(get_error_count(&out_come), 1);
    assert!(get_error_status(&out_come).contains("E69: pool reserved token balance less than MIN_RESERVE"));

}

#[test]
fn sim_rated_swap_two_no_rated_with_nearx() {
    let (root, _owner, pool, tokens, _token_rated_contracts) = 
        setup_rated_pool_with_liquidity(
            vec![near()],
            vec![nearx()],
            vec![100000*ONE_NEAR],
            vec![100000*ONE_NEARX],
            vec![24, 24],
            25,
            10000,
        );
    assert_eq!(
        view!(pool.get_pool(0)).unwrap_json::<PoolInfo>(),
        PoolInfo {
            pool_kind: "RATED_SWAP".to_string(),
            amp: 10000,
            token_account_ids: vec![near(), nearx()],
            amounts: vec![U128(100000*ONE_NEAR), U128(100000*ONE_NEARX)],
            total_fee: 25,
            shares_total_supply: U128(200000*ONE_LPT),
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
        200000*ONE_LPT
    );
    let balances = view!(pool.get_deposits(root.valid_account_id()))
        .unwrap_json::<HashMap<AccountId, U128>>();
    let balances = balances.values().cloned().collect::<Vec<_>>();
    assert_eq!(balances, vec![U128(0), U128(0)]);

    let c = tokens.get(0).unwrap();
    call!(
        root,
        c.ft_transfer_call(pool.valid_account_id(), U128(ONE_NEAR), None, "".to_string()),
        deposit = 1
    )
    .assert_success();

    let out_come = call!(
        root,
        pool.swap(
            vec![SwapAction {
                pool_id: 0,
                token_in: near(),
                amount_in: Some(U128(ONE_NEAR)),
                token_out: nearx(),
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
    assert_eq!(balances[&near()].0, 0);
    assert_eq!(balances[&nearx()].0, 997499999501274936452669);

    assert_eq!(
        view!(pool.get_pool(0)).unwrap_json::<PoolInfo>(),
        PoolInfo {
            pool_kind: "RATED_SWAP".to_string(),
            amp: 10000,
            token_account_ids: vec![near(), nearx()],
            amounts: vec![U128(100001*ONE_NEAR), U128(99999*ONE_NEARX+2500000498725063547331)],
            total_fee: 25,
            shares_total_supply: U128(200000*ONE_LPT + 499999994999720058346),
        }
    );
}

#[test]
fn sim_rated_swap_rate_one_with_fee_with_nearx() {
    let (root, owner, pool, tokens, token_rated_contracts) = 
        setup_rated_pool_with_liquidity(
            vec![near()],
            vec![nearx()],
            vec![100000*ONE_NEAR],
            vec![100000*ONE_NEARX],
            vec![24, 24],
            25,
            10000,
        );
    let nearx_contract = &token_rated_contracts[0];
    assert_eq!(
        view!(pool.get_pool(0)).unwrap_json::<PoolInfo>(),
        PoolInfo {
            pool_kind: "RATED_SWAP".to_string(),
            amp: 10000,
            token_account_ids: vec![near(), nearx()],
            amounts: vec![U128(100000*ONE_NEAR), U128(100000*ONE_NEARX)],
            total_fee: 25,
            shares_total_supply: U128(200000*ONE_LPT),
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
        200000*ONE_LPT
    );
    let balances = view!(pool.get_deposits(root.valid_account_id()))
        .unwrap_json::<HashMap<AccountId, U128>>();
    let balances = balances.values().cloned().collect::<Vec<_>>();
    assert_eq!(balances, vec![U128(0), U128(0)]);

    let c = tokens.get(0).unwrap();
    call!(
        root,
        c.ft_transfer_call(pool.valid_account_id(), U128(ONE_NEAR), None, "".to_string()),
        deposit = 1
    )
    .assert_success();

    call!(
        owner,
        pool.register_rated_token(
            "NEARX".to_string(),
            token_rated_contracts[0].valid_account_id()
        ),
        deposit = 1
    ).assert_success();

    call!(
        owner,
        pool.update_token_rate(
            nearx_contract.valid_account_id()
        ),
        deposit = 1
    ).assert_success();

    let rated_infos = view!(pool.list_rated_tokens()).unwrap_json::<HashMap<String, RatedTokenInfo>>();

    println!("{:?}", rated_infos);

    let out_come = call!(
        root,
        pool.swap(
            vec![SwapAction {
                pool_id: 0,
                token_in: near(),
                amount_in: Some(U128(ONE_NEAR)),
                token_out: nearx(),
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
    assert_eq!(balances[&near()].0, 0);
    assert_eq!(balances[&nearx()].0, 997499999501274936452669);

    assert_eq!(
        view!(pool.get_pool(0)).unwrap_json::<PoolInfo>(),
        PoolInfo {
            pool_kind: "RATED_SWAP".to_string(),
            amp: 10000,
            token_account_ids: vec![near(), nearx()],
            amounts: vec![U128(100001*ONE_NEAR), U128(99999*ONE_NEARX+2500000498725063547331)],
            total_fee: 25,
            shares_total_supply: U128(200000*ONE_LPT + 499999994999720058346),
        }
    );
}

#[test]
fn sim_rated_swap_rate_one_no_fee_with_nearx() {
    let (root, owner, pool, tokens, token_rated_contracts) = 
        setup_rated_pool_with_liquidity(
            vec![near()],
            vec![nearx()],
            vec![100000*ONE_NEAR],
            vec![100000*ONE_NEARX],
            vec![24, 24],
            0,
            10000,
        );
    let nearx_contract = &token_rated_contracts[0];
    assert_eq!(
        view!(pool.get_pool(0)).unwrap_json::<PoolInfo>(),
        PoolInfo {
            pool_kind: "RATED_SWAP".to_string(),
            amp: 10000,
            token_account_ids: vec![near(), nearx()],
            amounts: vec![U128(100000*ONE_NEAR), U128(100000*ONE_NEARX)],
            total_fee: 0,
            shares_total_supply: U128(200000*ONE_LPT),
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
        200000*ONE_LPT
    );
    let balances = view!(pool.get_deposits(root.valid_account_id()))
        .unwrap_json::<HashMap<AccountId, U128>>();
    let balances = balances.values().cloned().collect::<Vec<_>>();
    assert_eq!(balances, vec![U128(0), U128(0)]);

    let c = tokens.get(0).unwrap();
    call!(
        root,
        c.ft_transfer_call(pool.valid_account_id(), U128(ONE_NEAR), None, "".to_string()),
        deposit = 1
    )
    .assert_success();

    call!(
        owner,
        pool.register_rated_token(
            "NEARX".to_string(),
            token_rated_contracts[0].valid_account_id()
        ),
        deposit = 1
    ).assert_success();

    call!(
        owner,
        pool.update_token_rate(
            nearx_contract.valid_account_id()
        ),
        deposit = 1
    ).assert_success();

    let rated_infos = view!(pool.list_rated_tokens()).unwrap_json::<HashMap<String, RatedTokenInfo>>();

    println!("{:?}", rated_infos);

    let out_come = call!(
        root,
        pool.swap(
            vec![SwapAction {
                pool_id: 0,
                token_in: near(),
                amount_in: Some(U128(ONE_NEAR)),
                token_out: nearx(),
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
    assert_eq!(balances[&near()].0, 0);
    assert_eq!(balances[&nearx()].0, 999999999500024998950044);

    assert_eq!(
        view!(pool.get_pool(0)).unwrap_json::<PoolInfo>(),
        PoolInfo {
            pool_kind: "RATED_SWAP".to_string(),
            amp: 10000,
            token_account_ids: vec![near(), nearx()],
            amounts: vec![U128(100001*ONE_NEAR), U128(99999*ONE_NEARX+499975001049956)],
            total_fee: 0,
            shares_total_supply: U128(200000*ONE_LPT),
        }
    );
}