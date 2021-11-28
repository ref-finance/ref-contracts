use near_sdk_sim::{
    call, to_yocto, view, ContractAccount, ExecutionResult, UserAccount,
};
use test_token::ContractContract as TestToken;
use near_sdk::json_types::U128;
use near_sdk::AccountId;
use std::{collections::HashMap, convert::TryInto, process::id};
use ref_exchange::{ContractContract as Exchange, PoolInfo};
use rand::Rng;
use rand_pcg::Pcg32;
use crate::fuzzy::{
    types::*,
    utils::*,
    constants::*
};
use std::cmp::min;
use std::panic;

use uint::construct_uint;
construct_uint! {
    pub struct U256(4);
}

pub fn add_liquidity_action(pool :&ContractAccount<Exchange>, operator: &Operator, simple_pool_id: u64, liquidity1: u128, liquidity2: u128) -> ExecutionResult {
    call!(
        &operator.user,
        pool.add_liquidity(simple_pool_id, vec![U128(liquidity1), U128(liquidity2)], None),
        deposit = to_yocto("0.0009")// < 0.0009 ERR_STORAGE_DEPOSIT
    )
}

pub fn real_liquidity(pool :&ContractAccount<Exchange>, pool_id: u64, amounts: Vec<u128>) -> Option<(u128, u128)>{
    let mut res = (0, 0);
    let simple_pool_info = view!(pool.get_pool(pool_id)).unwrap_json::<PoolInfo>();

    if u128::from(simple_pool_info.shares_total_supply) > 0{
        let mut fair_supply = U256::max_value();
        for i in 0..simple_pool_info.token_account_ids.len() {
            fair_supply = min(
                fair_supply,
                U256::from(amounts[i]) * U256::from(simple_pool_info.shares_total_supply.0) / simple_pool_info.amounts[i].0,
            );
        }
        for i in 0..simple_pool_info.token_account_ids.len() {
            let amount = (U256::from(simple_pool_info.amounts[i].0) * fair_supply
                / U256::from(simple_pool_info.shares_total_supply.0))
            .as_u128();
            if i == 0 {
                res.0 = amount;
            }else{
                res.1 = amount;
            }
        }
    }else{
        return None;
    }
    Some(res)
}

