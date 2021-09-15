use std::collections::HashMap;
use std::convert::TryFrom;

use near_sdk::json_types::{ValidAccountId, U128};
use near_sdk::serde_json::{Value, from_value};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::AccountId;
use near_sdk_sim::{
    call, deploy, init_simulator, to_yocto, view, ContractAccount, ExecutionResult, UserAccount,
};

use ref_exchange::{ContractContract as Exchange, PoolInfo};
use test_token::ContractContract as TestToken;

near_sdk_sim::lazy_static_include::lazy_static_include_bytes! {
    TEST_TOKEN_WASM_BYTES => "../res/test_token.wasm",
    EXCHANGE_WASM_BYTES => "../res/ref_exchange_release.wasm",
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
        t.mint(to_va(root.account_id.clone()), to_yocto("1000").into())
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

/// get ref-exchange's whitelisted tokens
pub fn get_whitelist(pool: &ContractAccount<Exchange>) -> Vec<String> {
    view!(pool.get_whitelisted_tokens()).unwrap_json::<Vec<String>>()
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

pub fn mft_balance_of(
    pool: &ContractAccount<Exchange>,
    token_or_pool: &str,
    account_id: &AccountId,
) -> u128 {
    view!(pool.mft_balance_of(token_or_pool.to_string(), to_va(account_id.clone())))
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

pub fn swap() -> AccountId {
    "swap".to_string()
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
        pool.extend_whitelisted_tokens(vec![to_va(dai()), to_va(eth()), to_va(usdt())])
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
        pool.add_liquidity(1, vec![U128(to_yocto("10")), U128(to_yocto("10"))], None),
        deposit = to_yocto("0.0007")
    )
    .assert_success();
    (root, owner, pool, token1, token2, token3)
}
