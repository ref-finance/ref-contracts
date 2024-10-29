use std::collections::HashMap;
use std::convert::TryInto;
use std::vec;

use near_sdk_sim::{deploy, call, view, to_yocto};
use near_sdk::json_types::{U128};
use near_sdk::AccountId;
use near_sdk::serde_json::{json, Value};

pub use near_sdk::serde::{Deserialize, Serialize};
use mock_boost_farming::{ContractContract as MockBoostFarming};
use ref_exchange::{Action, SwapAction, AddLiquidityInfo, AddLiquidityPrediction};

use crate::common::utils::*;
pub mod common;

near_sdk_sim::lazy_static_include::lazy_static_include_bytes! {
    MOCK_BOOST_FARMING_WASM_BYTES => "../res/mock_boost_farming.wasm",
}

pub fn boost_farming() -> AccountId {
    "boost_farming".to_string()
}

#[test]
fn test_hotzap_simple_pool() {
    const DAI_ETH: u64 = 0;
    const ETH_USDT: u64 = 1;
    const DAI_USDT: u64 = 2;
    let (root, _, pool, token_dai, token_eth, token_usdt) = setup_pool_with_liquidity();
    let new_user = root.create_user("new_user".to_string(), to_yocto("100"));
    call!(
        new_user,
        pool.storage_deposit(None, None),
        deposit = to_yocto("1")
    )
    .assert_success();
    call!(
        new_user,
        token_dai.mint(to_va(new_user.account_id.clone()), U128(to_yocto("10")))
    )
    .assert_success();

    let out_eth_amount = view!(pool.get_return(DAI_ETH, to_va(token_dai.account_id().clone()), U128(to_yocto("1")), to_va(token_eth.account_id().clone()))).unwrap_json::<U128>().0;
    let out_usdt_amount = view!(pool.get_return(DAI_USDT, to_va(token_dai.account_id().clone()), U128(to_yocto("1")), to_va(token_usdt.account_id().clone()))).unwrap_json::<U128>().0;
    println!("out_eth_amount: {:?}", out_eth_amount);
    println!("out_usdt_amount: {:?}", out_usdt_amount);
    let add_liquidity_prediction = view!(pool.predict_add_simple_liquidity(ETH_USDT, &vec![U128(out_eth_amount), U128(out_usdt_amount)])).unwrap_json::<AddLiquidityPrediction>();
    println!("shares : {:?}", add_liquidity_prediction.mint_shares.0);
    println!("remain eth : {:?}", out_eth_amount - add_liquidity_prediction.need_amounts[0].0);
    println!("remain usdt : {:?}", out_usdt_amount - add_liquidity_prediction.need_amounts[1].0);

    println!("predict hot zap: {:?}", view!(pool.predict_hot_zap(
        None,
        None,
        token_dai.valid_account_id(),
        U128(to_yocto("2")),
        vec![
            Action::Swap(SwapAction { 
                pool_id: DAI_ETH, 
                token_in: token_dai.account_id(), 
                amount_in: Some(U128(to_yocto("1"))), 
                token_out: token_eth.account_id(), 
                min_amount_out: U128(1) 
            }),
            Action::Swap(SwapAction { 
                pool_id: DAI_USDT, 
                token_in: token_dai.account_id(), 
                amount_in: Some(U128(to_yocto("1"))), 
                token_out: token_usdt.account_id(), 
                min_amount_out: U128(1) 
            })
        ],
        vec![
            AddLiquidityInfo {
                pool_id: 1,
                amounts: add_liquidity_prediction.need_amounts.clone(),
                min_amounts: Some(vec![U128(1814048647419868151852681u128), U128(907024323709934075926341u128)]),
                min_shares: None
            }
        ]
    )).unwrap_json::<Option<(Vec<AddLiquidityPrediction>, HashMap<AccountId, U128>)>>());

    let mock_boost_farming = deploy!(
        contract: MockBoostFarming,
        contract_id: boost_farming(),
        bytes: &MOCK_BOOST_FARMING_WASM_BYTES,
        signer_account: root,
        init_method: new(root.account_id())
    );

    let seed_id = "swap@1".to_string();
    let outcome = call!(
        root,
        pool.mft_register(":1".to_string(), mock_boost_farming.valid_account_id()),
        deposit = to_yocto("1")
    );
    outcome.assert_success();

    let outcome = call!(
        new_user,
        mock_boost_farming.storage_deposit(None, None),
        deposit = 100_000_000_000_000_000_000_000
    );
    outcome.assert_success();

    let outcome = call!(
        root,
        mock_boost_farming.create_seed(seed_id.clone(),24, Some(U128(0)), Some(0)),
        deposit = 1
    );
    outcome.assert_success();

    let outcome = call!(
        new_user,
        token_dai.ft_transfer_call(
            to_va(swap()),
            to_yocto("2").into(),
            None,
            json!({
                "hot_zap_actions": [
                    {"pool_id": 0, "token_in": "dai001", "amount_in":"1000000000000000000000000", "token_out":"eth002", "min_amount_out":"1"},
                    {"pool_id": 2, "token_in": "dai001", "amount_in":"1000000000000000000000000", "token_out":"usdt", "min_amount_out":"1"}
                ],
                "add_liquidity_infos": vec![
                    AddLiquidityInfo {
                        pool_id: 1,
                        amounts: add_liquidity_prediction.need_amounts.clone(),
                        min_amounts: Some(vec![U128(1814048647419868151852681u128), U128(907024323709934075926341u128)]),
                        min_shares: None
                    }
                ],
            }).to_string()
        ),
        1,
        near_sdk_sim::DEFAULT_GAS
    );
    println!("{:#?}", get_logs(&outcome));
    // println!("{:?}", outcome.promise_errors()[0].as_ref().unwrap().status());
    assert!(outcome.promise_errors().is_empty());
    
    println!("before mft_transfer_all_call: {:?}", view!(mock_boost_farming.get_seed(seed_id.clone())).unwrap_json::<Value>());

    let outcome = call!(
        new_user,
        pool.mft_transfer_all_call(":1".to_string(), mock_boost_farming.valid_account_id(), None, "\"Free\"".to_string()),
        1,
        near_sdk_sim::DEFAULT_GAS
    );
    outcome.assert_success();

    println!("after mft_transfer_all_call: {:?}", view!(mock_boost_farming.get_seed(seed_id)).unwrap_json::<Value>());
    println!("view new_user deposit: {:?}", view!(pool.get_deposits(new_user.valid_account_id())).unwrap_json::<HashMap<AccountId, U128>>());
}

