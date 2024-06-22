use mock_price_oracle::Price;
use mock_pyth::PythPrice;
use near_contract_standards::fungible_token::metadata::FungibleTokenMetadata;
use near_sdk::{json_types::U128, AccountId};
use near_sdk_sim::{call, to_yocto, view};
use ref_exchange::{DegenOracleConfig, DegenTokenInfo, DegenType, PoolInfo, PriceOracleConfig, PythOracleConfig, SwapAction};
use std::{collections::HashMap, convert::TryInto};
use crate::common::utils::*;
pub mod common;

const ONE_BTC: u128 = 10u128.pow(8);
const ONE_ETH: u128 = 10u128.pow(18);
const ONE_NEAR: u128 = 10u128.pow(24);
const ONE_LPT: u128 = 10u128.pow(24);

#[test]
fn sim_degen() {
    let (root, owner, pool, tokens) = 
        setup_degen_pool(
            vec![eth(), near()],
            vec![100000*ONE_ETH, 100000*ONE_NEAR],
            vec![18, 24],
            25,
            10000,
        );
    let pyth_contract = setup_pyth_oracle(&root);
    let block_timestamp = root.borrow_runtime().current_block().block_timestamp;
    call!(
        root,
        pyth_contract.set_price(mock_pyth::PriceIdentifier(hex::decode("27e867f0f4f61076456d1a73b14c7edc1cf5cef4f4d6193a33424288f11bd0f4").unwrap().try_into().unwrap()), PythPrice {
            price: 100000000.into(),
            conf: 397570.into(),
            expo: -8,
            publish_time: nano_to_sec(block_timestamp) as i64,
        })
    ).assert_success();
    let price_oracle_contract = setup_price_oracle(&root);
    call!(
        root,
        price_oracle_contract.set_price_data(eth(), Price {
            multiplier: 10000,
            decimals: 22,
        })
    ).assert_success();

    call!(
        owner, 
        pool.register_degen_oracle_config(DegenOracleConfig::PriceOracle(PriceOracleConfig { 
            oracle_id: price_oracle(), 
            expire_ts: 3600 * 10u64.pow(9), 
            maximum_recency_duration_sec: 90, 
            maximum_staleness_duration_sec: 90
        })),
        deposit = 1
    )
    .assert_success();
    call!(
        owner, 
        pool.register_degen_oracle_config(DegenOracleConfig::PythOracle(PythOracleConfig { 
            oracle_id: pyth_oracle(), 
            expire_ts: 3600 * 10u64.pow(9), 
            pyth_price_valid_duration_sec: 60
        })),
        deposit = 1
    )
    .assert_success();
    call!(
        owner, 
        pool.register_degen_token(to_va(eth()), DegenType::PriceOracle { decimals: 18 }),
        deposit = 1
    )
    .assert_success();
    call!(
        owner, 
        pool.register_degen_token(to_va(near()), DegenType::PythOracle { price_identifier: ref_exchange::pyth_oracle::PriceIdentifier(hex::decode("27e867f0f4f61076456d1a73b14c7edc1cf5cef4f4d6193a33424288f11bd0f4").unwrap().try_into().unwrap()) }),
        deposit = 1
    )
    .assert_success();

    call!(
        root, 
        pool.update_degen_token_price(to_va(eth())),
        deposit = 0
    )
    .assert_success();

    call!(
        root, 
        pool.update_degen_token_price(to_va(near())),
        deposit = 0
    )
    .assert_success(); 

    let out_come = call!(
        root,
        pool.add_stable_liquidity(0, vec![100000*ONE_ETH, 100000*ONE_NEAR].into_iter().map(|x| U128(x)).collect(), U128(1)),
        deposit = to_yocto("0.0007")
    );
    out_come.assert_success();
    println!("{:#?}", get_logs(&out_come));

    assert_eq!(
        view!(pool.get_pool(0)).unwrap_json::<PoolInfo>(),
        PoolInfo {
            pool_kind: "DEGEN_SWAP".to_string(),
            amp: 10000,
            token_account_ids: vec![eth(), near()],
            amounts: vec![U128(100000*ONE_ETH), U128(100000*ONE_NEAR)],
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

    let c = tokens.get(1).unwrap();
    call!(
        root,
        c.ft_transfer_call(pool.valid_account_id(), U128(ONE_NEAR), None, "".to_string()),
        deposit = 1
    )
    .assert_success();

    let degen_infos = view!(pool.list_degen_tokens()).unwrap_json::<HashMap<String, DegenTokenInfo>>();

    println!("{:?}", degen_infos);

    assert_eq!(997499999501274936, view!(pool.get_return(0, to_va(near()), U128(ONE_NEAR), to_va(eth()))).unwrap_json::<U128>().0);

    let balances = view!(pool.get_deposits(root.valid_account_id()))
    .unwrap_json::<HashMap<AccountId, U128>>();
    assert_eq!(balances[&near()].0, ONE_NEAR);
    assert_eq!(balances[&eth()].0, 0);

    let out_come = call!(
        root,
        pool.swap(
            vec![SwapAction {
                pool_id: 0,
                token_in: near(),
                amount_in: Some(U128(ONE_NEAR)),
                token_out: eth(),
                min_amount_out: U128(1)
            }],
            None
        ),
        gas = 300000000000000
    );
    out_come.assert_success();
    println!("{:#?}", get_logs(&out_come));

    let balances = view!(pool.get_deposits(root.valid_account_id()))
        .unwrap_json::<HashMap<AccountId, U128>>();
    assert_eq!(balances[&near()].0, 0);
    assert_eq!(balances[&eth()].0, 997499999501274936);

    println!("{:?}", view!(pool.list_degen_tokens()).unwrap_json::<HashMap<String, DegenTokenInfo>>());
    println!("{:?}", view!(pool.list_degen_configs()).unwrap_json::<HashMap<String, DegenOracleConfig>>());
}


#[test]
fn sim_degen1() {
    let (root, owner, pool, tokens) = 
        setup_degen_pool(
            vec![eth(), btc()],
            vec![100000*ONE_ETH, 100000*ONE_BTC],
            vec![18, 8],
            25,
            10000,
        );
    let pyth_contract = setup_pyth_oracle(&root);
    let block_timestamp = root.borrow_runtime().current_block().block_timestamp;
    call!(
        root,
        pyth_contract.set_price(mock_pyth::PriceIdentifier(hex::decode("27e867f0f4f61076456d1a73b14c7edc1cf5cef4f4d6193a33424288f11bd0f4").unwrap().try_into().unwrap()), PythPrice {
            price: 100000000.into(),
            conf: 397570.into(),
            expo: -8,
            publish_time: nano_to_sec(block_timestamp) as i64,
        })
    ).assert_success();
    let price_oracle_contract = setup_price_oracle(&root);
    call!(
        root,
        price_oracle_contract.set_price_data(eth(), Price {
            multiplier: 10000,
            decimals: 22,
        })
    ).assert_success();

    call!(
        owner, 
        pool.register_degen_oracle_config(DegenOracleConfig::PriceOracle(PriceOracleConfig { 
            oracle_id: price_oracle(), 
            expire_ts: 3600 * 10u64.pow(9), 
            maximum_recency_duration_sec: 90, 
            maximum_staleness_duration_sec: 90
        })),
        deposit = 1
    )
    .assert_success();
    call!(
        owner, 
        pool.register_degen_oracle_config(DegenOracleConfig::PythOracle(PythOracleConfig { 
            oracle_id: pyth_oracle(), 
            expire_ts: 3600 * 10u64.pow(9), 
            pyth_price_valid_duration_sec: 60
        })),
        deposit = 1
    )
    .assert_success();
    call!(
        owner, 
        pool.register_degen_token(to_va(eth()), DegenType::PriceOracle { decimals: 18 }),
        deposit = 1
    )
    .assert_success();
    call!(
        owner, 
        pool.register_degen_token(to_va(btc()), DegenType::PythOracle { price_identifier: ref_exchange::pyth_oracle::PriceIdentifier(hex::decode("27e867f0f4f61076456d1a73b14c7edc1cf5cef4f4d6193a33424288f11bd0f4").unwrap().try_into().unwrap()) }),
        deposit = 1
    )
    .assert_success();

    call!(
        root, 
        pool.update_degen_token_price(to_va(eth())),
        deposit = 0
    )
    .assert_success();

    call!(
        root, 
        pool.update_degen_token_price(to_va(btc())),
        deposit = 0
    )
    .assert_success(); 

    let out_come = call!(
        root,
        pool.add_stable_liquidity(0, vec![100000*ONE_ETH, 100000*ONE_BTC].into_iter().map(|x| U128(x)).collect(), U128(1)),
        deposit = to_yocto("0.0007")
    );
    out_come.assert_success();
    println!("{:#?}", get_logs(&out_come));
    assert_eq!(100000000, view!(pool.get_pool_share_price(0)).unwrap_json::<U128>().0);
    assert_eq!(
        view!(pool.get_pool(0)).unwrap_json::<PoolInfo>(),
        PoolInfo {
            pool_kind: "DEGEN_SWAP".to_string(),
            amp: 10000,
            token_account_ids: vec![eth(), btc()],
            amounts: vec![U128(100000*ONE_ETH), U128(100000*ONE_BTC)],
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

    let c = tokens.get(1).unwrap();
    call!(
        root,
        c.ft_transfer_call(pool.valid_account_id(), U128(ONE_BTC), None, "".to_string()),
        deposit = 1
    )
    .assert_success();

    let degen_infos = view!(pool.list_degen_tokens()).unwrap_json::<HashMap<String, DegenTokenInfo>>();

    println!("{:?}", degen_infos);

    assert_eq!(997499999501274936, view!(pool.get_return(0, to_va(btc()), U128(ONE_BTC), to_va(eth()))).unwrap_json::<U128>().0);

    let balances = view!(pool.get_deposits(root.valid_account_id()))
    .unwrap_json::<HashMap<AccountId, U128>>();
    assert_eq!(balances[&btc()].0, ONE_BTC);
    assert_eq!(balances[&eth()].0, 0);

    let out_come = call!(
        root,
        pool.swap(
            vec![SwapAction {
                pool_id: 0,
                token_in: btc(),
                amount_in: Some(U128(ONE_BTC)),
                token_out: eth(),
                min_amount_out: U128(1)
            }],
            None
        ),
        gas = 300000000000000
    );
    out_come.assert_success();
    println!("{:#?}", get_logs(&out_come));

    let balances = view!(pool.get_deposits(root.valid_account_id()))
        .unwrap_json::<HashMap<AccountId, U128>>();
    assert_eq!(balances[&btc()].0, 0);
    assert_eq!(balances[&eth()].0, 997499999501274936);
}
