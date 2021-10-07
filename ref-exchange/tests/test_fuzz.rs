use std::collections::HashMap;
use std::convert::TryFrom;

use near_contract_standards::fungible_token::metadata::FungibleTokenMetadata;
use near_sdk::json_types::{ValidAccountId, U128};
use near_sdk::AccountId;
use near_sdk_sim::transaction::ExecutionStatus;
use near_sdk_sim::{
    call, deploy, init_simulator, to_yocto, view, ContractAccount, ExecutionResult, UserAccount,
};

use ref_exchange::{ContractContract as Exchange, PoolInfo, SwapAction};
use test_token::ContractContract as TestToken;

use rand::{Rng, SeedableRng};
use rand_pcg::Pcg32;

use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::serde_json::{Value, from_value};

near_sdk_sim::lazy_static_include::lazy_static_include_bytes! {
    TEST_TOKEN_WASM_BYTES => "../res/test_token.wasm",
    EXCHANGE_WASM_BYTES => "../res/ref_exchange_release.wasm",
}


const TOKENS: [&str; 10] = ["ref", "dai", "usdt", "usdc", "weth", "wnear", "1inch", "grt", "oct", "uni"];

const EVERY_PREFERENCE_NUM: i32 = 1;
const INIT_ACCOUNT_FOR_TOKEN: u64 = 200;

const INIT_TOKEN_TO_SWAP_POOL_LIMIT: u64 = 100;
const ADD_LIQUIDITY_LIMIT: u64 = 20;
const FEE_LIMIT: i32 = 30;

const FUZZY_NUM: usize = 2;
const OPERATION_NUM: i32 = 10;

#[derive(Default)]
struct OperationContext {
    pub token_contract_account: HashMap<AccountId, ContractAccount<TestToken>>
}

#[derive(Debug)]
enum Preference {
    CreateSamplePool, 
    DirectSwap,
    PoolSwap,
    AddLiquidity
}

#[derive(Debug)]
enum Scenario {
    Normal,
    Token1NotRegistered, 
    Token2NotRegistered, 
    Token1NoAccount,
    Token2NoAccount,
    NoStorageDeposit,
}

#[derive(Debug)]
struct Operator {
    pub user: UserAccount,
    pub preference: Preference
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct RefStorageState {
    pub deposit: U128,
    pub usage: U128,
}

pub fn show_promises(r: &ExecutionResult) {
    for promise in r.promise_results() {
        println!("{:?}", promise);
    }
}

pub fn get_logs(r: &ExecutionResult) -> Vec<String> {
    let mut logs: Vec<String> = vec![];
    r.promise_results().iter().map(
        |ex| ex.as_ref().unwrap().logs().iter().map(
            |x| logs.push(x.clone())
        ).for_each(drop)
    ).for_each(drop);
    logs
}

pub fn get_error_count(r: &ExecutionResult) -> u32 {
    r.promise_errors().len() as u32
}

pub fn get_error_status(r: &ExecutionResult) -> String {
    format!("{:?}", r.promise_errors()[0].as_ref().unwrap().status())
}

fn get_token_pair(rng: &mut Pcg32) -> (AccountId, AccountId){
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

fn get_operator<'a>(rng: &mut Pcg32, users: &'a Vec<Operator>) -> &'a Operator{
    let user_index = rng.gen_range(0..users.len());
    &users[user_index]
}

fn test_token(
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

fn to_va(a: AccountId) -> ValidAccountId {
    ValidAccountId::try_from(a).unwrap()
}

fn swap() -> AccountId {
    "swap".to_string()
}



fn init_pool_env(rng: &mut Pcg32) -> (
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
        init_method: new(to_va("owner".to_string()), 4, 1)
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
        pool.extend_whitelisted_tokens(TOKENS.map(|v| to_va(v.to_string())).to_vec())
    );
    (root, owner, pool, users)
}