#[test]
fn test_hotzap_simple_pool_add_two() {
    const DAI_ETH: u64 = 0;
    const ETH_USDT: u64 = 1;
    const DAI_USDT: u64 = 2;
    const ETH_USDT2: u64 = 3;

    let (root, _, pool, token_dai, token_eth, token_usdt) = setup_pool_with_liquidity();
    call!(
        root,
        pool.add_simple_pool(vec![to_va(eth()), to_va(usdt())], 25),
        deposit = to_yocto("1")
    )
    .assert_success();
    call!(
        root,
        pool.add_liquidity(ETH_USDT2, vec![U128(to_yocto("20")), U128(to_yocto("10"))], None),
        deposit = to_yocto("0.0007")
    )
    .assert_success();

    let new_user = root.create_user("new_user".to_string(), to_yocto("100"));
    call!(
        new_user,
        pool.storage_deposit(None, None),
        deposit = to_yocto("1")
    )
    .assert_success();
    call!(
        new_user,
        token_dai.mint(to_va(new_user.account_id.clone()), U128(to_yocto("10")))
    )
    .assert_success();

    let out_eth_amount = view!(pool.get_return(DAI_ETH, to_va(token_dai.account_id().clone()), U128(to_yocto("1")), to_va(token_eth.account_id().clone()))).unwrap_json::<U128>().0;
    let out_usdt_amount = view!(pool.get_return(DAI_USDT, to_va(token_dai.account_id().clone()), U128(to_yocto("1")), to_va(token_usdt.account_id().clone()))).unwrap_json::<U128>().0;
    println!("out_eth_amount: {:?}", out_eth_amount);
    println!("out_usdt_amount: {:?}", out_usdt_amount);
    let add_liquidity_prediction = view!(pool.predict_add_simple_liquidity(ETH_USDT, &vec![U128(out_eth_amount / 2), U128(out_usdt_amount / 2)])).unwrap_json::<AddLiquidityPrediction>();
    let add_liquidity_prediction2 = view!(pool.predict_add_simple_liquidity(ETH_USDT2, &vec![U128(out_eth_amount / 2), U128(out_usdt_amount / 2)])).unwrap_json::<AddLiquidityPrediction>();
    println!("remain eth : {:?}", out_eth_amount - add_liquidity_prediction.need_amounts[0].0 - add_liquidity_prediction2.need_amounts[0].0);
    println!("remain usdt : {:?}", out_usdt_amount - add_liquidity_prediction.need_amounts[1].0 - add_liquidity_prediction2.need_amounts[1].0);

    println!("predict hot zap: {:?}", view!(pool.predict_hot_zap(
        None,
        None,
        token_dai.valid_account_id(),
        U128(to_yocto("2")),
        vec![
            Action::Swap(SwapAction { 
                pool_id: DAI_ETH, 
                token_in: token_dai.account_id(), 
                amount_in: Some(U128(to_yocto("1"))), 
                token_out: token_eth.account_id(), 
                min_amount_out: U128(1) 
            }),
            Action::Swap(SwapAction { 
                pool_id: DAI_USDT, 
                token_in: token_dai.account_id(), 
                amount_in: Some(U128(to_yocto("1"))), 
                token_out: token_usdt.account_id(), 
                min_amount_out: U128(1) 
            })
        ],
        vec![
            AddLiquidityInfo {
                pool_id: ETH_USDT,
                amounts: add_liquidity_prediction.need_amounts.clone(),
                min_amounts: Some(vec![U128(0), U128(0)]),
                min_shares: None
            },
            AddLiquidityInfo {
                pool_id: ETH_USDT2,
                amounts: add_liquidity_prediction2.need_amounts.clone(),
                min_amounts: Some(vec![U128(0), U128(0)]),
                min_shares: None
            }
        ]
    )).unwrap_json::<Option<(Vec<AddLiquidityPrediction>, HashMap<AccountId, U128>)>>());

    let mock_boost_farming = deploy!(
        contract: MockBoostFarming,
        contract_id: boost_farming(),
        bytes: &MOCK_BOOST_FARMING_WASM_BYTES,
        signer_account: root,
        init_method: new(root.account_id())
    );

    let seed_id = "swap@1".to_string();
    let outcome = call!(
        root,
        pool.mft_register(":1".to_string(), mock_boost_farming.valid_account_id()),
        deposit = to_yocto("1")
    );
    outcome.assert_success();

    let outcome = call!(
        new_user,
        mock_boost_farming.storage_deposit(None, None),
        deposit = 100_000_000_000_000_000_000_000
    );
    outcome.assert_success();

    let outcome = call!(
        root,
        mock_boost_farming.create_seed(seed_id.clone(),24, Some(U128(0)), Some(0)),
        deposit = 1
    );
    outcome.assert_success();

    let outcome = call!(
        new_user,
        token_dai.ft_transfer_call(
            to_va(swap()),
            to_yocto("2").into(),
            None,
            json!({
                "hot_zap_actions": [
                    {"pool_id": 0, "token_in": "dai001", "amount_in":"1000000000000000000000000", "token_out":"eth002", "min_amount_out":"1"},
                    {"pool_id": 2, "token_in": "dai001", "amount_in":"1000000000000000000000000", "token_out":"usdt", "min_amount_out":"1"}
                ],
                "add_liquidity_infos": vec![
                    AddLiquidityInfo {
                        pool_id: ETH_USDT,
                        amounts: add_liquidity_prediction.need_amounts.clone(),
                        min_amounts: Some(vec![U128(0), U128(0)]),
                        min_shares: None
                    },
                    AddLiquidityInfo {
                        pool_id: ETH_USDT2,
                        amounts: add_liquidity_prediction2.need_amounts.clone(),
                        min_amounts: Some(vec![U128(0), U128(0)]),
                        min_shares: None
                    }
                ],
            }).to_string()
        ),
        1,
        near_sdk_sim::DEFAULT_GAS
    );
    println!("{:#?}", get_logs(&outcome));
    // println!("{:?}", outcome.promise_errors()[0].as_ref().unwrap().status());
    assert!(outcome.promise_errors().is_empty());
    println!("gas burn: {:?}", outcome.gas_burnt());
    
    println!("before mft_transfer_all_call: {:?}", view!(mock_boost_farming.get_seed(seed_id.clone())).unwrap_json::<Value>());

    let outcome = call!(
        new_user,
        pool.mft_transfer_all_call(":1".to_string(), mock_boost_farming.valid_account_id(), None, "\"Free\"".to_string()),
        1,
        near_sdk_sim::DEFAULT_GAS
    );
    outcome.assert_success();

    println!("after mft_transfer_all_call: {:?}", view!(mock_boost_farming.get_seed(seed_id)).unwrap_json::<Value>());
    println!("view new_user deposit: {:?}", view!(pool.get_deposits(new_user.valid_account_id())).unwrap_json::<HashMap<AccountId, U128>>());
}

