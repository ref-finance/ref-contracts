use std::collections::HashMap;
use std::convert::TryFrom;

use near_sdk::json_types::{ValidAccountId, U128};
use near_sdk::serde_json::{Value, from_value};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::AccountId;
use near_sdk_sim::{
    call, deploy, init_simulator, to_yocto, view, ContractAccount, ExecutionResult, UserAccount,
};

use ref_exchange::{ContractContract as Exchange, PoolInfo, ContractMetadata};
use test_token::ContractContract as TestToken;
use test_rated_token::ContractContract as TestRatedToken;

near_sdk_sim::lazy_static_include::lazy_static_include_bytes! {
    TEST_TOKEN_WASM_BYTES => "../res/test_token.wasm",
    TEST_RATED_TOKEN_WASM_BYTES => "../res/test_rated_token.wasm",
    EXCHANGE_WASM_BYTES => "../res/ref_exchange.wasm",
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct RefStorageState {
    pub deposit: U128,
    pub usage: U128,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct StorageBalance {
    pub total: U128,
    pub available: U128,
}

// pub fn should_fail(r: ExecutionResult) {
//     println!("{:?}", r.status());
//     match r.status() {
//         ExecutionStatus::Failure(_) => {}
//         _ => panic!("Should fail"),
//     }
// }

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

pub fn test_token(
    root: &UserAccount,
    token_id: AccountId,
    accounts_to_register: Vec<AccountId>,
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
        t.mint(to_va(root.account_id.clone()), to_yocto("1000000000").into())
    )
    .assert_success();
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

pub fn test_rated_token(
    root: &UserAccount,
    token_id: AccountId,
    accounts_to_register: Vec<AccountId>,
) -> ContractAccount<TestRatedToken> {
    let t = deploy!(
        contract: TestRatedToken,
        contract_id: token_id,
        bytes: &TEST_RATED_TOKEN_WASM_BYTES,
        signer_account: root
    );
    call!(root, t.new(token_id.clone(), token_id.clone(), 24, U128(10u128.pow(24)))).assert_success();
    call!(
        root,
        t.storage_deposit(Some(to_va(root.account_id.clone())), None),
        deposit = to_yocto("1")
    )
    .assert_success();
    call!(
        root,
        t.mint(to_va(root.account_id.clone()), to_yocto("1000000000").into())
    )
    .assert_success();
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

//*****************************
// View functions
//*****************************

/// tell a user if he has registered to given ft token
pub fn is_register_to_token(
    token: &ContractAccount<TestToken>, 
    account_id: ValidAccountId
) -> bool {
    let sb = view!(token.storage_balance_of(account_id)).unwrap_json_value();
    if let Value::Null = sb {
        false
    } else {
        true
    }
}

/// get user's ft balance of given token
pub fn balance_of(token: &ContractAccount<TestToken>, account_id: &AccountId) -> u128 {
    view!(token.ft_balance_of(to_va(account_id.clone()))).unwrap_json::<U128>().0
}

/// get ref-exchange's metadata
pub fn get_metadata(pool: &ContractAccount<Exchange>) -> ContractMetadata {
    view!(pool.metadata()).unwrap_json::<ContractMetadata>()
}

/// get ref-exchange's version
pub fn get_version(pool: &ContractAccount<Exchange>) -> String {
    view!(pool.version()).unwrap_json::<String>()
}

/// get ref-exchange's pool count
pub fn get_num_of_pools(pool: &ContractAccount<Exchange>) -> u64 {
    view!(pool.get_number_of_pools()).unwrap_json::<u64>()
}

/// get ref-exchange's all pool info
pub fn get_pools(pool: &ContractAccount<Exchange>) -> Vec<PoolInfo> {
    view!(pool.get_pools(0, 100)).unwrap_json::<Vec<PoolInfo>>()
}

/// get ref-exchange's pool info
pub fn get_pool(pool: &ContractAccount<Exchange>, pool_id: u64) -> PoolInfo {
    view!(pool.get_pool(pool_id))
        .unwrap_json::<PoolInfo>()
}

pub fn get_deposits(
    pool: &ContractAccount<Exchange>, 
    account_id: ValidAccountId
) -> HashMap<String, U128> {
    view!(pool.get_deposits(account_id)).unwrap_json::<HashMap<String, U128>>()
}

pub fn list_referrals(pool: &ContractAccount<Exchange>) -> HashMap<String, u32> {
    view!(pool.list_referrals(None, None)).unwrap_json::<HashMap<String, u32>>()
}

/// get ref-exchange's frozenlist tokens
pub fn get_frozenlist(pool: &ContractAccount<Exchange>) -> Vec<String> {
    view!(pool.get_frozenlist_tokens()).unwrap_json::<Vec<String>>()
}

/// get ref-exchange's whitelisted tokens
pub fn get_whitelist(pool: &ContractAccount<Exchange>) -> Vec<String> {
    view!(pool.get_whitelisted_tokens()).unwrap_json::<Vec<String>>()
}

/// get ref-exchange's user whitelisted tokens
pub fn get_user_tokens(pool: &ContractAccount<Exchange>, account_id: ValidAccountId) -> Vec<String> {
    view!(pool.get_user_whitelisted_tokens(account_id)).unwrap_json::<Vec<String>>()
}

pub fn get_storage_balance(
    pool: &ContractAccount<Exchange>, 
    account_id: ValidAccountId
) -> Option<StorageBalance> {
    let sb = view!(pool.storage_balance_of(account_id)).unwrap_json_value();
    if let Value::Null = sb {
        None
    } else {
        // near_sdk::serde_json::
        let ret: StorageBalance = from_value(sb).unwrap();
        Some(ret)
    }
}

pub fn get_storage_state(
    pool: &ContractAccount<Exchange>, 
    account_id: ValidAccountId
) -> Option<RefStorageState> {
    let sb = view!(pool.get_user_storage_state(account_id)).unwrap_json_value();
    if let Value::Null = sb {
        None
    } else {
        let ret: RefStorageState = from_value(sb).unwrap();
        Some(ret)
    }
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

pub fn mft_has_registered(
    pool: &ContractAccount<Exchange>,
    token_or_pool: &str,
    account_id: ValidAccountId,
) -> bool {
    view!(pool.mft_has_registered(token_or_pool.to_string(), account_id))
        .unwrap_json::<bool>()
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

//************************************

pub fn dai() -> AccountId {
    "dai001".to_string()
}

pub fn eth() -> AccountId {
    "eth002".to_string()
}

pub fn usdt() -> AccountId {
    "usdt".to_string()
}

pub fn usdc() -> AccountId {
    "usdc".to_string()
}

pub fn swap() -> AccountId {
    "swap".to_string()
}

pub fn near() -> AccountId {
    "near".to_string()
}

pub fn stnear() -> AccountId {
    "stnear".to_string()
}

pub fn linear() -> AccountId {
    "linear".to_string()
}

pub fn nearx() -> AccountId {
    "nearx".to_string()
}

pub fn to_va(a: AccountId) -> ValidAccountId {
    ValidAccountId::try_from(a).unwrap()
}

pub fn setup_pool_with_liquidity() -> (
    UserAccount,
    UserAccount,
    ContractAccount<Exchange>,
    ContractAccount<TestToken>,
    ContractAccount<TestToken>,
    ContractAccount<TestToken>,
) {
    let root = init_simulator(None);
    let owner = root.create_user("owner".to_string(), to_yocto("100"));
    let pool = deploy!(
        contract: Exchange,
        contract_id: swap(),
        bytes: &EXCHANGE_WASM_BYTES,
        signer_account: root,
        init_method: new(to_va("owner".to_string()), 4, 1)
    );
    let token1 = test_token(&root, dai(), vec![swap()]);
    let token2 = test_token(&root, eth(), vec![swap()]);
    let token3 = test_token(&root, usdt(), vec![swap()]);
    call!(
        owner,
        pool.extend_whitelisted_tokens(vec![to_va(dai()), to_va(eth()), to_va(usdt())]),
        deposit = 1
    );
    call!(
        root,
        pool.add_simple_pool(vec![to_va(dai()), to_va(eth())], 25),
        deposit = to_yocto("1")
    )
    .assert_success();
    call!(
        root,
        pool.add_simple_pool(vec![to_va(eth()), to_va(usdt())], 25),
        deposit = to_yocto("1")
    )
    .assert_success();
    call!(
        root,
        pool.add_simple_pool(vec![to_va(usdt()), to_va(dai())], 25),
        deposit = to_yocto("1")
    )
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

    call!(
        root,
        token1.ft_transfer_call(to_va(swap()), to_yocto("105").into(), None, "".to_string()),
        deposit = 1
    )
    .assert_success();
    call!(
        root,
        token2.ft_transfer_call(to_va(swap()), to_yocto("110").into(), None, "".to_string()),
        deposit = 1
    )
    .assert_success();
    call!(
        root,
        token3.ft_transfer_call(to_va(swap()), to_yocto("110").into(), None, "".to_string()),
        deposit = 1
    )
    .assert_success();
    call!(
        root,
        pool.add_liquidity(0, vec![U128(to_yocto("10")), U128(to_yocto("20"))], None),
        deposit = to_yocto("0.0007")
    )
    .assert_success();
    call!(
        root,
        pool.add_liquidity(1, vec![U128(to_yocto("20")), U128(to_yocto("10"))], None),
        deposit = to_yocto("0.0007")
    )
    .assert_success();
    call!(
        root,
        pool.add_liquidity(2, vec![U128(to_yocto("10")), U128(to_yocto("10"))], None),
        deposit = to_yocto("0.0007")
    )
    .assert_success();
    (root, owner, pool, token1, token2, token3)
}

pub fn setup_stable_pool_with_liquidity(
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
) {
    let root = init_simulator(None);
    let owner = root.create_user("owner".to_string(), to_yocto("100"));
    let pool = deploy!(
        contract: Exchange,
        contract_id: swap(),
        bytes: &EXCHANGE_WASM_BYTES,
        signer_account: root,
        init_method: new(owner.valid_account_id(), 1600, 400)
    );

    let mut token_contracts: Vec<ContractAccount<TestToken>> = vec![];
    for token_name in &tokens {
        token_contracts.push(test_token(&root, token_name.clone(), vec![swap()]));
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
    (root, owner, pool, token_contracts)
}

pub fn setup_rated_pool_with_liquidity(
    tokens: Vec<String>,
    reated_tokens: Vec<String>,
    amounts: Vec<u128>,
    reated_amounts: Vec<u128>,
    decimals: Vec<u8>,
    pool_fee: u32,
    amp: u64,
) -> (
    UserAccount,
    UserAccount,
    ContractAccount<Exchange>,
    Vec<ContractAccount<TestToken>>,
    Vec<ContractAccount<TestRatedToken>>,
) {
    use near_sdk_sim::runtime::GenesisConfig;
    pub const GENESIS_TIMESTAMP: u64 = 1_600_000_000 * 10u64.pow(9);
    let mut genesis_config = GenesisConfig::default();
    genesis_config.genesis_time = GENESIS_TIMESTAMP;
    genesis_config.block_prod_time = 0;
    let root = init_simulator(Some(genesis_config));
    let owner = root.create_user("owner".to_string(), to_yocto("100"));
    let pool = deploy!(
        contract: Exchange,
        contract_id: swap(),
        bytes: &EXCHANGE_WASM_BYTES,
        signer_account: root,
        init_method: new(owner.valid_account_id(), 1600, 400)
    );

    let mut pool_tokens = vec![];

    let mut token_contracts: Vec<ContractAccount<TestToken>> = vec![];
    for token_name in &tokens {
        pool_tokens.push(to_va(token_name.clone()));
        token_contracts.push(test_token(&root, token_name.clone(), vec![swap()]));
    }

    let mut token_rated_contracts: Vec<ContractAccount<TestRatedToken>> = vec![];
    for token_name in &reated_tokens {
        pool_tokens.push(to_va(token_name.clone()));
        token_rated_contracts.push(test_rated_token(&root, token_name.clone(), vec![swap()]));
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
        pool.extend_whitelisted_tokens(
            (&token_rated_contracts).into_iter().map(|x| x.valid_account_id()).collect()
        ),
        deposit=1
    );

    call!(
        owner,
        pool.add_rated_swap_pool(
            pool_tokens, 
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

    for (idx, amount) in reated_amounts.clone().into_iter().enumerate() {
        let c = token_rated_contracts.get(idx).unwrap();
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

    assert_eq!(100000000, view!(pool.get_pool_share_price(0)).unwrap_json::<U128>().0);
    let all_amounts = amounts.iter().chain(reated_amounts.iter());
    call!(
        root,
        pool.add_stable_liquidity(0, all_amounts.map(|x| U128(*x)).collect(), U128(1)),
        deposit = to_yocto("0.0007")
    )
    .assert_success();
    assert_eq!(100000000, view!(pool.get_pool_share_price(0)).unwrap_json::<U128>().0);
    (root, owner, pool, token_contracts, token_rated_contracts)
}

pub fn setup_rated_pool(
    tokens: Vec<String>,
    reated_tokens: Vec<String>,
    decimals: Vec<u8>,
    pool_fee: u32,
    amp: u64,
) -> (
    UserAccount,
    UserAccount,
    ContractAccount<Exchange>,
    Vec<ContractAccount<TestToken>>,
    Vec<ContractAccount<TestRatedToken>>,
) {
    use near_sdk_sim::runtime::GenesisConfig;
    pub const GENESIS_TIMESTAMP: u64 = 1_600_000_000 * 10u64.pow(9);
    let mut genesis_config = GenesisConfig::default();
    genesis_config.genesis_time = GENESIS_TIMESTAMP;
    genesis_config.block_prod_time = 0;
    let root = init_simulator(Some(genesis_config));
    let owner = root.create_user("owner".to_string(), to_yocto("100"));
    let pool = deploy!(
        contract: Exchange,
        contract_id: swap(),
        bytes: &EXCHANGE_WASM_BYTES,
        signer_account: root,
        init_method: new(owner.valid_account_id(), 1600, 400)
    );

    let mut pool_tokens = vec![];

    let mut token_contracts: Vec<ContractAccount<TestToken>> = vec![];
    for token_name in &tokens {
        pool_tokens.push(to_va(token_name.clone()));
        token_contracts.push(test_token(&root, token_name.clone(), vec![swap()]));
    }

    let mut token_rated_contracts: Vec<ContractAccount<TestRatedToken>> = vec![];
    for token_name in &reated_tokens {
        pool_tokens.push(to_va(token_name.clone()));
        token_rated_contracts.push(test_rated_token(&root, token_name.clone(), vec![swap()]));
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
        pool.extend_whitelisted_tokens(
            (&token_rated_contracts).into_iter().map(|x| x.valid_account_id()).collect()
        ),
        deposit=1
    );

    call!(
        owner,
        pool.add_rated_swap_pool(
            pool_tokens, 
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
    (root, owner, pool, token_contracts, token_rated_contracts)
}

pub fn mint_and_deposit_rated_token(
    user: &UserAccount,
    token: &ContractAccount<TestRatedToken>,
    ex: &ContractAccount<Exchange>,
    amount: u128,
) {
    call!(
        user,
        token.storage_deposit(Some(to_va(user.account_id.clone())), None),
        deposit = to_yocto("1")
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
        token.mint(user.valid_account_id(), U128(amount))
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

pub fn mint_and_deposit_token(
    user: &UserAccount,
    token: &ContractAccount<TestToken>,
    ex: &ContractAccount<Exchange>,
    amount: u128,
) {
    call!(
        user,
        token.mint(user.valid_account_id(), U128(amount))
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

pub fn setup_exchange(root: &UserAccount, exchange_fee: u32, referral_fee: u32) -> (
    UserAccount,
    ContractAccount<Exchange>,
) {
    let owner = root.create_user("owner".to_string(), to_yocto("100"));
    let pool = deploy!(
        contract: Exchange,
        contract_id: swap(),
        bytes: &EXCHANGE_WASM_BYTES,
        signer_account: root,
        init_method: new(to_va("owner".to_string()), exchange_fee, referral_fee)
    );
    (owner, pool)
}

pub fn whitelist_token(
    owner: &UserAccount, 
    ex: &ContractAccount<Exchange>,
    tokens: Vec<ValidAccountId>,
) {
    call!(
        owner,
        ex.extend_whitelisted_tokens(tokens),
        deposit=1
    ).assert_success();
}

pub fn deposit_token(
    user: &UserAccount, 
    ex: &ContractAccount<Exchange>,
    tokens: Vec<&ContractAccount<TestToken>>,
    amounts: Vec<u128>,
) {
    for (idx, token) in tokens.into_iter().enumerate() {
        call!(
            user,
            ex.storage_deposit(None, None),
            deposit = to_yocto("0.1")
        )
        .assert_success();
        call!(
            user,
            token.ft_transfer_call(
                ex.valid_account_id(), 
                U128(amounts[idx]), 
                None, 
                "".to_string()
            ),
            deposit = 1
        )
        .assert_success();
    }
}