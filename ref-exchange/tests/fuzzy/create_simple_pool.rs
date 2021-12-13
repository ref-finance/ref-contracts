use near_sdk_sim::{
    call, to_yocto, view, ContractAccount, UserAccount,
};
use ref_exchange::{ContractContract as Exchange, PoolInfo};
use rand::Rng;
use rand_pcg::Pcg32;
use crate::fuzzy::types::*;
use crate::fuzzy::utils::*;
use crate::fuzzy::constants::*;

pub fn create_simple_pool(ctx: &mut OperationContext, rng: &mut Pcg32, root: &UserAccount, operator: &Operator, pool :&ContractAccount<Exchange>){
    let (token1, token2) = get_token_pair(rng);

    if !ctx.token_contract_account.contains_key(&token1){
        let token_contract1 = test_token(root, token1.clone(), vec![swap()], vec![&operator.user]);
        ctx.token_contract_account.insert(token1.clone(), token_contract1);
    }
    if !ctx.token_contract_account.contains_key(&token2){
        let token_contract2 = test_token(root, token2.clone(), vec![swap()], vec![&operator.user]);
        ctx.token_contract_account.insert(token2.clone(), token_contract2);
    }

    let fee = rng.gen_range(5..FEE_LIMIT);
    let pool_id = call!(
        &operator.user,
        pool.add_simple_pool(vec![to_va(token1.clone()), to_va(token2.clone())], fee as u32),
        deposit = to_yocto("1")
    )
    .unwrap_json::<u64>();

    println!("user: {} ,pool_id: {}, pool_info: {:?}", operator.user.account_id.clone(), pool_id, view!(pool.get_pool(pool_id)).unwrap_json::<PoolInfo>());
}