#[test]
fn test_hotzap_stable_pool() {
    const ONE_DAI: u128 = 1000000000000000000;
    const ONE_USDT: u128 = 1000000;
    const ONE_USDC: u128 = 1000000;

    const DAI_USDT_USDC: u64 = 0;
    const DAI_USDT: u64 = 1;
    const DAI_USDC: u64 = 2;
    let (root, _owner, pool, tokens) = 
        setup_stable_pool_with_liquidity(
            vec![dai(), usdt(), usdc()],
            vec![100000*ONE_DAI, 100000*ONE_USDT, 100000*ONE_USDC],
            vec![18, 6, 6],
            25,
            10000,
        );

    call!(
        root,
        pool.add_simple_pool(vec![to_va(dai()), to_va(usdt())], 25),
        deposit = to_yocto("1")
    )
    .assert_success();
    call!(
        root,
        pool.add_simple_pool(vec![to_va(dai()), to_va(usdc())], 25),
        deposit = to_yocto("1")
    )
    .assert_success();

    let token_dai = &tokens[0];
    let token_usdt = &tokens[1];
    let token_usdc = &tokens[2];

    call!(
        root,
        token_dai.ft_transfer_call(pool.valid_account_id(), U128(200*ONE_DAI), None, "".to_string()),
        deposit = 1
    )
    .assert_success();

    call!(
        root,
        token_usdt.ft_transfer_call(pool.valid_account_id(), U128(100*ONE_USDT), None, "".to_string()),
        deposit = 1
    )
    .assert_success();

    call!(
        root,
        token_usdc.ft_transfer_call(pool.valid_account_id(), U128(100*ONE_USDC), None, "".to_string()),
        deposit = 1
    )
    .assert_success();
    
    call!(
        root,
        pool.add_liquidity(DAI_USDT, vec![U128(100*ONE_DAI), U128(100*ONE_USDT)], None),
        deposit = to_yocto("0.0007")
    )
    .assert_success();
    call!(
        root,
        pool.add_liquidity(DAI_USDC, vec![U128(100*ONE_DAI), U128(100*ONE_USDC)], None),
        deposit = to_yocto("0.0007")
    )
    .assert_success();

    let new_user = root.create_user("new_user".to_string(), to_yocto("100"));
    call!(
        new_user,
        pool.storage_deposit(None, None),
        deposit = to_yocto("1")
    )
    .assert_success();
    call!(
        new_user,
        token_dai.mint(to_va(new_user.account_id.clone()), U128(to_yocto("10")))
    )
    .assert_success();

    let out_usdt_amount = view!(pool.get_return(DAI_USDT, to_va(token_dai.account_id().clone()), U128(5*ONE_DAI), to_va(token_usdt.account_id().clone()))).unwrap_json::<U128>().0;
    let out_usdc_amount = view!(pool.get_return(DAI_USDC, to_va(token_dai.account_id().clone()), U128(5*ONE_DAI), to_va(token_usdc.account_id().clone()))).unwrap_json::<U128>().0;
    println!("out_usdt_amount: {:?}", out_usdt_amount);
    println!("out_usdc_amount: {:?}", out_usdc_amount);
    println!("{:?}", view!(pool.predict_add_stable_liquidity(DAI_USDT_USDC, &vec![U128(5*ONE_DAI), U128(4750565), U128(4750565)])).unwrap_json::<U128>().0);

    println!("predict hot zap: {:?}", view!(pool.predict_hot_zap(
        None,
        None,
        token_dai.valid_account_id(),
        U128(15*ONE_DAI),
        vec![
            Action::Swap(SwapAction { 
                pool_id: DAI_USDT, 
                token_in: token_dai.account_id(), 
                amount_in: Some(U128(5*ONE_DAI)), 
                token_out: token_usdt.account_id(), 
                min_amount_out: U128(1) 
            }),
            Action::Swap(SwapAction { 
                pool_id: DAI_USDC, 
                token_in: token_dai.account_id(), 
                amount_in: Some(U128(5*ONE_DAI)), 
                token_out: token_usdc.account_id(), 
                min_amount_out: U128(1) 
            })
        ],
        vec![
            AddLiquidityInfo {
                pool_id: 0,
                amounts: vec![U128(5*ONE_DAI), U128(4750565), U128(4750565)],
                min_amounts: None,
                min_shares: Some(U128(0))
            }
        ]
    )).unwrap_json::<Option<(Vec<AddLiquidityPrediction>, HashMap<AccountId, U128>)>>());

    let mock_boost_farming = deploy!(
        contract: MockBoostFarming,
        contract_id: boost_farming(),
        bytes: &MOCK_BOOST_FARMING_WASM_BYTES,
        signer_account: root,
        init_method: new(root.account_id())
    );

    let seed_id = "swap@0".to_string();
    let outcome = call!(
        root,
        pool.mft_register(":0".to_string(), mock_boost_farming.valid_account_id()),
        deposit = to_yocto("1")
    );
    outcome.assert_success();

    let outcome = call!(
        new_user,
        mock_boost_farming.storage_deposit(None, None),
        deposit = 100_000_000_000_000_000_000_000
    );
    outcome.assert_success();

    let outcome = call!(
        root,
        mock_boost_farming.create_seed(seed_id.clone(),24, Some(U128(0)), Some(0)),
        deposit = 1
    );
    outcome.assert_success();

    let outcome = call!(
        new_user,
        token_dai.ft_transfer_call(
            to_va(swap()),
            U128(15*ONE_DAI),
            None,
            json!({
                "hot_zap_actions": [
                    {"pool_id": DAI_USDT, "token_in": token_dai.account_id(), "amount_in": (5*ONE_DAI).to_string(), "token_out": token_usdt.account_id(), "min_amount_out":"1"},
                    {"pool_id": DAI_USDC, "token_in": token_dai.account_id(), "amount_in": (5*ONE_DAI).to_string(), "token_out": token_usdc.account_id(), "min_amount_out":"1"}
                ],
                "add_liquidity_infos": vec![
                    AddLiquidityInfo {
                        pool_id: 0,
                        amounts: vec![U128(5*ONE_DAI), U128(4750565), U128(4750565)],
                        min_amounts: None,
                        min_shares: Some(U128(0))
                    }
                ],
            }).to_string()
        ),
        deposit = 1
    );
    
    println!("{:#?}", get_logs(&outcome));
    assert!(outcome.promise_errors().is_empty());

    println!("before mft_transfer_all_call: {:?}", view!(mock_boost_farming.get_seed(seed_id.clone())).unwrap_json::<Value>());

    let outcome = call!(
        new_user,
        pool.mft_transfer_all_call(":0".to_string(), mock_boost_farming.valid_account_id(), None, "\"Free\"".to_string()),
        1,
        near_sdk_sim::DEFAULT_GAS
    );
    outcome.assert_success();

    println!("after mft_transfer_all_call: {:?}", view!(mock_boost_farming.get_seed(seed_id)).unwrap_json::<Value>());
    println!("view new_user deposit: {:?}", view!(pool.get_deposits(new_user.valid_account_id())).unwrap_json::<HashMap<AccountId, U128>>());
}

