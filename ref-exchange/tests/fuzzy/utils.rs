#![allow(unused)] 
use near_sdk_sim::{
    call, deploy, init_simulator, to_yocto, view, ContractAccount, ExecutionResult, UserAccount,
};
use std::collections::HashMap;
use near_sdk::serde_json::{Value, from_value};
use std::convert::TryFrom;
use rand::Rng;
use rand_pcg::Pcg32;
use near_sdk::json_types::{ValidAccountId, U128};
use ref_exchange::{ContractContract as Exchange, PoolInfo};
use test_token::ContractContract as TestToken;
use near_sdk::AccountId;
use crate::fuzzy::types::*;
use crate::fuzzy::constants::*;



near_sdk_sim::lazy_static_include::lazy_static_include_bytes! {
    TEST_TOKEN_WASM_BYTES => "../res/test_token.wasm",
    EXCHANGE_WASM_BYTES => "../res/ref_exchange.wasm",
}

/**
 * Related to common
 */

pub fn get_operator<'a, T>(rng: &mut Pcg32, users: &'a Vec<T>) -> &'a T{
    let user_index = rng.gen_range(0..users.len());
    &users[user_index]
}

/**
 * Related to amm swap
 */

pub fn get_error_count(r: &ExecutionResult) -> u32 {
    r.promise_errors().len() as u32
}

pub fn get_error_status(r: &ExecutionResult) -> String {
    format!("{:?}", r.promise_errors()[0].as_ref().unwrap().status())
}

pub fn get_token_pair(rng: &mut Pcg32) -> (AccountId, AccountId){
    loop {
        let token1_index = rng.gen_range(0..TOKENS.len());
        let token2_index = rng.gen_range(0..TOKENS.len());
        if token1_index == token2_index {
            continue;
        }
        let token1 = TOKENS[token1_index];
        let token2 = TOKENS[token2_index];
        return (token1.to_string(), token2.to_string())
    }
}


pub fn test_token(
    root: &UserAccount,
    token_id: AccountId,
    accounts_to_register: Vec<AccountId>,
    accounts_to_mint: Vec<&UserAccount>
) -> ContractAccount<TestToken> {
    let t = deploy!(
        contract: TestToken,
        contract_id: token_id,
        bytes: &TEST_TOKEN_WASM_BYTES,
        signer_account: root
    );
    call!(root, t.new()).assert_success();
    call!(
        root,
        t.mint(to_va(root.account_id.clone()), to_yocto(&format!("{}", INIT_ACCOUNT_FOR_TOKEN)).into())
    )
    .assert_success();
    
    for user in accounts_to_mint{
        call!(
            root,
            t.mint(to_va(user.account_id.clone()), to_yocto(&format!("{}", INIT_ACCOUNT_FOR_TOKEN)).into())
        )
        .assert_success();
    }

    for account_id in accounts_to_register {
        call!(
            root,
            t.storage_deposit(Some(to_va(account_id)), None),
            deposit = to_yocto("1")
        )
        .assert_success();
    }
    t
}

pub fn to_va(a: AccountId) -> ValidAccountId {
    ValidAccountId::try_from(a).unwrap()
}

pub fn swap() -> AccountId {
    "swap".to_string()
}

pub fn get_token_amount_in_pool(simple_pool_info: &PoolInfo, token_account_id: &AccountId) -> u128 {
    simple_pool_info.amounts[simple_pool_info.token_account_ids.iter().position(|id| id == token_account_id).unwrap()].0
}

pub fn user_storage_deposit(pool :&ContractAccount<Exchange>, operator: &Operator){
    call!(
        &operator.user,
        pool.storage_deposit(None, None),
        deposit = to_yocto("1")
    )
    .assert_success();
}

pub fn user_init_deposit_token(ctx: &mut OperationContext, rng: &mut Pcg32, operator: &Operator, token: &AccountId) {
    let init_token = rng.gen_range(10..INIT_TOKEN_TO_SWAP_POOL_LIMIT);
    let token_contract2 = ctx.token_contract_account.get(token).unwrap();
    call!(
        &operator.user,
        token_contract2.ft_transfer_call(to_va(swap()), to_yocto(&init_token.to_string()).into(), None, "".to_string()),
        deposit = 1
    )
    .assert_success();
}

