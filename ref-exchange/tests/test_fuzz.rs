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

near_sdk_sim::lazy_static_include::lazy_static_include_bytes! {
    TEST_TOKEN_WASM_BYTES => "../res/test_token.wasm",
    EXCHANGE_WASM_BYTES => "../res/ref_exchange_release.wasm",
}


const USER_NUM: i32 = 2;
const SIMPLE_POOL_NUM: u64 = 5;
const INIT_ACCOUNT_FOR_SIMPLE_POOL: u64 = 500;

const INIT_TOKEN_TO_SWAP_LIMIT: u64 = INIT_ACCOUNT_FOR_SIMPLE_POOL / 2;
const FEE_LIMIT: i32 = 30;

const FUZZY_NUM: i32 = 2;
const SWAP_NUM: i32 = 10;
const LIQUIDITY_NUM: i32 = 10;

fn test_token(
    root: &UserAccount,
    token_id: AccountId,
    accounts_to_register: Vec<AccountId>,
    accounts_to_mint: &Vec<UserAccount>
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
        t.mint(to_va(root.account_id.clone()), to_yocto(&format!("{}", SIMPLE_POOL_NUM * INIT_ACCOUNT_FOR_SIMPLE_POOL)).into())
    )
    .assert_success();
    
    for user in accounts_to_mint{
        call!(
            root,
            t.mint(to_va(user.account_id.clone()), to_yocto(&format!("{}", SIMPLE_POOL_NUM * INIT_ACCOUNT_FOR_SIMPLE_POOL)).into())
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

fn dai(pool_id: u64) -> AccountId {
    format!("dai{}", pool_id)
}

fn eth(pool_id: u64) -> AccountId {
    format!("eth{}", pool_id)
}

fn swap() -> AccountId {
    "swap".to_string()
}



fn init_pool_env(rng: &mut Pcg32) -> (
    UserAccount,
    UserAccount,
    ContractAccount<Exchange>,
    Vec<(ContractAccount<TestToken>, ContractAccount<TestToken>)>,
    Vec<UserAccount>
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

    //users
    let mut users = Vec::new();
    for user_id in 0..USER_NUM{
        let user = root.create_user(format!("user{}", user_id), to_yocto("100"));
        users.push(user);
    }

    //tokens
    let mut all_token_accounts = Vec::new();
    let mut all_token_pair = Vec::new();
    for pool_id in 0..SIMPLE_POOL_NUM {
        let token1 = test_token(&root, dai(pool_id), vec![swap()], &users);
        let token2 = test_token(&root, eth(pool_id), vec![swap()], &users);
        all_token_pair.push((token1, token2));
        all_token_accounts.push(to_va(dai(pool_id)));
        all_token_accounts.push(to_va(eth(pool_id)));
        
    }

    call!(
        owner,
        pool.extend_whitelisted_tokens(all_token_accounts)
    );

    //simple_pools
    for pool_id in 0..SIMPLE_POOL_NUM {
        let fee = rng.gen_range(5..FEE_LIMIT);
        call!(
            root,
            pool.add_simple_pool(vec![to_va(dai(pool_id)), to_va(eth(pool_id))], fee as u32),
            deposit = to_yocto("1")
        )
        .assert_success();
    }
    
    for (token_contract1, token_contract2) in &all_token_pair{
        let root_init = rng.gen_range(10..INIT_TOKEN_TO_SWAP_LIMIT);
        call!(
            root,
            token_contract1.ft_transfer_call(to_va(swap()), to_yocto(&root_init.to_string()).into(), None, "".to_string()),
            deposit = 1
        )
        .assert_success();
        call!(
            root,
            token_contract2.ft_transfer_call(to_va(swap()), to_yocto(&root_init.to_string()).into(), None, "".to_string()),
            deposit = 1
        )
        .assert_success();

        for user in &users{
            let user_init = rng.gen_range(10..INIT_TOKEN_TO_SWAP_LIMIT);
            call!(
                user,
                pool.storage_deposit(None, None),
                deposit = to_yocto("1")
            )
            .assert_success();
            call!(
                user,
                token_contract1.ft_transfer_call(to_va(swap()), to_yocto(&user_init.to_string()).into(), None, "".to_string()),
                deposit = 1
            )
            .assert_success();
            call!(
                user,
                token_contract2.ft_transfer_call(to_va(swap()), to_yocto(&user_init.to_string()).into(), None, "".to_string()),
                deposit = 1
            )
            .assert_success();
        }
    }
    (root, owner, pool, all_token_pair, users)
}

#[derive(Debug)]
struct FuzzyResults {
    seed: u64,
    reslut: bool,
}

#[test]
fn test_fuzzy(){
    // let mut fuzzy_results: Vec<SeedResults> = Vec::with_capacity(SEED_COUNT as usize);
    // let mut rng = rand::thread_rng();
    for seed in 0..FUZZY_NUM {
        
        let show_details = true;

        let mut rng = Pcg32::seed_from_u64(seed as u64);
        let (root, owner, pool, token_pair, users) = init_pool_env(&mut rng);

        if show_details{
            println!("current pool info:");
            for simple_pool in 0..SIMPLE_POOL_NUM{
                println!("NO.{} : {:?}", simple_pool, view!(pool.get_pool(simple_pool as u64)).unwrap_json::<PoolInfo>());
                let (token1, token2) = match token_pair.get(simple_pool as usize){
                    Some((token1, token2)) => (token1, token2),
                    None => {
                        println!("error token_pair index");
                        return
                    }
                };
                for user in &users{
                    println!("{}, token1 balance : {}", user.account_id, view!(token1.ft_balance_of(to_va(user.account_id.clone())))
                    .unwrap_json::<U128>()
                    .0);
                    println!("{}, token2 balance : {}", user.account_id, view!(token2.ft_balance_of(to_va(user.account_id.clone())))
                    .unwrap_json::<U128>()
                    .0);
                    println!("{} deposits {} : {}", user.account_id, token1.account_id(), 
                        view!(pool.get_deposits(to_va(user.account_id.clone()))).unwrap_json::<HashMap<AccountId, U128>>().get(&token1.account_id()).unwrap().0);
                    println!("{} deposits {} : {}", user.account_id, token2.account_id(), 
                        view!(pool.get_deposits(to_va(user.account_id.clone()))).unwrap_json::<HashMap<AccountId, U128>>().get(&token2.account_id()).unwrap().0);              
                }
            }
        }

        for _ in 0..LIQUIDITY_NUM{
            let user_index = rng.gen_range(0..USER_NUM);
            let pool_index = rng.gen_range(0..SIMPLE_POOL_NUM);
            let operator = users.get(user_index as usize).unwrap();
            call!(
                operator,
                pool.add_liquidity(pool_index, vec![U128(to_yocto("5")), U128(to_yocto("10"))], None),
                deposit = to_yocto("0.0007")
            )
            .assert_success();
        }

        for _ in 0..SWAP_NUM{
            let user_index = rng.gen_range(0..USER_NUM);
            let pool_index = rng.gen_range(0..SIMPLE_POOL_NUM);
            let operator = users.get(user_index as usize).unwrap();
            call!(
                operator,
                pool.swap(
                    vec![SwapAction {
                        pool_id: pool_index,
                        token_in: dai(pool_index),
                        amount_in: Some(U128(to_yocto("1"))),
                        token_out: eth(pool_index),
                        min_amount_out: U128(1)
                    }],
                    None
                ),
                deposit = 1
            )
            .assert_success();
        }
    }
}