#[test]
fn test_hotzap_stable_pool_add_two() {
    const ONE_DAI: u128 = 1000000000000000000;
    const ONE_USDT: u128 = 1000000;
    const ONE_USDC: u128 = 1000000;

    const DAI_USDT_USDC: u64 = 0;
    const DAI_USDT: u64 = 1;
    const DAI_USDC: u64 = 2;
    const DAI_USDT_USDC2: u64 = 3;
    
    let (root, owner, pool, tokens) = 
        setup_stable_pool_with_liquidity(
            vec![dai(), usdt(), usdc()],
            vec![100000*ONE_DAI, 100000*ONE_USDT, 100000*ONE_USDC],
            vec![18, 6, 6],
            25,
            10000,
        );

    call!(
        root,
        pool.add_simple_pool(vec![to_va(dai()), to_va(usdt())], 25),
        deposit = to_yocto("1")
    )
    .assert_success();
    call!(
        root,
        pool.add_simple_pool(vec![to_va(dai()), to_va(usdc())], 25),
        deposit = to_yocto("1")
    )
    .assert_success();

    call!(
        owner,
        pool.add_stable_swap_pool(
            vec![dai(), usdt(), usdc()].into_iter().map(|x| x.try_into().unwrap()).collect(), 
            vec![18, 6, 6],
            25,
            10000
        ),
        deposit = to_yocto("1"))
    .assert_success();

    let token_dai = &tokens[0];
    let token_usdt = &tokens[1];
    let token_usdc = &tokens[2];

    call!(
        root,
        token_dai.ft_transfer_call(pool.valid_account_id(), U128(1000*ONE_DAI), None, "".to_string()),
        deposit = 1
    )
    .assert_success();

    call!(
        root,
        token_usdt.ft_transfer_call(pool.valid_account_id(), U128(1000*ONE_USDT), None, "".to_string()),
        deposit = 1
    )
    .assert_success();

    call!(
        root,
        token_usdc.ft_transfer_call(pool.valid_account_id(), U128(1000*ONE_USDC), None, "".to_string()),
        deposit = 1
    )
    .assert_success();
    
    call!(
        root,
        pool.add_liquidity(DAI_USDT, vec![U128(100*ONE_DAI), U128(100*ONE_USDT)], None),
        deposit = to_yocto("0.0007")
    )
    .assert_success();
    call!(
        root,
        pool.add_liquidity(DAI_USDC, vec![U128(100*ONE_DAI), U128(100*ONE_USDC)], None),
        deposit = to_yocto("0.0007")
    )
    .assert_success();

    call!(
        root,
        pool.add_stable_liquidity(DAI_USDT_USDC2, vec![100*ONE_DAI, 100*ONE_USDT, 100*ONE_USDC].into_iter().map(|x| U128(x)).collect(), U128(1)),
        deposit = to_yocto("0.0007")
    )
    .assert_success();

    let new_user = root.create_user("new_user".to_string(), to_yocto("100"));
    call!(
        new_user,
        pool.storage_deposit(None, None),
        deposit = to_yocto("1")
    )
    .assert_success();
    call!(
        new_user,
        token_dai.mint(to_va(new_user.account_id.clone()), U128(to_yocto("10")))
    )
    .assert_success();

    let out_usdt_amount = view!(pool.get_return(DAI_USDT, to_va(token_dai.account_id().clone()), U128(5*ONE_DAI), to_va(token_usdt.account_id().clone()))).unwrap_json::<U128>().0;
    let out_usdc_amount = view!(pool.get_return(DAI_USDC, to_va(token_dai.account_id().clone()), U128(5*ONE_DAI), to_va(token_usdc.account_id().clone()))).unwrap_json::<U128>().0;
    println!("out_usdt_amount: {:?}", out_usdt_amount);
    println!("out_usdc_amount: {:?}", out_usdc_amount);
    println!("{:?}", view!(pool.predict_add_stable_liquidity(DAI_USDT_USDC, &vec![U128(5*ONE_DAI / 2), U128(4750565 / 2), U128(4750565 / 2)])).unwrap_json::<U128>().0);
    println!("{:?}", view!(pool.predict_add_stable_liquidity(DAI_USDT_USDC2, &vec![U128(5*ONE_DAI / 2), U128(4750565 / 2), U128(4750565 / 2)])).unwrap_json::<U128>().0);

    println!("predict hot zap: {:?}", view!(pool.predict_hot_zap(
        None,
        None,
        token_dai.valid_account_id(),
        U128(15*ONE_DAI),
        vec![
            Action::Swap(SwapAction { 
                pool_id: DAI_USDT, 
                token_in: token_dai.account_id(), 
                amount_in: Some(U128(5*ONE_DAI)), 
                token_out: token_usdt.account_id(), 
                min_amount_out: U128(1) 
            }),
            Action::Swap(SwapAction { 
                pool_id: DAI_USDC, 
                token_in: token_dai.account_id(), 
                amount_in: Some(U128(5*ONE_DAI)), 
                token_out: token_usdc.account_id(), 
                min_amount_out: U128(1) 
            })
        ],
        vec![
            AddLiquidityInfo {
                pool_id: 0,
                amounts: vec![U128(5*ONE_DAI / 2), U128(4750565 / 2), U128(4750565 / 2)],
                min_amounts: None,
                min_shares: Some(U128(0))
            },
            AddLiquidityInfo {
                pool_id: 3,
                amounts: vec![U128(5*ONE_DAI / 2), U128(4750565 / 2), U128(4750565 / 2)],
                min_amounts: None,
                min_shares: Some(U128(0))
            }
        ]
    )).unwrap_json::<Option<(Vec<AddLiquidityPrediction>, HashMap<AccountId, U128>)>>());

    let mock_boost_farming = deploy!(
        contract: MockBoostFarming,
        contract_id: boost_farming(),
        bytes: &MOCK_BOOST_FARMING_WASM_BYTES,
        signer_account: root,
        init_method: new(root.account_id())
    );

    let seed_id = "swap@0".to_string();
    let outcome = call!(
        root,
        pool.mft_register(":0".to_string(), mock_boost_farming.valid_account_id()),
        deposit = to_yocto("1")
    );
    outcome.assert_success();

    let outcome = call!(
        new_user,
        mock_boost_farming.storage_deposit(None, None),
        deposit = 100_000_000_000_000_000_000_000
    );
    outcome.assert_success();

    let outcome = call!(
        root,
        mock_boost_farming.create_seed(seed_id.clone(),24, Some(U128(0)), Some(0)),
        deposit = 1
    );
    outcome.assert_success();

    let outcome = call!(
        new_user,
        token_dai.ft_transfer_call(
            to_va(swap()),
            U128(15*ONE_DAI),
            None,
            json!({
                "hot_zap_actions": [
                    {"pool_id": DAI_USDT, "token_in": token_dai.account_id(), "amount_in": (5*ONE_DAI).to_string(), "token_out": token_usdt.account_id(), "min_amount_out":"1"},
                    {"pool_id": DAI_USDC, "token_in": token_dai.account_id(), "amount_in": (5*ONE_DAI).to_string(), "token_out": token_usdc.account_id(), "min_amount_out":"1"}
                ],
                "add_liquidity_infos": vec![
                    AddLiquidityInfo {
                        pool_id: 0,
                        amounts: vec![U128(5*ONE_DAI / 2), U128(4750565 / 2), U128(4750565 / 2)],
                        min_amounts: None,
                        min_shares: Some(U128(0))
                    },
                    AddLiquidityInfo {
                        pool_id: 3,
                        amounts: vec![U128(5*ONE_DAI / 2), U128(4750565 / 2), U128(4750565 / 2)],
                        min_amounts: None,
                        min_shares: Some(U128(0))
                    }
                ],
            }).to_string()
        ),
        deposit = 1
    );
    
    println!("{:#?}", get_logs(&outcome));
    assert!(outcome.promise_errors().is_empty());

    println!("before mft_transfer_all_call: {:?}", view!(mock_boost_farming.get_seed(seed_id.clone())).unwrap_json::<Value>());

    let outcome = call!(
        new_user,
        pool.mft_transfer_all_call(":0".to_string(), mock_boost_farming.valid_account_id(), None, "\"Free\"".to_string()),
        1,
        near_sdk_sim::DEFAULT_GAS
    );
    outcome.assert_success();

    println!("after mft_transfer_all_call: {:?}", view!(mock_boost_farming.get_seed(seed_id)).unwrap_json::<Value>());
    println!("view new_user deposit: {:?}", view!(pool.get_deposits(new_user.valid_account_id())).unwrap_json::<HashMap<AccountId, U128>>());
}