pub fn user_init_token_account(ctx: &mut OperationContext, root: &UserAccount, operator: &Operator, token: &AccountId){
    let token_contract = ctx.token_contract_account.get(token).unwrap();
    call!(
        root,
        token_contract.mint(to_va(operator.user.account_id.clone()), to_yocto(&format!("{}", INIT_ACCOUNT_FOR_TOKEN)).into())
    )
    .assert_success();
}

pub fn add_token_deposit(ctx: &mut OperationContext, root: &UserAccount, operator: &Operator, token: &AccountId, token_amount: u128, need_value: u128, current_value: u128){
    println!("add_token_deposit");
    let token_contract = ctx.token_contract_account.get(token).unwrap();
    if token_amount < need_value - current_value {
        println!("mint {} {} to {}", INIT_ACCOUNT_FOR_TOKEN, token_contract.account_id(), operator.user.account_id);
        call!(
            root,
            token_contract.ft_transfer_call(to_va(operator.user.account_id.clone()), to_yocto(&format!("{}", INIT_ACCOUNT_FOR_TOKEN)).into(), None,  "".to_string()),
            deposit = 1
        )
        .assert_success();
    }
    println!("deposit {} {} to {}", INIT_ACCOUNT_FOR_TOKEN, token_contract.account_id(), operator.user.account_id);
    call!(
        &operator.user,
        token_contract.ft_transfer_call(to_va(swap()), (need_value - current_value).into(), None, "".to_string()),
        deposit = 1
    )
    .assert_success();
}

pub fn current_evn_info(ctx: &mut OperationContext, pool :&ContractAccount<Exchange>, operator: &Operator, tokens: &Vec<String>) -> (Scenario, u128, u128, u128, u128){
    let storage_state = view!(pool.get_user_storage_state(operator.user.valid_account_id())).unwrap_json_value();
    if let Value::Null = storage_state {
        println!("{} has no user_storage_state", operator.user.account_id);
        return (Scenario::NoStorageDeposit, 0, 0, 0, 0);
    } else {
        let ret: RefStorageState = from_value(storage_state).unwrap();
        println!("{} user_storage_state: {:?}", operator.user.account_id, ret);
    }
    
    let token_contract1 = ctx.token_contract_account.get(tokens.get(0).unwrap()).unwrap();
    let token_contract2 = ctx.token_contract_account.get(tokens.get(1).unwrap()).unwrap();
    let token1_account = view!(token_contract1.ft_balance_of(to_va(operator.user.account_id.clone()))).unwrap_json::<U128>().0;
    println!("{} has {} balance : {}", operator.user.account_id, token_contract1.account_id(), token1_account);
    if token1_account == 0{
        return (Scenario::Token1NoAccount, 0, 0, 0, 0); 
    }
    let token2_account = view!(token_contract2.ft_balance_of(to_va(operator.user.account_id.clone()))).unwrap_json::<U128>().0;
    println!("{} has {} balance : {}", operator.user.account_id, token_contract2.account_id(), token2_account);
    if token2_account == 0{
        return (Scenario::Token2NoAccount, token1_account, 0, 0, 0); 
    }

    let pool_deposits = view!(pool.get_deposits(to_va(operator.user.account_id.clone()))).unwrap_json::<HashMap<AccountId, U128>>();

    let token1_deposit = match pool_deposits.get(&token_contract1.account_id()){
        Some(d) => {
            println!("{} deposits {} : {}", operator.user.account_id, token_contract1.account_id(), d.0);
            d.0
        },
        None => {
            println!("{} has no deposits {} !", operator.user.account_id, token_contract1.account_id());
            return (Scenario::Token1NotRegistered, token1_account, token2_account, 0, 0);
        }
    };
    let token2_deposit = match pool_deposits.get(&token_contract2.account_id()){
        Some(d) => {
            println!("{} deposits {} : {}", operator.user.account_id, token_contract2.account_id(), d.0);
            d.0
        },
        None => {
            println!("{} has no deposits {} !", operator.user.account_id, token_contract2.account_id());
            return (Scenario::Token2NotRegistered, token1_account, token2_account, token1_deposit, 0);
        }
    };
    
    (Scenario::Normal, token1_account, token2_account, token1_deposit, token2_deposit)
}

