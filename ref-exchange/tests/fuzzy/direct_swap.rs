#![allow(unused)] 
use near_sdk_sim::{
    call, to_yocto, view, ContractAccount, ExecutionResult, UserAccount,
};
use near_sdk::json_types::U128;
use ref_exchange::{ContractContract as Exchange, PoolInfo};
use rand::Rng;
use rand_pcg::Pcg32;
use crate::fuzzy::{
    types::*,
    utils::*,
    liquidity_manage::*,
    constants::*
};

fn pack_action(
    pool_id: u64,
    token_in: &str,
    token_out: &str,
    amount_in: Option<u128>,
    min_amount_out: u128,
) -> String {
    if let Some(amount_in) = amount_in {
        format!(
            "{{\"pool_id\": {}, \"token_in\": \"{}\", \"amount_in\": \"{}\", \"token_out\": \"{}\", \"min_amount_out\": \"{}\"}}",
            pool_id, token_in, amount_in, token_out, min_amount_out
        )
    } else {
        format!(
            "{{\"pool_id\": {}, \"token_in\": \"{}\", \"token_out\": \"{}\", \"min_amount_out\": \"{}\"}}",
            pool_id, token_in, token_out, min_amount_out
        )
    }
}

fn direct_swap_action(
    ctx: &mut OperationContext,
    user: &UserAccount,
    token: &String,
    actions: Vec<String>,
    transfer_amount: u128
) -> ExecutionResult {
    let token_contract = ctx.token_contract_account.get(token).unwrap();
    let actions_str = actions.join(", ");
    let msg_str = format!("{{\"actions\": [{}]}}", actions_str);
    call!(
        user,
        token_contract.ft_transfer_call(to_va(swap()), transfer_amount.into(), None, msg_str),
        deposit = 1
    )
}

pub fn do_direct_swap(ctx: &mut OperationContext, rng: &mut Pcg32, root: &UserAccount, operator: &Operator, pool :&ContractAccount<Exchange>, simple_pool_count: u64){
    let simple_pool_id = if simple_pool_count == 0 { 0 } else { rng.gen_range(0..simple_pool_count) };
    let simple_pool_info = view!(pool.get_pool(simple_pool_id)).unwrap_json::<PoolInfo>();

    let tokens = &simple_pool_info.token_account_ids;

    let is_shuffle:i8 = rng.gen();

    let (token_in, token_out) = if is_shuffle % 2 == 1 {
        (tokens.get(0).unwrap(), tokens.get(1).unwrap())
    }else{
        (tokens.get(1).unwrap(), tokens.get(0).unwrap())
    };

    let amount_in = to_yocto("10");
    // let transfer_amount = rng.gen_range(1..TRANSFER_AMOUNT_LIMIT);
    let transfer_amount = to_yocto(&TRANSFER_AMOUNT_LIMIT.to_string());

    let min_amount_out = to_yocto("1");

    println!("amount_in: {}, transfer_amount:{}", amount_in, transfer_amount);

    let action = pack_action(
        simple_pool_id,
        token_in,
        token_out,
        Some(amount_in),
        min_amount_out,
    );

    loop {

        let simple_pool_info = view!(pool.get_pool(simple_pool_id)).unwrap_json::<PoolInfo>();

        let token_in_pool_amount = get_token_amount_in_pool(&simple_pool_info, token_in);
        let token_out_pool_amount = get_token_amount_in_pool(&simple_pool_info, token_out);

        let test_token_in_amount = get_test_token_amount(ctx, operator, token_in);
        let test_token_out_amount = get_test_token_amount(ctx, operator, token_out);


        let mut scenario = DSScenario::Normal;
        if test_token_in_amount == 0{
            scenario = DSScenario::TokenInZero;
        }else if test_token_out_amount == 0{
            scenario = DSScenario::TokenOutZero;
        }else if token_in_pool_amount == 0 || token_out_pool_amount == 0 {
            scenario = DSScenario::LiquidityEmpty;
        } 

        println!("direct_swap scenario : {:?} begin!", scenario);
        
        match scenario {
            DSScenario::Normal => {

                let swap_amount_budget = view!(pool.get_return(simple_pool_id, to_va(token_in.clone()), U128(amount_in), to_va(token_out.clone()))).unwrap_json::<U128>().0;
                
                let out_come = direct_swap_action(ctx, &operator.user, token_in, vec![action.clone()], transfer_amount);
                out_come.assert_success();

                let test_token_in_amount_new = get_test_token_amount(ctx, operator, token_in);
                let test_token_out_amount_new = get_test_token_amount(ctx, operator, token_out);

                if swap_amount_budget < min_amount_out {
                    assert_eq!(get_error_count(&out_come), 1);
                    assert!(get_error_status(&out_come).contains("ERR_MIN_AMOUNT"));
                    assert_eq!(test_token_in_amount, test_token_in_amount_new);
                    assert_eq!(test_token_out_amount, test_token_out_amount_new);
                }else{
                    assert_eq!(test_token_in_amount - amount_in, test_token_in_amount_new);
                    assert_eq!(test_token_out_amount + swap_amount_budget, test_token_out_amount_new);
                }
                
                let new_simple_pool_info = view!(pool.get_pool(simple_pool_id)).unwrap_json::<PoolInfo>();
                println!("after pool swap current simple pool info {:?} ", new_simple_pool_info);
                break;
            },
            DSScenario::LiquidityEmpty => {
                let out_come = direct_swap_action(ctx, &operator.user, token_in, vec![action.clone()], transfer_amount);
                if amount_in > transfer_amount {
                    assert_eq!(get_error_count(&out_come), 1);
                    assert!(get_error_status(&out_come).contains("E22: not enough tokens in deposit"));
                } else {
                    assert_eq!(get_error_count(&out_come), 1);
                    assert!(get_error_status(&out_come).contains("Smart contract panicked: panicked at 'ERR_INVALID'"));
                }
                do_add_liquidity(ctx, rng, root, operator, pool, simple_pool_count, Some(simple_pool_id));
            },

            DSScenario::TokenInZero => {
                let out_come = direct_swap_action(ctx, &operator.user, token_in, vec![action.clone()], transfer_amount);
                assert_eq!(get_error_count(&out_come), 1);
                assert!(get_error_status(&out_come).contains("Smart contract panicked: The account"));
                assert!(get_error_status(&out_come).contains("is not registered"));
                user_init_token_account(ctx, root, operator, token_in);
            },
            DSScenario::TokenOutZero => {
                user_init_token_account(ctx, root, operator, token_out);
            },
            
            
        }
        println!("direct_swap scenario : {:?} end!", scenario);
    }

}