#[test]
fn test_hotzap_rate_pool() {
    let (root, owner, pool, tokens, token_rated_contracts) = 
        setup_rated_pool(
            vec![near()],
            vec![stnear()],
            vec![24, 24],
            25,
            10000,
        );

    let token_near = &tokens[0];
    let token_stnear = &token_rated_contracts[0];

    call!(
        owner,
        pool.register_rated_token(
            "STNEAR".to_string(),
            token_rated_contracts[0].valid_account_id(),
            None
        ),
        deposit = 1
    ).assert_success();

    call!(
        root,
        token_stnear.set_price(U128(2 * 10u128.pow(24)))
    ).assert_success();

    call!(
        owner,
        pool.update_token_rate(
            token_stnear.valid_account_id()
        ),
        deposit = 1
    ).assert_success();

    call!(
        root,
        pool.add_simple_pool(vec![to_va(stnear()), to_va(near())], 25),
        deposit = to_yocto("1")
    )
    .assert_success();
    
    let outcome = call!(
        root,
        token_stnear.ft_transfer_call(pool.valid_account_id(), U128(to_yocto("100")), None, "".to_string()),
        deposit = 1
    );
    outcome.assert_success();
    // println!("token_linear: {:?}", outcome.promise_errors()[0].as_ref().unwrap().status());



    let outcome = call!(
        root,
        token_near.ft_transfer_call(pool.valid_account_id(), U128(to_yocto("100")), None, "".to_string()),
        deposit = 1
    );
    outcome.assert_success();
    // println!("token_near: {:?}", outcome.promise_errors()[0].as_ref().unwrap().status());

    call!(
        root,
        pool.add_liquidity(1, vec![U128(to_yocto("100")), U128(to_yocto("100"))], None),
        deposit = to_yocto("0.0007")
    )
    .assert_success();

    println!("{:?}", view!(pool.get_pool(0)).unwrap_json::<Value>());
    println!("{:?}", view!(pool.get_pool(1)).unwrap_json::<Value>());

    let new_user = root.create_user("new_user".to_string(), to_yocto("100"));
    call!(
        new_user,
        pool.storage_deposit(None, None),
        deposit = to_yocto("1")
    )
    .assert_success();

    call!(
        new_user,
        token_near.mint(to_va(new_user.account_id.clone()), U128(to_yocto("10")))
    )
    .assert_success();

    let out_linear_amount = view!(pool.get_return(1, to_va(token_near.account_id().clone()), U128(to_yocto("5")), to_va(token_stnear.account_id().clone()))).unwrap_json::<U128>().0;
    println!("out_linear_amount: {:?}", out_linear_amount);
    println!("{:?}", view!(pool.predict_add_stable_liquidity(0, &vec![U128(to_yocto("5")), U128(4750565543517085367305631u128)])).unwrap_json::<U128>().0);

    println!("predict hot zap: {:?}", view!(pool.predict_hot_zap(
        None,
        None,
        token_near.valid_account_id(),
        U128(to_yocto("10")),
        vec![
            Action::Swap(SwapAction { 
                pool_id: 1, 
                token_in: token_near.account_id(), 
                amount_in: Some(U128(to_yocto("5"))), 
                token_out: token_stnear.account_id(), 
                min_amount_out: U128(1) 
            }),
        ],
        vec![
            AddLiquidityInfo {
                pool_id: 0,
                amounts: vec![U128(to_yocto("5")), U128(4750565543517085367305631u128)],
                min_amounts: None,
                min_shares: Some(U128(0))
            }
        ]
    )).unwrap_json::<Option<(Vec<AddLiquidityPrediction>, HashMap<AccountId, U128>)>>());

    let mock_boost_farming = deploy!(
        contract: MockBoostFarming,
        contract_id: boost_farming(),
        bytes: &MOCK_BOOST_FARMING_WASM_BYTES,
        signer_account: root,
        init_method: new(root.account_id())
    );

    let seed_id = "swap@0".to_string();
    let outcome = call!(
        root,
        pool.mft_register(":0".to_string(), mock_boost_farming.valid_account_id()),
        deposit = to_yocto("1")
    );
    outcome.assert_success();

    let outcome = call!(
        new_user,
        mock_boost_farming.storage_deposit(None, None),
        deposit = 100_000_000_000_000_000_000_000
    );
    outcome.assert_success();

    let outcome = call!(
        root,
        mock_boost_farming.create_seed(seed_id.clone(),24, Some(U128(0)), Some(0)),
        deposit = 1
    );
    outcome.assert_success();

    let outcome = call!(
        new_user,
        token_near.ft_transfer_call(
            to_va(swap()),
            U128(to_yocto("10")),
            None,
            json!({
                "hot_zap_actions": [
                    {"pool_id": 1, "token_in": token_near.account_id(), "amount_in": to_yocto("5").to_string(), "token_out": token_stnear.account_id(), "min_amount_out":"1"},
                ],
                "add_liquidity_infos": vec![
                    AddLiquidityInfo {
                        pool_id: 0,
                        amounts: vec![U128(to_yocto("5")), U128(4750565543517085367305631u128)],
                        min_amounts: None,
                        min_shares: Some(U128(0))
                    }
                ],
            }).to_string()
        ),
        deposit = 1
    );
    
    println!("{:#?}", get_logs(&outcome));
    assert!(outcome.promise_errors().is_empty());

    println!("before mft_transfer_all_call: {:?}", view!(mock_boost_farming.get_seed(seed_id.clone())).unwrap_json::<Value>());

    let outcome = call!(
        new_user,
        pool.mft_transfer_all_call(":0".to_string(), mock_boost_farming.valid_account_id(), None, "\"Free\"".to_string()),
        1,
        near_sdk_sim::DEFAULT_GAS
    );
    outcome.assert_success();

    println!("after mft_transfer_all_call: {:?}", view!(mock_boost_farming.get_seed(seed_id)).unwrap_json::<Value>());
    println!("view new_user deposit: {:?}", view!(pool.get_deposits(new_user.valid_account_id())).unwrap_json::<HashMap<AccountId, U128>>());
}