pub fn get_test_token_amount(ctx: &mut OperationContext, operator: &Operator, token: &String) -> u128 {
    let token_contract = ctx.token_contract_account.get(token).unwrap();
    let token_amount = view!(token_contract.ft_balance_of(to_va(operator.user.account_id.clone()))).unwrap_json::<U128>().0;
    println!("{} has {} balance : {}", operator.user.account_id, token_contract.account_id(), token_amount);
    token_amount
    
}

pub fn init_pool_env() -> (
    UserAccount,
    UserAccount,
    ContractAccount<Exchange>,
    Vec<Operator>
){
    
    let root = init_simulator(None);
    let owner = root.create_user("owner".to_string(), to_yocto("100"));
    
    let pool = deploy!(
        contract: Exchange,
        contract_id: swap(),
        bytes: &EXCHANGE_WASM_BYTES,
        signer_account: root,
        init_method: new(to_va("owner".to_string()), to_va("boost_farm".to_string()), to_va("burrowland".to_string()), 5, 0)
    );

    call!(
        root,
        pool.storage_deposit(None, None),
        deposit = to_yocto("1")
    )
    .assert_success();

    call!(
        owner,
        pool.storage_deposit(None, None),
        deposit = to_yocto("1")
    )
    .assert_success();

    let mut users = Vec::new();
    for user_id in 0..EVERY_PREFERENCE_NUM{
        let user = Operator{
            user: root.create_user(format!("user_create_sample_pool_{}", user_id), to_yocto("100")),
            preference: Preference::CreateSamplePool
        };
        users.push(user);
        let user = Operator{
            user: root.create_user(format!("user_direct_swap_{}", user_id), to_yocto("100")),
            preference: Preference::DirectSwap
        };
        users.push(user);
        let user = Operator{
            user: root.create_user(format!("user_pool_swap_{}", user_id), to_yocto("100")),
            preference: Preference::PoolSwap
        };
        users.push(user);
        let user = Operator{
            user: root.create_user(format!("user_add_liquidity_{}", user_id), to_yocto("100")),
            preference: Preference::AddLiquidity
        };
        users.push(user);
    }

    call!(
        owner,
        pool.extend_whitelisted_tokens(TOKENS.map(|v| to_va(v.to_string())).to_vec()),
        deposit=1
    );
    (root, owner, pool, users)
}

/**
 * Related to stable swap
 */