pub fn do_add_liquidity(ctx: &mut OperationContext, rng: &mut Pcg32, root: &UserAccount, operator: &Operator, pool :&ContractAccount<Exchange>, simple_pool_count: u64, specified: Option<u64>){
    let simple_pool_id = match specified{
        Some(id) => id,
        None => {
            if simple_pool_count == 0 { 0 } else { rng.gen_range(0..simple_pool_count) }
        }
    };
    
    let simple_pool_info = view!(pool.get_pool(simple_pool_id)).unwrap_json::<PoolInfo>();

    let tokens = simple_pool_info.token_account_ids;

    let (liquidity1, liquidity2) = (to_yocto(&ADD_LIQUIDITY_LIMIT.to_string()), to_yocto(&ADD_LIQUIDITY_LIMIT.to_string()));

    loop{
        let (scenario, token1_account, token2_account, token1_deposit, token2_deposit) = 
            current_evn_info(ctx, pool, operator, &tokens);
        println!("add_liquidity scenario : {:?} begin!", scenario);
        
        match scenario {
            Scenario::Normal => {

                let (real_liquidity1, real_liquidity2) = match real_liquidity(pool, simple_pool_id, vec![liquidity1, liquidity2]){
                    Some((real_liquidity1, real_liquidity2)) => (real_liquidity1, real_liquidity2),
                    None => (liquidity1, liquidity2)
                };

                if token1_deposit <  real_liquidity1{
                    let out_come = add_liquidity_action(pool, operator, simple_pool_id, liquidity1, liquidity2);
                    assert_eq!(get_error_count(&out_come), 1);
                    assert!(get_error_status(&out_come).contains("E22: not enough tokens in deposit"));
                    add_token_deposit(ctx, root, operator, tokens.get(0).unwrap(), token1_account, liquidity1, token1_deposit);
                }

                if token2_deposit < real_liquidity2 {
                    let out_come = add_liquidity_action(pool, operator, simple_pool_id, liquidity1, liquidity2);
                    assert_eq!(get_error_count(&out_come), 1);
                    assert!(get_error_status(&out_come).contains("E22: not enough tokens in deposit"));
                    add_token_deposit(ctx, root, operator, tokens.get(1).unwrap(), token2_account, liquidity2, token2_deposit);
                }

                add_liquidity_action(pool, operator, simple_pool_id, liquidity1, liquidity2).assert_success();

                println!("add_liquidity scenario : Normal end!");
                let new_simple_pool_info = view!(pool.get_pool(simple_pool_id)).unwrap_json::<PoolInfo>();
                println!("after add liquidity current simple pool info {:?} ", new_simple_pool_info);
                break;
            },
            Scenario::Token1NoAccount => {
                let out_come = add_liquidity_action(pool, operator, simple_pool_id, liquidity1, liquidity2);
                assert_eq!(get_error_count(&out_come), 1);
                assert!(get_error_status(&out_come).contains("token not registered"));

                user_init_token_account(ctx, root, operator,  tokens.get(0).unwrap());
            },
            Scenario::Token2NoAccount => {
                let (real_liquidity1, _) = match real_liquidity(pool, simple_pool_id, vec![liquidity1, liquidity2]){
                    Some((real_liquidity1, real_liquidity2)) => (real_liquidity1, real_liquidity2),
                    None => (liquidity1, liquidity2)
                };
                let token1_deposit = view!(pool.get_deposit(to_va(operator.user.account_id.clone()), to_va(tokens.get(0).unwrap().clone()))).unwrap_json::<U128>().0;
                let out_come = add_liquidity_action(pool, operator, simple_pool_id, liquidity1, liquidity2);
                
                if token1_deposit != 0 && token1_deposit < real_liquidity1 {
                    assert_eq!(get_error_count(&out_come), 1);
                    assert!(get_error_status(&out_come).contains("E22: not enough tokens in deposit"));
                }else {
                    assert_eq!(get_error_count(&out_come), 1);
                    assert!(get_error_status(&out_come).contains("token not registered"));
                }

                user_init_token_account(ctx, root, operator,  tokens.get(1).unwrap());
            },
            Scenario::Token1NotRegistered => {
                let out_come = add_liquidity_action(pool, operator, simple_pool_id, liquidity1, liquidity2);
                assert_eq!(get_error_count(&out_come), 1);
                assert!(get_error_status(&out_come).contains("token not registered"));

                user_init_deposit_token(ctx, rng, operator, tokens.get(0).unwrap());
            },
            Scenario::Token2NotRegistered => {
                let (real_liquidity1, _) = match real_liquidity(pool, simple_pool_id, vec![liquidity1, liquidity2]){
                    Some((real_liquidity1, real_liquidity2)) => (real_liquidity1, real_liquidity2),
                    None => (liquidity1, liquidity2)
                };
                let token1_deposit = view!(pool.get_deposit(to_va(operator.user.account_id.clone()), to_va(tokens.get(0).unwrap().clone()))).unwrap_json::<U128>().0;
                let out_come = add_liquidity_action(pool, operator, simple_pool_id, liquidity1, liquidity2);

                if token1_deposit != 0 && token1_deposit < real_liquidity1 {
                    assert_eq!(get_error_count(&out_come), 1);
                    assert!(get_error_status(&out_come).contains("E22: not enough tokens in deposit"));
                }else {
                    assert_eq!(get_error_count(&out_come), 1);
                    assert!(get_error_status(&out_come).contains("token not registered"));
                }

                user_init_deposit_token(ctx, rng, operator, tokens.get(1).unwrap());
            },
            Scenario::NoStorageDeposit => {
                let out_come = add_liquidity_action(pool, operator, simple_pool_id, liquidity1, liquidity2);
                assert_eq!(get_error_count(&out_come), 1);
                assert!(get_error_status(&out_come).contains("token not registered"));
                user_storage_deposit(pool, operator);
            }
        }
        println!("add_liquidity scenario : {:?} end!", scenario);
    }
}