#[test]
fn test_hotzap_rate_pool_add_two() {
    let (root, owner, pool, tokens, token_rated_contracts) = 
        setup_rated_pool(
            vec![near()],
            vec![stnear()],
            vec![24, 24],
            25,
            10000,
        );

    let token_near = &tokens[0];
    let token_stnear = &token_rated_contracts[0];

    call!(
        owner,
        pool.register_rated_token(
            "STNEAR".to_string(),
            token_rated_contracts[0].valid_account_id(),
            None
        ),
        deposit = 1
    ).assert_success();

    call!(
        root,
        token_stnear.set_price(U128(2 * 10u128.pow(24)))
    ).assert_success();

    call!(
        owner,
        pool.update_token_rate(
            token_stnear.valid_account_id()
        ),
        deposit = 1
    ).assert_success();

    call!(
        root,
        pool.add_simple_pool(vec![to_va(stnear()), to_va(near())], 25),
        deposit = to_yocto("1")
    )
    .assert_success();

    call!(
        owner,
        pool.add_rated_swap_pool(
            vec![near(), stnear()].into_iter().map(|x| x.try_into().unwrap()).collect(), 
            vec![24, 24],
            25,
            10000
        ),
        deposit = to_yocto("1"))
    .assert_success();
    
    let outcome = call!(
        root,
        token_stnear.ft_transfer_call(pool.valid_account_id(), U128(to_yocto("100")), None, "".to_string()),
        deposit = 1
    );
    outcome.assert_success();
    // println!("token_linear: {:?}", outcome.promise_errors()[0].as_ref().unwrap().status());



    let outcome = call!(
        root,
        token_near.ft_transfer_call(pool.valid_account_id(), U128(to_yocto("100")), None, "".to_string()),
        deposit = 1
    );
    outcome.assert_success();
    // println!("token_near: {:?}", outcome.promise_errors()[0].as_ref().unwrap().status());

    call!(
        root,
        pool.add_liquidity(1, vec![U128(to_yocto("100")), U128(to_yocto("100"))], None),
        deposit = to_yocto("0.0007")
    )
    .assert_success();

    println!("{:?}", view!(pool.get_pool(0)).unwrap_json::<Value>());
    println!("{:?}", view!(pool.get_pool(1)).unwrap_json::<Value>());

    let new_user = root.create_user("new_user".to_string(), to_yocto("100"));
    call!(
        new_user,
        pool.storage_deposit(None, None),
        deposit = to_yocto("1")
    )
    .assert_success();

    call!(
        new_user,
        token_near.mint(to_va(new_user.account_id.clone()), U128(to_yocto("10")))
    )
    .assert_success();

    let out_linear_amount = view!(pool.get_return(1, to_va(token_near.account_id().clone()), U128(to_yocto("5")), to_va(token_stnear.account_id().clone()))).unwrap_json::<U128>().0;
    println!("out_linear_amount: {:?}", out_linear_amount);
    println!("{:?}", view!(pool.predict_add_stable_liquidity(0, &vec![U128(to_yocto("5") / 2), U128(4750565543517085367305631u128 / 2)])).unwrap_json::<U128>().0);
    println!("{:?}", view!(pool.predict_add_stable_liquidity(2, &vec![U128(to_yocto("5") / 2), U128(4750565543517085367305631u128 / 2)])).unwrap_json::<U128>().0);

    println!("predict hot zap: {:?}", view!(pool.predict_hot_zap(
        None,
        None,
        token_near.valid_account_id(),
        U128(to_yocto("10")),
        vec![
            Action::Swap(SwapAction { 
                pool_id: 1, 
                token_in: token_near.account_id(), 
                amount_in: Some(U128(to_yocto("5"))), 
                token_out: token_stnear.account_id(), 
                min_amount_out: U128(1) 
            }),
        ],
        vec![
            AddLiquidityInfo {
                pool_id: 0,
                amounts: vec![U128(to_yocto("5") / 2), U128(4750565543517085367305631u128 / 2)],
                min_amounts: None,
                min_shares: Some(U128(0))
            },
            AddLiquidityInfo {
                pool_id: 2,
                amounts: vec![U128(to_yocto("5") / 2), U128(4750565543517085367305631u128 / 2)],
                min_amounts: None,
                min_shares: Some(U128(0))
            }
        ]
    )).unwrap_json::<Option<(Vec<AddLiquidityPrediction>, HashMap<AccountId, U128>)>>());

    let mock_boost_farming = deploy!(
        contract: MockBoostFarming,
        contract_id: boost_farming(),
        bytes: &MOCK_BOOST_FARMING_WASM_BYTES,
        signer_account: root,
        init_method: new(root.account_id())
    );

    let seed_id = "swap@0".to_string();
    let outcome = call!(
        root,
        pool.mft_register(":0".to_string(), mock_boost_farming.valid_account_id()),
        deposit = to_yocto("1")
    );
    outcome.assert_success();

    let outcome = call!(
        new_user,
        mock_boost_farming.storage_deposit(None, None),
        deposit = 100_000_000_000_000_000_000_000
    );
    outcome.assert_success();

    let outcome = call!(
        root,
        mock_boost_farming.create_seed(seed_id.clone(),24, Some(U128(0)), Some(0)),
        deposit = 1
    );
    outcome.assert_success();

    let outcome = call!(
        new_user,
        token_near.ft_transfer_call(
            to_va(swap()),
            U128(to_yocto("10")),
            None,
            json!({
                "hot_zap_actions": [
                    {"pool_id": 1, "token_in": token_near.account_id(), "amount_in": to_yocto("5").to_string(), "token_out": token_stnear.account_id(), "min_amount_out":"1"},
                ],
                "add_liquidity_infos": vec![
                    AddLiquidityInfo {
                        pool_id: 0,
                        amounts: vec![U128(to_yocto("5") / 2), U128(4750565543517085367305631u128 / 2)],
                        min_amounts: None,
                        min_shares: Some(U128(0))
                    },
                    AddLiquidityInfo {
                        pool_id: 2,
                        amounts: vec![U128(to_yocto("5") / 2), U128(4750565543517085367305631u128 / 2)],
                        min_amounts: None,
                        min_shares: Some(U128(0))
                    }
                ],
            }).to_string()
        ),
        deposit = 1
    );
    
    println!("{:#?}", get_logs(&outcome));
    assert!(outcome.promise_errors().is_empty());

    println!("before mft_transfer_all_call: {:?}", view!(mock_boost_farming.get_seed(seed_id.clone())).unwrap_json::<Value>());

    let outcome = call!(
        new_user,
        pool.mft_transfer_all_call(":0".to_string(), mock_boost_farming.valid_account_id(), None, "\"Free\"".to_string()),
        1,
        near_sdk_sim::DEFAULT_GAS
    );
    outcome.assert_success();

    println!("after mft_transfer_all_call: {:?}", view!(mock_boost_farming.get_seed(seed_id)).unwrap_json::<Value>());
    println!("view new_user deposit: {:?}", view!(pool.get_deposits(new_user.valid_account_id())).unwrap_json::<HashMap<AccountId, U128>>());
}

