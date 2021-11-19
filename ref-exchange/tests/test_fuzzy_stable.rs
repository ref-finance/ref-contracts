use near_sdk_sim::{
    view, ContractAccount, UserAccount,
};

use test_token::ContractContract as TestToken;
use ref_exchange::{PoolInfo, SwapAction};
use ref_exchange::{ContractContract as Exchange};

use rand::{Rng, SeedableRng};
use rand_pcg::Pcg32;

mod fuzzy;
use fuzzy::{constants::*,
    create_simple_pool::*,
    direct_swap::*,
    liquidity_manage::*,
    pool_swap::*,
    types::*,
    utils::*,
    constants::*
};


fn do_operation(rng: &mut Pcg32, root: &UserAccount, operator: &StableOperator, pool :&ContractAccount<Exchange>, token_contracts: &Vec<ContractAccount<TestToken>>){
    println!("current stable pool info: {:?}", view!(pool.get_pool(0)).unwrap_json::<PoolInfo>());
    do_stable_add_liquidity(token_contracts, rng, root, operator, pool, None);
    do_stable_pool_swap(token_contracts, rng, root, operator, pool);
    // match operator.preference{
    //     StablePreference::RemoveLiquidity => {
    //         // do_direct_swap(ctx, rng, root, operator, pool, simple_pool_count);
    //     },
    //     StablePreference::PoolSwap => {
    //         // do_pool_swap(ctx, rng, root, operator, pool, simple_pool_count);
    //     },
    //     StablePreference::AddLiquidity => {
    //         do_stable_add_liquidity(rng, root, operator, pool, simple_pool_count, None);
    //     }
    // }
}

fn generate_fuzzy_seed() -> Vec<u64>{
    let mut seeds:Vec<u64> = Vec::new();

    let mut rng = rand::thread_rng();
    for _ in 0..FUZZY_NUM {
        let seed: u64 = rng.gen();
        seeds.push(seed);
    }
    seeds
}

#[test]
fn test_fuzzy_stable() {

    let seeds = generate_fuzzy_seed();

    for seed in seeds {

        println!("*********************************************");
        println!("current seed : {}", seed);
        println!("*********************************************");

        let mut rng = Pcg32::seed_from_u64(seed as u64);
        let (root, _owner, pool, token_contracts, operators) = 
        setup_stable_pool_with_liquidity_and_operators(
            vec![dai(), usdt(), usdc()],
            vec![100000*ONE_DAI, 100000*ONE_USDT, 100000*ONE_USDC],
            vec![18, 6, 6],
            25,
            10000,
        );

        for i in 0..OPERATION_NUM{
            let operator = get_operator(&mut rng, &operators);
            println!("NO.{} : {:?}", i, operator);
            do_operation(&mut rng, &root, operator, &pool, &token_contracts);
        }
    }
}