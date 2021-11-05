use near_sdk_sim::{
    call, to_yocto, view, ContractAccount, ExecutionResult, UserAccount,
};
use std::collections::HashMap;
use near_sdk::AccountId;
use near_sdk::json_types::U128;
use ref_exchange::{ContractContract as Exchange, PoolInfo, SwapAction};
use rand::Rng;
use rand_pcg::Pcg32;
use crate::fuzzy::{
    types::*,
    utils::*,
    liquidity_manage::*,
    constants::*
};

pub fn swap_action(pool :&ContractAccount<Exchange>, operator: &Operator, token_in: AccountId, token_out: AccountId, amount_in: u128, simple_pool_id: u64) -> ExecutionResult{
    call!(
        &operator.user,
        pool.swap(
            vec![SwapAction {
                pool_id: simple_pool_id,
                token_in: token_in,
                amount_in: Some(U128(amount_in)),
                token_out: token_out,
                min_amount_out: U128(1)
            }],
            None
        ),
        deposit = 1
    )
}

pub fn do_pool_swap(ctx: &mut OperationContext, rng: &mut Pcg32, root: &UserAccount, operator: &Operator, pool :&ContractAccount<Exchange>, simple_pool_count: u64){
    let simple_pool_id = if simple_pool_count == 0 { 0 } else { rng.gen_range(0..simple_pool_count) };
    let simple_pool_info = view!(pool.get_pool(simple_pool_id)).unwrap_json::<PoolInfo>();

    let tokens = &simple_pool_info.token_account_ids;

    let is_shuffle:i8 = rng.gen();

    let (token_in, token_out) = if is_shuffle % 2 == 1 {
        (tokens.get(0).unwrap(), tokens.get(1).unwrap())
    }else{
        (tokens.get(1).unwrap(), tokens.get(0).unwrap())
    };

    let amount_in = to_yocto(&AMOUNT_IN_LIMIT.to_string());

    loop {

        let simple_pool_info = view!(pool.get_pool(simple_pool_id)).unwrap_json::<PoolInfo>();

        let token_in_pool_amount = get_token_amount_in_pool(&simple_pool_info, token_in);
        let token_out_pool_amount = get_token_amount_in_pool(&simple_pool_info, token_out);

        let (scenario, token1_account, token2_account, token1_deposit, token2_deposit) = 
            current_evn_info(ctx, pool, operator, &tokens);

        let (token_in_amount, _token_out_amount, token_in_deposit, _token_out_deposit) = if is_shuffle % 2 == 1 {
            (token1_account, token2_account, token1_deposit, token2_deposit)
        }else{
            (token2_account, token1_account, token2_deposit, token1_deposit)
        };

        println!("pool_swap scenario : {:?} begin!", scenario);
        
        match scenario {
            Scenario::Normal => {
                if token_in_pool_amount == 0 || token_out_pool_amount == 0 {
                    let out_come = swap_action(pool, operator, token_in.clone(), token_out.clone(), amount_in, simple_pool_id);
                    assert_eq!(get_error_count(&out_come), 1);
                    if amount_in > token_in_deposit { 
                        assert!(get_error_status(&out_come).contains("E22: not enough tokens in deposit"));
                    }else {
                        assert!(get_error_status(&out_come).contains("Smart contract panicked: panicked at 'ERR_INVALID'"));
                    }
                    do_add_liquidity(ctx, rng, root, operator, pool, simple_pool_count, Some(simple_pool_id));
                }
               
                let pool_deposits = view!(pool.get_deposits(to_va(operator.user.account_id.clone()))).unwrap_json::<HashMap<AccountId, U128>>();
                let token_in_deposit_old = pool_deposits.get(token_in).unwrap().0;
                let token_out_deposit_old = pool_deposits.get(token_out).unwrap().0;

                if amount_in > token_in_deposit_old {
                    let out_come = swap_action(pool, operator, token_in.clone(), token_out.clone(), amount_in, simple_pool_id);
                    assert_eq!(get_error_count(&out_come), 1);
                    assert!(get_error_status(&out_come).contains("E22: not enough tokens in deposit"));
                    println!("token_amount: {} need_amount: {}  current_amount: {}", token_in_amount, amount_in, token_in_deposit_old);
                    add_token_deposit(ctx, root, operator, token_in, token_in_amount, amount_in, token_in_deposit_old);
                }
                
                let swap_amount_budget = view!(pool.get_return(simple_pool_id, to_va(token_in.clone()), U128(amount_in), to_va(token_out.clone()))).unwrap_json::<U128>().0;

                let swap_amount_string = swap_action(pool, operator, token_in.clone(), token_out.clone(), amount_in, simple_pool_id).unwrap_json::<String>();
                let swap_amount = swap_amount_string.parse::<u128>().unwrap();
                
                let pool_deposits = view!(pool.get_deposits(to_va(operator.user.account_id.clone()))).unwrap_json::<HashMap<AccountId, U128>>();
                let token_out_deposit = pool_deposits.get(token_out).unwrap().0;
                
                assert_eq!(token_out_deposit, swap_amount + token_out_deposit_old);
                assert_eq!(swap_amount, swap_amount_budget);
                let new_simple_pool_info = view!(pool.get_pool(simple_pool_id)).unwrap_json::<PoolInfo>();
                println!("after pool swap current simple pool info {:?} ", new_simple_pool_info);
                break;
            },
            Scenario::Token1NoAccount => {
                if is_shuffle % 2 == 1 {
                    let account_tokens = view!(pool.get_user_whitelisted_tokens(to_va(operator.user.account_id.clone()))).unwrap_json::<Vec<AccountId>>();
                    let out_come = swap_action(pool, operator, token_in.clone(), token_out.clone(), amount_in, simple_pool_id);
                    if account_tokens.contains(token_in) {
                        assert_eq!(get_error_count(&out_come), 1);
                        assert!(get_error_status(&out_come).contains("Smart contract panicked: panicked at 'ERR_INVALID'"));
                    }else {
                        assert_eq!(get_error_count(&out_come), 1);
                        assert!(get_error_status(&out_come).contains("token not registered"));
                    }
                }
                
                user_init_token_account(ctx, root, operator,  tokens.get(0).unwrap());
            },
            Scenario::Token2NoAccount => {
                let account_tokens = view!(pool.get_user_whitelisted_tokens(to_va(operator.user.account_id.clone()))).unwrap_json::<Vec<AccountId>>();
                let out_come = swap_action(pool, operator, token_in.clone(), token_out.clone(), amount_in, simple_pool_id);
                if account_tokens.contains(token_in) {
                    assert_eq!(get_error_count(&out_come), 1);
                    assert!(get_error_status(&out_come).contains("Smart contract panicked: panicked at 'ERR_INVALID'"));
                }else {
                    assert_eq!(get_error_count(&out_come), 1);
                    assert!(get_error_status(&out_come).contains("token not registered"));
                }

                user_init_token_account(ctx, root, operator,  tokens.get(1).unwrap());
            },
            Scenario::Token1NotRegistered => {
                if token_in_pool_amount == 0 || token_out_pool_amount == 0 {
                    let account_tokens = view!(pool.get_user_whitelisted_tokens(to_va(operator.user.account_id.clone()))).unwrap_json::<Vec<AccountId>>();
                    let out_come = swap_action(pool, operator, token_in.clone(), token_out.clone(), amount_in, simple_pool_id);
                    if account_tokens.contains(token_in) {
                        assert_eq!(get_error_count(&out_come), 1);
                        assert!(get_error_status(&out_come).contains("Smart contract panicked: panicked at 'ERR_INVALID'"));
                    }else {
                        assert_eq!(get_error_count(&out_come), 1);
                        assert!(get_error_status(&out_come).contains("token not registered"));
                    }
                }
                
                user_init_deposit_token(ctx, rng, operator, tokens.get(0).unwrap());
            },
            Scenario::Token2NotRegistered => {
                if token_in_pool_amount == 0 || token_out_pool_amount == 0 {
                    let account_tokens = view!(pool.get_user_whitelisted_tokens(to_va(operator.user.account_id.clone()))).unwrap_json::<Vec<AccountId>>();
                    let out_come = swap_action(pool, operator, token_in.clone(), token_out.clone(), amount_in, simple_pool_id);
                    if account_tokens.contains(token_in) {
                        assert_eq!(get_error_count(&out_come), 1);
                        assert!(get_error_status(&out_come).contains("Smart contract panicked: panicked at 'ERR_INVALID'"));
                    }else {
                        assert_eq!(get_error_count(&out_come), 1);
                        assert!(get_error_status(&out_come).contains("token not registered"));
                    }
                }
                
                user_init_deposit_token(ctx, rng, operator, tokens.get(1).unwrap());
            },
            Scenario::NoStorageDeposit => {
                let out_come = swap_action(pool, operator, token_in.clone(), token_out.clone(), amount_in, simple_pool_id);
                assert_eq!(get_error_count(&out_come), 1);
                assert!(get_error_status(&out_come).contains("E10: account not registered"));
                user_storage_deposit(pool, operator);
            }
        }
        println!("pool_swap scenario : {:?} end!", scenario);
    }
    
}