fn create_simple_pool(ctx: &mut OperationContext, rng: &mut Pcg32, root: &UserAccount, operator: &Operator, pool :&ContractAccount<Exchange>){
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

fn get_token_amount_in_pool(simple_pool_info: &PoolInfo, token_account_id: &AccountId) -> u128 {
    simple_pool_info.amounts[simple_pool_info.token_account_ids.iter().position(|id| id == token_account_id).unwrap()].0
}

fn user_storage_deposit(pool :&ContractAccount<Exchange>, operator: &Operator){
    call!(
        &operator.user,
        pool.storage_deposit(None, None),
        deposit = to_yocto("1")
    )
    .assert_success();
}

fn user_init_deposit_token(ctx: &mut OperationContext, rng: &mut Pcg32, operator: &Operator, token: &AccountId) {
    let init_token = rng.gen_range(10..INIT_TOKEN_TO_SWAP_POOL_LIMIT);
    let token_contract2 = ctx.token_contract_account.get(token).unwrap();
    call!(
        &operator.user,
        token_contract2.ft_transfer_call(to_va(swap()), to_yocto(&init_token.to_string()).into(), None, "".to_string()),
        deposit = 1
    )
    .assert_success();
}

fn user_init_token_account(ctx: &mut OperationContext, root: &UserAccount, operator: &Operator, token: &AccountId){
    let token_contract = ctx.token_contract_account.get(token).unwrap();
    call!(
        root,
        token_contract.mint(to_va(operator.user.account_id.clone()), to_yocto(&format!("{}", INIT_ACCOUNT_FOR_TOKEN)).into())
    )
    .assert_success();
}

fn add_token_deposit(ctx: &mut OperationContext, root: &UserAccount, operator: &Operator, token: &AccountId, token_amount: u128, need_value: u128, current_value: u128){
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

fn current_evn_info(ctx: &mut OperationContext, pool :&ContractAccount<Exchange>, operator: &Operator, tokens: &Vec<String>) -> (Scenario, u128, u128, u128, u128){
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

fn swap_action(pool :&ContractAccount<Exchange>, operator: &Operator, token_in: AccountId, token_out: AccountId, amount_in: u128, simple_pool_id: u64) -> ExecutionResult{
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

fn do_pool_swap(ctx: &mut OperationContext, rng: &mut Pcg32, root: &UserAccount, operator: &Operator, pool :&ContractAccount<Exchange>, simple_pool_count: u64){
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

    loop {

        let simple_pool_info = view!(pool.get_pool(simple_pool_id)).unwrap_json::<PoolInfo>();

        let token_in_pool_amount = get_token_amount_in_pool(&simple_pool_info, token_in);
        let token_out_pool_amount = get_token_amount_in_pool(&simple_pool_info, token_out);

        let (scenario, token1_account, token2_account, token1_deposit, token2_deposit) = 
            current_evn_info(ctx, pool, operator, &tokens);

        let (token_in_amount, token_out_amount, ) = if is_shuffle % 2 == 1 {
            (token1_account, token2_account)
        }else{
            (token2_account, token1_account)
        };

        println!("pool_swap scenario : {:?} begin!", scenario);
        
        match scenario {
            Scenario::Normal => {
                if token_in_pool_amount == 0 || token_out_pool_amount == 0 {
                    let out_come = swap_action(pool, operator, token_in.clone(), token_out.clone(), amount_in, simple_pool_id);
                    assert_eq!(get_error_count(&out_come), 1);
                    assert!(get_error_status(&out_come).contains("Smart contract panicked: panicked at 'ERR_INVALID'"));
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
                let account_tokens = view!(pool.get_user_whitelisted_tokens(to_va(operator.user.account_id.clone()))).unwrap_json::<Vec<AccountId>>();
                let out_come = swap_action(pool, operator, token_in.clone(), token_out.clone(), amount_in, simple_pool_id);
                if account_tokens.contains(token_in) {
                    assert_eq!(get_error_count(&out_come), 1);
                    assert!(get_error_status(&out_come).contains("Smart contract panicked: panicked at 'ERR_INVALID'"));
                }else {
                    assert_eq!(get_error_count(&out_come), 1);
                    assert!(get_error_status(&out_come).contains("token not registered"));
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

fn add_liquidity_action(pool :&ContractAccount<Exchange>, operator: &Operator, simple_pool_id: u64, liquidity1: u128, liquidity2: u128) -> ExecutionResult {
    call!(
        &operator.user,
        pool.add_liquidity(simple_pool_id, vec![U128(liquidity1), U128(liquidity2)], None),
        deposit = to_yocto("0.0009")// < 0.0009 ERR_STORAGE_DEPOSIT
    )
}

fn do_add_liquidity(ctx: &mut OperationContext, rng: &mut Pcg32, root: &UserAccount, operator: &Operator, pool :&ContractAccount<Exchange>, simple_pool_count: u64, specified: Option<u64>){
    let simple_pool_id = match specified{
        Some(id) => id,
        None => {
            if simple_pool_count == 0 { 0 } else { rng.gen_range(0..simple_pool_count) }
        }
    };
    
    let simple_pool_info = view!(pool.get_pool(simple_pool_id)).unwrap_json::<PoolInfo>();

    let tokens = simple_pool_info.token_account_ids;

    let (liquidity1, liquidity2) = (to_yocto("20"), to_yocto("20"));

    loop{
        let (scenario, token1_account, token2_account, token1_deposit, token2_deposit) = 
            current_evn_info(ctx, pool, operator, &tokens);
        println!("add_liquidity scenario : {:?} begin!", scenario);
        
        match scenario {
            Scenario::Normal => {
                if token1_deposit <  liquidity1{
                    let out_come = add_liquidity_action(pool, operator, simple_pool_id, liquidity1, liquidity2);
                    assert_eq!(get_error_count(&out_come), 1);
                    assert!(get_error_status(&out_come).contains("E22: not enough tokens in deposit"));
                    add_token_deposit(ctx, root, operator, tokens.get(0).unwrap(), token1_account, liquidity1, token1_deposit);
                }

                if token2_deposit < liquidity2 {
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
                let token1_deposit = view!(pool.get_deposit(to_va(operator.user.account_id.clone()), to_va(tokens.get(0).unwrap().clone()))).unwrap_json::<U128>().0;
                let out_come = add_liquidity_action(pool, operator, simple_pool_id, liquidity1, liquidity2);
                
                if token1_deposit != 0 && token1_deposit < liquidity1 {
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
                let token1_deposit = view!(pool.get_deposit(to_va(operator.user.account_id.clone()), to_va(tokens.get(0).unwrap().clone()))).unwrap_json::<U128>().0;
                let out_come = add_liquidity_action(pool, operator, simple_pool_id, liquidity1, liquidity2);

                if token1_deposit != 0 && token1_deposit < liquidity1 {
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

        },
        Preference::PoolSwap => {
            do_pool_swap(ctx, rng, root, operator, pool, simple_pool_count);
        },
        Preference::AddLiquidity => {
            do_add_liquidity(ctx, rng, root, operator, pool, simple_pool_count, None);
        }
    }
}

#[derive(Debug)]
struct FuzzyResults {
    seed: u64,
    reslut: bool,
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

        let mut ctx = OperationContext::default();
        
        let mut rng = Pcg32::seed_from_u64(seed as u64);
        let (root, owner, pool, users) = init_pool_env(&mut rng);

        for i in 0..OPERATION_NUM{
            let operator = get_operator(&mut rng, &users);
            println!("NO.{} : {:?}", i, operator);
            do_operation(&mut ctx, &mut rng, &root, operator, &pool);
        }
    }
}