#[test]
fn test_hotzap_simple_pool_frozen_token() {
    const DAI_ETH: u64 = 0;
    const ETH_USDT: u64 = 1;
    const DAI_USDT: u64 = 2;
    let (root, owner, pool, token_dai, token_eth, token_usdt) = setup_pool_with_liquidity();
    let new_user = root.create_user("new_user".to_string(), to_yocto("100"));
    call!(
        new_user,
        pool.storage_deposit(None, None),
        deposit = to_yocto("1")
    )
    .assert_success();
    call!(
        new_user,
        token_dai.mint(to_va(new_user.account_id.clone()), U128(to_yocto("10")))
    )
    .assert_success();

    let out_eth_amount = view!(pool.get_return(DAI_ETH, to_va(token_dai.account_id().clone()), U128(to_yocto("1")), to_va(token_eth.account_id().clone()))).unwrap_json::<U128>().0;
    let out_usdt_amount = view!(pool.get_return(DAI_USDT, to_va(token_dai.account_id().clone()), U128(to_yocto("1")), to_va(token_usdt.account_id().clone()))).unwrap_json::<U128>().0;
    let add_liquidity_prediction = view!(pool.predict_add_simple_liquidity(ETH_USDT, &vec![U128(out_eth_amount), U128(out_usdt_amount)])).unwrap_json::<AddLiquidityPrediction>();

    let out_come = call!(
        owner,
        pool.extend_frozenlist_tokens(vec![to_va(dai())]),
        deposit=1
    );
    out_come.assert_success();

    let outcome = call!(
        new_user,
        token_dai.ft_transfer_call(
            to_va(swap()),
            to_yocto("2").into(),
            None,
            json!({
                "hot_zap_actions": [
                    {"pool_id": 0, "token_in": "dai001", "amount_in":"1000000000000000000000000", "token_out":"eth002", "min_amount_out":"1"},
                    {"pool_id": 2, "token_in": "dai001", "amount_in":"1000000000000000000000000", "token_out":"usdt", "min_amount_out":"1"}
                ],
                "add_liquidity_infos": vec![
                    AddLiquidityInfo {
                        pool_id: 1,
                        amounts: add_liquidity_prediction.need_amounts.clone(),
                        min_amounts: Some(vec![U128(1814048647419868151852681u128), U128(907024323709934075926341u128)]),
                        min_shares: None
                    }
                ],
            }).to_string()
        ),
        1,
        near_sdk_sim::DEFAULT_GAS
    );
    let exe_status = format!("{:?}", outcome.promise_errors()[0].as_ref().unwrap().status());
    assert!(exe_status.contains("E52: token frozen"));
}

