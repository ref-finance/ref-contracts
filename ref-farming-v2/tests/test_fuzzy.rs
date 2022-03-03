use near_sdk_sim::{
    view, call, ContractAccount, UserAccount,to_yocto
};
use rand::{Rng, SeedableRng};
use rand_pcg::Pcg32;
use ref_farming_v2::{ContractContract as Farming, FarmInfo};
use ref_exchange::{ContractContract as TestRef};
mod fuzzy;
use fuzzy::{
    utils::*,
    types::*,
    stake::*,
    unstake::*,
    claim::*,
    constant::*
};

pub fn get_operator<'a>(rng: &mut Pcg32, users: &'a Vec<Operator>) -> &'a Operator{
    let user_index = rng.gen_range(0..users.len());
    &users[user_index]
}

pub fn do_operation(ctx: &mut FarmInfo, rng: &mut Pcg32, root: &UserAccount, operator: &Operator, farming :&ContractAccount<Farming>, pool :&ContractAccount<TestRef>){
    println!("seedinfo -- {:?}", view!(farming.get_seed_info(format!("{}@0", pool.account_id()))).unwrap_json::<SeedInfo>());
    println!("farminfo -- {:?}", view!(farming.get_farm(FARM_ID.to_string())).unwrap_json::<FarmInfo>());
    match operator.preference{
        Preference::Stake => {
            do_stake(ctx, rng, root, operator, farming, pool);
        },
        Preference::Unstake => {
            do_unstake(ctx, rng, root, operator, farming, pool);
        },
        Preference::Claim => {
            do_claim(ctx, rng, root, operator, farming, pool);
        },
    }
    println!("----->> move to 60 secs later.");
    assert!(root.borrow_runtime_mut().produce_blocks(60).is_ok());
    println!("<<----- Chain goes 60 blocks, now #{}, ts:{}.", 
    root.borrow_runtime().current_block().block_height, 
    root.borrow_runtime().current_block().block_timestamp);
    
    if view!(farming.get_seed_info(format!("{}@0", pool.account_id()))).unwrap_json::<SeedInfo>().amount.0 == 0{
        ctx.claimed_reward.0 += to_yocto("1");
        ctx.beneficiary_reward.0 += to_yocto("1");
    }else{
        ctx.unclaimed_reward.0 += to_yocto("1");
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
fn test_fuzzy(){

    let seeds = generate_fuzzy_seed();
    for seed in seeds {

        println!("*********************************************");
        println!("current seed : {}", seed);
        println!("*********************************************");

        let (root, owner, farming, pool, users) = prepair_env();

        let mut rng = Pcg32::seed_from_u64(seed as u64);
        let mut ctx = view!(farming.get_farm(FARM_ID.to_string())).unwrap_json::<FarmInfo>().clone();
        for i in 0..OPERATION_NUM{
            let operator = get_operator(&mut rng, &users);
            println!("NO.{} : {:?}", i, operator);
            do_operation(&mut ctx, &mut rng, &root, operator, &farming, &pool);
        }
        let farm_info = show_farminfo(&farming, FARM_ID.to_string(), false);
        assert_farming(&farm_info, "Ended".to_string(), to_yocto(&OPERATION_NUM.to_string()), ctx.cur_round, ctx.last_round, ctx.claimed_reward.0, ctx.unclaimed_reward.0, ctx.beneficiary_reward.0);
        // let out_come = call!(
        //     owner,
        //     farming.force_clean_farm(FARM_ID.to_string()),
        //     deposit = 0
        // );
        // out_come.assert_success();
        // assert_farming(&farm_info, "Ended".to_string(), to_yocto(&OPERATION_NUM.to_string()), ctx.cur_round, ctx.last_round, ctx.claimed_reward.0, ctx.unclaimed_reward.0, ctx.beneficiary_reward.0);
    }
}