use near_sdk_sim::{
    view, ContractAccount, UserAccount,
};

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
    utils::*
};

fn do_operation(ctx: &mut OperationContext, rng: &mut Pcg32, root: &UserAccount, operator: &Operator, pool :&ContractAccount<Exchange>){
    let simple_pool_count = view!(pool.get_number_of_pools()).unwrap_json::<u64>();
    println!("current pool num : {}", simple_pool_count);

    if simple_pool_count == 0 {
        create_simple_pool(ctx, rng, root, operator, pool);
    }
    match operator.preference{
        Preference::CreateSamplePool => {
            const NEED_REPEAT_CREATE: i8 = 1;
            let repeat_create: i8 = rng.gen();
            if simple_pool_count != 0 && NEED_REPEAT_CREATE == repeat_create % 2{
                create_simple_pool(ctx, rng, root, operator, pool);
            }
        }, 
        Preference::DirectSwap => {
            do_direct_swap(ctx, rng, root, operator, pool, simple_pool_count);
        },
        Preference::PoolSwap => {
            do_pool_swap(ctx, rng, root, operator, pool, simple_pool_count);
        },
        Preference::AddLiquidity => {
            do_add_liquidity(ctx, rng, root, operator, pool, simple_pool_count, None);
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
fn test_fuzzy_amm(){

    let seeds = generate_fuzzy_seed();
    for seed in seeds {

        println!("*********************************************");
        println!("current seed : {}", seed);
        println!("*********************************************");

        let mut ctx = OperationContext::default();
        
        let mut rng = Pcg32::seed_from_u64(seed as u64);
        let (root, _owner, pool, users) = init_pool_env();

        for i in 0..OPERATION_NUM{
            let operator = get_operator(&mut rng, &users);
            println!("NO.{} : {:?}", i, operator);
            do_operation(&mut ctx, &mut rng, &root, operator, &pool);
        }
    }
}