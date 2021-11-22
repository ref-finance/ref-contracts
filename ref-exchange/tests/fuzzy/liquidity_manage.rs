use near_sdk_sim::{
    call, to_yocto, view, ContractAccount, ExecutionResult, UserAccount,
};
use test_token::ContractContract as TestToken;
use near_sdk::json_types::U128;
use std::{convert::TryInto};
use ref_exchange::{ContractContract as Exchange, PoolInfo, stable_swap::StableSwapPool, admin_fee::AdminFees};
use rand::Rng;
use rand_pcg::Pcg32;
use crate::fuzzy::{
    types::*,
    utils::*,
    constants::*
};
use std::cmp::min;

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

pub fn calculate_add_liquidity_out(real_pool :&ContractAccount<Exchange>, amounts: Vec<u128>) -> u128 {
    let current_pool_info = view!(real_pool.get_pool(0)).unwrap_json::<PoolInfo>();
    let mut pool =
            StableSwapPool::new(0, STABLE_TOKENS.iter().map(|&v| v.clone().to_string().try_into().unwrap()).collect(), vec![18, 6, 6], 10000, 25);
    pool.amounts = current_pool_info.amounts.iter().map(|&v| v.0).collect();
    pool.token_account_ids = current_pool_info.token_account_ids;
    pool.total_fee = current_pool_info.total_fee;
    pool.shares_total_supply = current_pool_info.shares_total_supply.0;

    let mut amounts = amounts;
    pool.add_liquidity(&"root".to_string().into(), &mut amounts, 1, &AdminFees::new(1600))
}

pub fn do_stable_add_liquidity(token_contracts: &Vec<ContractAccount<TestToken>>, rng: &mut Pcg32, root: &UserAccount, operator: &StableOperator, pool :&ContractAccount<Exchange>) -> u128{
    let add_amounts = vec![rng.gen_range(1..ADD_LIQUIDITY_LIMIT as u128) * ONE_DAI,
            rng.gen_range(1..ADD_LIQUIDITY_LIMIT as u128) * ONE_USDT,
            rng.gen_range(1..ADD_LIQUIDITY_LIMIT as u128) * ONE_USDC];

    println!("do_stable_add_liquidity add_amounts : {:?}", add_amounts);
    for (idx, amount) in add_amounts.clone().into_iter().enumerate() {
        let token_contract = token_contracts.get(idx).unwrap();
        add_and_deposit_token(root, &operator.user, token_contract, pool, amount);
    }

    let cal_share = calculate_add_liquidity_out(pool, add_amounts.clone());

    let old_share = mft_balance_of(pool, ":0", &operator.user.account_id());
    let share = call!(
        operator.user,
        pool.add_stable_liquidity(0, add_amounts.into_iter().map(|x| U128(x)).collect(), U128(1)),
        deposit = to_yocto("0.001")
    ).unwrap_json::<U128>().0;

    assert_eq!(cal_share, share);
    assert_eq!(mft_balance_of(pool, ":0", &operator.user.account_id()), old_share + share);

    share
}

pub fn do_stable_remove_liquidity_by_shares(token_contracts: &Vec<ContractAccount<TestToken>>, rng: &mut Pcg32, root: &UserAccount, operator: &StableOperator, pool :&ContractAccount<Exchange>){
    

    let mut scenario = StableScenario::Normal;

    let min_amounts = vec![U128(1*ONE_DAI), U128(1*ONE_USDT), U128(1*ONE_USDC)];

    let user_lpt = if mft_balance_of(&pool, ":0", &operator.user.account_id()) == 0 {
        do_stable_add_liquidity(token_contracts, rng, root, operator, pool)
    } else {
        mft_balance_of(&pool, ":0", &operator.user.account_id())
    };

    let remove_lp_num = rng.gen_range(1..LP_LIMIT) * ONE_LPT * 10;

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
    

    println!("user has lpt : {}, remove : {}", user_lpt, remove_lp_num);

    let out_come = call!(
        operator.user,
        pool.remove_liquidity(0, U128(remove_lp_num), min_amounts),
        deposit = 1 
    );
    match scenario {
        StableScenario::Normal => {
            
            out_come.assert_success();
        },
        StableScenario::Slippage => {
            assert_eq!(get_error_count(&out_come), 1);
            assert!(get_error_status(&out_come).contains("E68: slippage error"));
        },
        StableScenario::InsufficientLpShares => {
            assert_eq!(get_error_count(&out_come), 1);
            assert!(get_error_status(&out_come).contains("E34: insufficient lp shares"));
        }
        _ => {
            panic!("do_stable_remove_liquidity_by_shares find new StableScenario {:?}", scenario);
        }
    }
    
}


pub fn do_stable_remove_liquidity_by_token(token_contracts: &Vec<ContractAccount<TestToken>>, rng: &mut Pcg32, root: &UserAccount, operator: &StableOperator, pool :&ContractAccount<Exchange>){

    let mut scenario = StableScenario::Normal;

    let user_lpt = mft_balance_of(&pool, ":0", &operator.user.account_id());

    

    let remove_lp_num = rng.gen_range(1..LP_LIMIT) * ONE_LPT;

    let total_supply = mft_total_supply(pool, ":0");
    let mut result = vec![0u128; STABLE_TOKENS.len()];

    let amounts  = view!(pool.get_pool(0)).unwrap_json::<PoolInfo>().amounts;
    let min_amounts = vec![U128(1*ONE_DAI), U128(1*ONE_USDT), U128(1*ONE_USDC)];
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

    println!("user has lpt : {}, remove : {}", user_lpt, remove_lp_num);

    let out_come = call!(
        operator.user,
        pool.remove_liquidity(0, U128(remove_lp_num), min_amounts),
        deposit = 1 
    );

    match scenario {
        StableScenario::Normal => {
            out_come.assert_success();
        },
        StableScenario::Slippage => {
            assert_eq!(get_error_count(&out_come), 1);
            println!("{}", get_error_status(&out_come));
            assert!(get_error_status(&out_come).contains("E68: slippage error"));
        },
        _ => {
            panic!("do_stable_remove_liquidity_by_shares find new StableScenario {:?}", scenario);
        }
    }
    
}