pub fn setup_stable_pool_with_liquidity_and_operators(
    tokens: Vec<String>,
    amounts: Vec<u128>,
    decimals: Vec<u8>,
    pool_fee: u32,
    amp: u64,
) -> (
    UserAccount,
    UserAccount,
    ContractAccount<Exchange>,
    Vec<ContractAccount<TestToken>>,
    Vec<StableOperator>
) {
    let root = init_simulator(None);
    let owner = root.create_user("owner".to_string(), to_yocto("100"));
    let pool = deploy!(
        contract: Exchange,
        contract_id: swap(),
        bytes: &EXCHANGE_WASM_BYTES,
        signer_account: root,
        init_method: new(owner.valid_account_id(), to_va("boost_farm".to_string()), to_va("burrowland".to_string()), 1600, 0)
    );

    let mut users = Vec::new();
    for user_id in 0..EVERY_PREFERENCE_NUM{
        let user = root.create_user(format!("user_remove_stable_liquidity_by_share_{}", user_id), to_yocto("100"));
        call!(
            user,
            pool.storage_deposit(None, None),
            deposit = to_yocto("1")
        )
        .assert_success();
        users.push(StableOperator{
            user,
            preference: StablePreference::RemoveLiquidityByShare
        });

        let user = root.create_user(format!("user_remove_stable_liquidity_by_token_{}", user_id), to_yocto("100"));
        call!(
            user,
            pool.storage_deposit(None, None),
            deposit = to_yocto("1")
        )
        .assert_success();
        users.push(StableOperator{
            user,
            preference: StablePreference::RemoveLiquidityByToken
        });

        let user = root.create_user(format!("user_pool_stable_swap_{}", user_id), to_yocto("100"));
        call!(
            user,
            pool.storage_deposit(None, None),
            deposit = to_yocto("1")
        )
        .assert_success();
        users.push(StableOperator{
            user,
            preference: StablePreference::PoolSwap
        });
        
        let user = root.create_user(format!("user_add_stable_liquidity_{}", user_id), to_yocto("100"));
        call!(
            user,
            pool.storage_deposit(None, None),
            deposit = to_yocto("1")
        )
        .assert_success();
        users.push(StableOperator{
            user,
            preference: StablePreference::AddLiquidity
        });
    }

    let mut token_contracts: Vec<ContractAccount<TestToken>> = vec![];
    for token_name in &tokens {
        token_contracts.push(test_token(&root, token_name.clone(), vec![swap()], vec![]));
    }

    call!(
        owner,
        pool.extend_whitelisted_tokens(
            (&token_contracts).into_iter().map(|x| x.valid_account_id()).collect()
        ),
        deposit=1
    );
    call!(
        owner,
        pool.add_stable_swap_pool(
            (&token_contracts).into_iter().map(|x| x.valid_account_id()).collect(), 
            decimals,
            pool_fee,
            amp
        ),
        deposit = to_yocto("1"))
    .assert_success();

    call!(
        root,
        pool.storage_deposit(None, None),
        deposit = to_yocto("1")
    )
    .assert_success();

    call!(
        owner,
        pool.storage_deposit(None, None),
        deposit = to_yocto("1")
    )
    .assert_success();

    for (idx, amount) in amounts.clone().into_iter().enumerate() {
        let c = token_contracts.get(idx).unwrap();
        call!(
            root,
            c.ft_transfer_call(
                pool.valid_account_id(), 
                U128(amount), 
                None, 
                "".to_string()
            ),
            deposit = 1
        )
        .assert_success();
    }

    call!(
        root,
        pool.add_stable_liquidity(0, amounts.into_iter().map(|x| U128(x)).collect(), U128(1)),
        deposit = to_yocto("0.0007")
    )
    .assert_success();
    (root, owner, pool, token_contracts, users)
}

pub fn dai() -> AccountId {
    STABLE_TOKENS[0].to_string()
}

pub fn usdt() -> AccountId {
    STABLE_TOKENS[1].to_string()
}

pub fn usdc() -> AccountId {
    STABLE_TOKENS[2].to_string()
}
pub fn add_and_deposit_token(
    root: &UserAccount,
    user: &UserAccount,
    token: &ContractAccount<TestToken>,
    ex: &ContractAccount<Exchange>,
    amount: u128,
) {
    if 0 == view!(token.ft_balance_of(user.valid_account_id())).unwrap_json::<U128>().0{
        call!(
            user,
            token.mint(user.valid_account_id(), U128(10))
        )
        .assert_success();
    }
    
    call!(
        root,
        token.ft_transfer(user.valid_account_id(), U128(amount), None),
        deposit = 1
    )
    .assert_success();

    call!(
        user,
        ex.storage_deposit(None, None),
        deposit = to_yocto("1")
    )
    .assert_success();

    call!(
        user,
        token.ft_transfer_call(
            ex.valid_account_id(), 
            U128(amount), 
            None,
            "".to_string()
        ),
        deposit = 1
    )
    .assert_success();   
}

pub fn mft_balance_of(
    pool: &ContractAccount<Exchange>,
    token_or_pool: &str,
    account_id: &AccountId,
) -> u128 {
    view!(pool.mft_balance_of(token_or_pool.to_string(), to_va(account_id.clone())))
        .unwrap_json::<U128>()
        .0
}

pub fn mft_total_supply(
    pool: &ContractAccount<Exchange>,
    token_or_pool: &str,
) -> u128 {
    view!(pool.mft_total_supply(token_or_pool.to_string()))
        .unwrap_json::<U128>()
        .0
}

pub fn pool_share_price(
    pool: &ContractAccount<Exchange>,
    pool_id: u64,
) -> u128 {
    view!(pool.get_pool_share_price(pool_id))
        .unwrap_json::<U128>()
        .0
}