#[test]
fn test_hotzap_simple_pool_not_whitelisted_token() {
    const DAI_ETH: u64 = 0;
    const ETH_USDT: u64 = 1;
    const DAI_USDT: u64 = 2;
    let (root, owner, pool, token_dai, token_eth, token_usdt) = setup_pool_with_liquidity();
    let new_user = root.create_user("new_user".to_string(), to_yocto("100"));
    call!(
        new_user,
        pool.storage_deposit(None, None),
        deposit = to_yocto("1")
    )
    .assert_success();
    call!(
        new_user,
        token_dai.mint(to_va(new_user.account_id.clone()), U128(to_yocto("10")))
    )
    .assert_success();

    let out_eth_amount = view!(pool.get_return(DAI_ETH, to_va(token_dai.account_id().clone()), U128(to_yocto("1")), to_va(token_eth.account_id().clone()))).unwrap_json::<U128>().0;
    let out_usdt_amount = view!(pool.get_return(DAI_USDT, to_va(token_dai.account_id().clone()), U128(to_yocto("1")), to_va(token_usdt.account_id().clone()))).unwrap_json::<U128>().0;
    let add_liquidity_prediction = view!(pool.predict_add_simple_liquidity(ETH_USDT, &vec![U128(out_eth_amount), U128(out_usdt_amount)])).unwrap_json::<AddLiquidityPrediction>();

    let out_come = call!(
        owner,
        pool.remove_whitelisted_tokens(vec![to_va(dai())]),
        deposit = 1
    );
    out_come.assert_success();

    let outcome = call!(
        new_user,
        token_dai.ft_transfer_call(
            to_va(swap()),
            to_yocto("2").into(),
            None,
            json!({
                "hot_zap_actions": [
                    {"pool_id": 0, "token_in": "dai001", "amount_in":"1000000000000000000000000", "token_out":"eth002", "min_amount_out":"1"},
                    {"pool_id": 2, "token_in": "dai001", "amount_in":"1000000000000000000000000", "token_out":"usdt", "min_amount_out":"1"}
                ],
                "add_liquidity_infos": vec![
                    AddLiquidityInfo {
                        pool_id: 1,
                        amounts: add_liquidity_prediction.need_amounts.clone(),
                        min_amounts: Some(vec![U128(1814048647419868151852681u128), U128(907024323709934075926341u128)]),
                        min_shares: None
                    }
                ],
            }).to_string()
        ),
        1,
        near_sdk_sim::DEFAULT_GAS
    );
    let exe_status = format!("{:?}", outcome.promise_errors()[0].as_ref().unwrap().status());
    assert!(exe_status.contains("E12: token not whitelisted"));
}