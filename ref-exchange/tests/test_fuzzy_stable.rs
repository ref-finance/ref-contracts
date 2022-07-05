use near_sdk_sim::{
    view, ContractAccount, UserAccount,
};

use test_token::ContractContract as TestToken;
use ref_exchange::PoolInfo;
use ref_exchange::{ContractContract as Exchange};

use rand::{Rng, SeedableRng};
use rand_pcg::Pcg32;

mod fuzzy;
use fuzzy::{constants::*,
    liquidity_manage::*,
    pool_swap::*,
    types::*,
    utils::*,
};

fn do_operation(rng: &mut Pcg32, root: &UserAccount, operator: &StableOperator, pool :&ContractAccount<Exchange>, token_contracts: &Vec<ContractAccount<TestToken>>){
    println!("current stable pool info: {:?}", view!(pool.get_pool(0)).unwrap_json::<PoolInfo>());
    match operator.preference{
        StablePreference::RemoveLiquidityByToken => {
            do_stable_remove_liquidity_by_token(token_contracts, rng, root, operator, pool);
        },
        StablePreference::RemoveLiquidityByShare => {
            do_stable_remove_liquidity_by_shares(token_contracts, rng, root, operator, pool);
        },
        StablePreference::PoolSwap => {
            do_stable_pool_swap(token_contracts, rng, root, operator, pool);
        },
        StablePreference::AddLiquidity => {
            do_stable_add_liquidity(token_contracts, rng, root, operator, pool);
        }
    }
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
#[ignore]
fn test_fuzzy_stable() {
    let seeds = generate_fuzzy_seed();

    for seed in seeds {

        println!("*********************************************");
        println!("current seed : {}", seed);
        println!("*********************************************");

        let mut rng = Pcg32::seed_from_u64(seed as u64);
        let (root, _owner, pool, token_contracts, operators) = 
        setup_stable_pool_with_liquidity_and_operators(
            STABLE_TOKENS.iter().map(|&v| v.to_string()).collect(),
            vec![100000*ONE_DAI, 100000*ONE_USDT, 100000*ONE_USDC],
            DECIMALS.to_vec(),
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