pub fn do_stable_add_liquidity(token_contracts: &Vec<ContractAccount<TestToken>>, rng: &mut Pcg32, root: &UserAccount, operator: &StableOperator, pool :&ContractAccount<Exchange>) -> u128{
    let mut scenario = StableScenario::Normal;
    
    let add_amounts = vec![U128(rng.gen_range(1..ADD_LIQUIDITY_LIMIT as u128) * ONE_DAI),
                                    U128(rng.gen_range(1..ADD_LIQUIDITY_LIMIT as u128) * ONE_USDT),
                                    U128(rng.gen_range(1..ADD_LIQUIDITY_LIMIT as u128) * ONE_USDC)];

    let min_shares = rng.gen_range(1..ADD_LIQUIDITY_LIMIT) as u128;

    let old_share = mft_balance_of(pool, ":0", &operator.user.account_id());

    println!("do_stable_add_liquidity add_amounts : {:?}", add_amounts);
    for (idx, amount) in add_amounts.clone().into_iter().enumerate() {
        let token_contract = token_contracts.get(idx).unwrap();
        add_and_deposit_token(root, &operator.user, token_contract, pool, amount.0);
    }

    let cal_share = view!(pool.predict_add_stable_liqudity(0, &add_amounts)).unwrap_json::<U128>().0;

    if min_shares > cal_share {
        scenario = StableScenario::Slippage;
    }

    let out_come = call!(
        operator.user,
        pool.add_stable_liquidity(0, add_amounts, U128(min_shares)),
        deposit = to_yocto("0.01")
    );

    let mut share = 0;
    match scenario {
        StableScenario::Normal => {
            share = out_come.unwrap_json::<U128>().0;
            assert_eq!(cal_share, share);
            assert_eq!(mft_balance_of(pool, ":0", &operator.user.account_id()), old_share + share);
        },
        StableScenario::Slippage => {
            assert_eq!(get_error_count(&out_come), 1);
            assert!(get_error_status(&out_come).contains("E68: slippage error"));
        },
        StableScenario::InsufficientLpShares => {
        }
    }
    share
}

pub fn do_stable_remove_liquidity_by_shares(token_contracts: &Vec<ContractAccount<TestToken>>, rng: &mut Pcg32, root: &UserAccount, operator: &StableOperator, pool :&ContractAccount<Exchange>){
    let mut scenario = StableScenario::Normal;

    let min_amounts = vec![U128(1*ONE_DAI), U128(1*ONE_USDT), U128(1*ONE_USDC)];
    let remove_lp_num = rng.gen_range(1..LP_LIMIT) * ONE_LPT * 10;

    let mut user_lpt =  mft_balance_of(&pool, ":0", &operator.user.account_id());

    while user_lpt == 0 {
        user_lpt = do_stable_add_liquidity(token_contracts, rng, root, operator, pool);
    }

    let old_balances = view!(pool.get_deposits(operator.user.valid_account_id())).unwrap_json::<HashMap<AccountId, U128>>();
    let old_share = mft_balance_of(pool, ":0", &operator.user.account_id());

    if user_lpt < remove_lp_num {
        scenario = StableScenario::InsufficientLpShares;
    }else{
        let total_supply = mft_total_supply(pool, ":0");
        let mut result = vec![0u128; STABLE_TOKENS.len()];
        let amounts  = view!(pool.get_pool(0)).unwrap_json::<PoolInfo>().amounts;
        for i in 0..STABLE_TOKENS.len() {
            result[i] = U256::from(amounts[i].0)
                .checked_mul(remove_lp_num.into())
                .unwrap()
                .checked_div(total_supply.into())
                .unwrap()
                .as_u128();
            if result[i] < min_amounts[i].0 {
                scenario = StableScenario::Slippage;
                break;
            }
        }
    }
    
    let mut increase_amounts = vec![];
    if scenario == StableScenario::Normal {
        increase_amounts = view!(pool.predict_remove_liqudity(0, U128(remove_lp_num))).unwrap_json::<Vec<U128>>();
    }

    println!("user has lpt : {}, remove : {}", user_lpt, remove_lp_num);

    let out_come = call!(
        operator.user,
        pool.remove_liquidity(0, U128(remove_lp_num), min_amounts),
        deposit = 1 
    );

    println!("do_stable_remove_liquidity_by_shares scenario : {:?} begin!", scenario);
    match scenario {
        StableScenario::Normal => {
            out_come.assert_success();
            let user_share = mft_balance_of(pool, ":0", &operator.user.account_id());
            let balances = view!(pool.get_deposits(operator.user.valid_account_id())).unwrap_json::<HashMap<AccountId, U128>>();
            assert_eq!(user_share, old_share - remove_lp_num);
            for (idx, item) in increase_amounts.iter().enumerate() {
                assert_eq!(balances.get(STABLE_TOKENS[idx]).unwrap().0, 
                old_balances.get(STABLE_TOKENS[idx]).unwrap().0 + item.0);
            }
        },
        StableScenario::Slippage => {
            assert_eq!(get_error_count(&out_come), 1);
            assert!(get_error_status(&out_come).contains("E68: slippage error"));
        },
        StableScenario::InsufficientLpShares => {
            assert_eq!(get_error_count(&out_come), 1);
            assert!(get_error_status(&out_come).contains("E34: insufficient lp shares"));
        }
    }
    println!("do_stable_remove_liquidity_by_shares scenario : {:?} end!", scenario);
}

pub fn do_stable_remove_liquidity_by_token(token_contracts: &Vec<ContractAccount<TestToken>>, rng: &mut Pcg32, root: &UserAccount, operator: &StableOperator, pool :&ContractAccount<Exchange>){

    let mut scenario = StableScenario::Normal;

    let remove_amounts = vec![ U128(rng.gen_range(1..REMOVE_LIQUIDITY_LIMIT as u128) * ONE_DAI),
                                        U128(rng.gen_range(1..REMOVE_LIQUIDITY_LIMIT as u128) * ONE_USDT),
                                        U128(rng.gen_range(1..REMOVE_LIQUIDITY_LIMIT as u128) * ONE_USDC)];
    let max_burn_shares = rng.gen_range(1..LP_LIMIT as u128) * ONE_LPT;
    let mut user_lpt =  mft_balance_of(&pool, ":0", &operator.user.account_id());

    while user_lpt == 0 {
        user_lpt = do_stable_add_liquidity(token_contracts, rng, root, operator, pool);
    }

    let old_balances = view!(pool.get_deposits(operator.user.valid_account_id())).unwrap_json::<HashMap<AccountId, U128>>();

    let remove_lpt = view!(pool.predict_remove_liqudity_by_tokens(0, &remove_amounts)).unwrap_json::<U128>().0;

    if remove_lpt > user_lpt{
        scenario = StableScenario::InsufficientLpShares;
    }else if remove_lpt > max_burn_shares{
        scenario = StableScenario::Slippage;
    }

    println!("remove tokens: {:?}", remove_amounts);
    println!("remove lpt: {} {} {}", user_lpt, remove_lpt, max_burn_shares);

    let out_come = call!(
        operator.user,
        pool.remove_liquidity_by_tokens(0, remove_amounts.clone(), U128(max_burn_shares)),
        deposit = 1 
    );

    println!("do_stable_remove_liquidity_by_token scenario : {:?} begin!", scenario);
    match scenario {
        StableScenario::Normal => {
            out_come.assert_success();
            let current_share = mft_balance_of(&pool, ":0", &operator.user.account_id());
            assert_eq!(current_share, user_lpt - remove_lpt);
            let balances = view!(pool.get_deposits(operator.user.valid_account_id())).unwrap_json::<HashMap<AccountId, U128>>();
            for (idx, item) in remove_amounts.iter().enumerate() {
                assert_eq!(balances.get(STABLE_TOKENS[idx]).unwrap().0, 
                old_balances.get(STABLE_TOKENS[idx]).unwrap().0 + item.0);
            }
        },
        StableScenario::Slippage => {
            assert_eq!(get_error_count(&out_come), 1);
            assert!(get_error_status(&out_come).contains("E68: slippage error"));
        },
        StableScenario::InsufficientLpShares => {
            assert_eq!(get_error_count(&out_come), 1);
            assert!(get_error_status(&out_come).contains("E34: insufficient lp shares"));
        }
    }
    
    println!("do_stable_remove_liquidity_by_token scenario : {:?} end!", scenario);
}