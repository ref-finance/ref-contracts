use std::collections::HashMap;
use std::convert::TryFrom;
use near_sdk::json_types::{ValidAccountId, U128};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{AccountId, Balance};
use near_sdk_sim::{
    call, deploy, init_simulator, to_yocto, view, ContractAccount, UserAccount,
};

use ref_exchange::{ContractContract as Exchange,};
use test_token::ContractContract as TestToken;

near_sdk_sim::lazy_static_include::lazy_static_include_bytes! {
    TEST_TOKEN_WASM_BYTES => "../res/test_token.wasm",
    EXCHANGE_WASM_BYTES => "../res/ref_exchange_local.wasm",
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct StorageBalance {
    pub total: U128,
    pub available: U128,
}

fn prepare_token(
    root: &UserAccount,
    token_id: AccountId,
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
    t
}

fn register_token_user(
    root: &UserAccount,
    token: &ContractAccount<TestToken>,
    accounts_to_register: Vec<AccountId>,
) {
    for account_id in accounts_to_register {
        call!(
            root,
            token.storage_deposit(Some(to_va(account_id)), None),
            deposit = to_yocto("1")
        )
        .assert_success();
    }
}

fn transfer_token(
    token: &ContractAccount<TestToken>,
    from: &UserAccount,
    to: AccountId,
    amount: Balance,
) {
    call!(
        from,
        token.ft_transfer(to_va(to), U128(amount), None),
        deposit = 1
    )
    .assert_success();
}

fn dai() -> AccountId {
    "dai".to_string()
}

fn eth() -> AccountId {
    "eth".to_string()
}

fn usdt() -> AccountId {
    "usdt".to_string()
}

fn usdc() -> AccountId {
    "usdc".to_string()
}

fn swap() -> AccountId {
    "swap".to_string()
}

fn to_va(a: AccountId) -> ValidAccountId {
    ValidAccountId::try_from(a).unwrap()
}

/// return root account, owner of the exchange, exchange itself
fn setup_exchange() -> (
    UserAccount,
    UserAccount,
    ContractAccount<Exchange>,
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
    (root, owner, pool)
}

fn get_storage_balance(ex: &ContractAccount<Exchange>, account_id: &AccountId, used: u128, print: bool) {
    let real = view!(ex.storage_balance_of(to_va(account_id.clone()))).unwrap_json::<Option<StorageBalance>>();
    if print {
        println!("{:?}", real);
    }
    assert!(real.is_some());
    if let Some(real) = real {
        assert_eq!(real.total.0 - real.available.0, used);
    }
}

fn get_deposits(ex: &ContractAccount<Exchange>, account_id: &AccountId) {
    let balances = view!(ex.get_deposits(to_va(account_id.clone()))).unwrap_json::<HashMap<AccountId, U128>>();
    println!("user {} deposits: {:#?}", account_id, balances);
}

#[test]
fn simulate_storage_max_account_id() {
    let (root, _, ex) = setup_exchange();
    // make a 64 bytes long account_id;
    let user1 = root.create_user("u123012345678900123456789001234567890123456789001234567890123456".to_string(), to_yocto("100"));
    assert_eq!(user1.account_id().len(), 64);
    // register user to ex, 97 bytes is the max MIN_STORAGE_DEPOSIT with 64 bytes account_id,
    call!(
        user1,
        ex.storage_deposit(None, None),
        deposit = to_yocto("0.00138")
    )
    .assert_success();
    get_storage_balance(&ex, &user1.account_id(), to_yocto("0.00138"), true);

    // another 64 bytes long user
    let user2 = root.create_user("u223012345678900123456789001234567890123456789001234567890123456".to_string(), to_yocto("100"));
    assert_eq!(user2.account_id().len(), 64);
    // register user to ex, 97 bytes is the max MIN_STORAGE_DEPOSIT with 64 bytes account_id,
    call!(
        user2,
        ex.storage_deposit(None, None),
        deposit = to_yocto("0.00138")
    )
    .assert_success();
    get_storage_balance(&ex, &user2.account_id(), to_yocto("0.00138"), true);
}

#[test]
fn simulate_storage_add_liqudity() {
    let (root, owner, ex) = setup_exchange();
    let token1 = prepare_token(&root, dai());
    let token2 = prepare_token(&root, eth());
    call!(
        owner,
        ex.extend_whitelisted_tokens(vec![to_va(dai()), to_va(eth())])
    )
    .assert_success();

    // user1 is the one to create the pool
    let user1 = root.create_user("user1".to_string(), to_yocto("100"));
    call!(
        user1,
        ex.add_simple_pool(vec![to_va(dai()), to_va(eth())], 25),
        deposit = to_yocto("0.00288")
    )
    .assert_success();
    // 0.002_880_000_000_000_000_000_000
    get_storage_balance(&ex, &user1.account_id(), to_yocto("0.00288"), false);

    // user2 is the one to add liqudity
    let user2 = root.create_user("user2".to_string(), to_yocto("100"));
    register_token_user(&root, &token1, vec![ex.account_id(), user2.account_id()]);
    register_token_user(&root, &token2, vec![ex.account_id(), user2.account_id()]);
    transfer_token(&token1, &root, user2.account_id(), to_yocto("10"));
    transfer_token(&token2, &root, user2.account_id(), to_yocto("10"));
    call!(
        user2,
        ex.storage_deposit(None, None),
        deposit = to_yocto("0.00125")
    )
    .assert_success();
    // get_deposits(&ex, &user2.account_id());
    call!(
        user2,
        token1.ft_transfer_call(to_va(swap()), to_yocto("5").into(), None, "".to_string()),
        deposit = 1
    )
    .assert_success();
    // get_deposits(&ex, &user2.account_id());
    call!(
        user2,
        token2.ft_transfer_call(to_va(swap()), to_yocto("5").into(), None, "".to_string()),
        deposit = 1
    )
    .assert_success();
    // get_deposits(&ex, &user2.account_id());
    get_storage_balance(&ex, &user2.account_id(), to_yocto("0.00125"), false);
    call!(
        user2,
        ex.add_liquidity(0, vec![U128(to_yocto("1")), U128(to_yocto("1"))], None),
        deposit = to_yocto("0.0007")
    )
    .assert_success();
    get_storage_balance(&ex, &user2.account_id(), to_yocto("0.00195"), false);
    call!(
        user2,
        ex.add_liquidity(0, vec![U128(to_yocto("1")), U128(to_yocto("1"))], None),
        deposit = 1
    )
    .assert_success();
    get_storage_balance(&ex, &user2.account_id(), to_yocto("0.00195"), false);

    // user3 is the one to buy liquidity from user2 and then remove liqudity
    let user3 = root.create_user("user3".to_string(), to_yocto("100"));
    call!(
        user3,
        ex.mft_register(":0".to_string(), to_va(user3.account_id())),
        deposit = to_yocto("0.00149")
    )
    .assert_success();
    get_storage_balance(&ex, &user3.account_id(), to_yocto("0.00149"), false);
    // get_deposits(&ex, &user3.account_id());
    call!(
        user2,
        ex.mft_transfer(":0".to_string(), to_va(user3.account_id()), U128(to_yocto("1")), None),
        deposit = 1
    )
    .assert_success();
    get_storage_balance(&ex, &user2.account_id(), to_yocto("0.00195"), false);
    get_storage_balance(&ex, &user3.account_id(), to_yocto("0.00149"), false);
    call!(
        user3,
        ex.remove_liquidity(0, U128(to_yocto("1")), vec![U128(1), U128(1)]),
        deposit = to_yocto("0.00046")
    )
    .assert_success();
    get_deposits(&ex, &user3.account_id());
    get_storage_balance(&ex, &user3.account_id(), to_yocto("0.00195"), false);
}

#[test]
fn simulate_storage_register_token() {
    let (root, _, ex) = setup_exchange();
    // let token1 = prepare_token(&root, dai());
    // let token2 = prepare_token(&root, eth());

    let user1 = root.create_user("user1".to_string(), to_yocto("100"));

    // register user to ex, 97 bytes is the max MIN_STORAGE_DEPOSIT with 64 bytes account_id,
    call!(
        user1,
        ex.storage_deposit(None, None),
        deposit = to_yocto("0.00097")
    )
    .assert_success();
    // 79 bytes for an empty account here, may various due to different accountID. 
    get_storage_balance(&ex, &user1.account_id(), to_yocto("0.00079"), true);

    // user register token to exchange
    call!(
        user1,
        ex.storage_deposit(None, None),
        deposit = to_yocto("0.00005")
    )
    .assert_success();
    call!(
        user1,
        ex.register_tokens(vec![to_va(dai())]),
        deposit = 1
    )
    .assert_success();
    // delta = 23 bytes
    get_storage_balance(&ex, &user1.account_id(), to_yocto("0.00102"), false);

    // user register token to exchange
    call!(
        user1,
        ex.storage_deposit(None, None),
        deposit = to_yocto("0.00023")
    )
    .assert_success();
    call!(
        user1,
        ex.register_tokens(vec![to_va(eth())]),
        deposit = 1
    )
    .assert_success();
    // delta = 23 bytes
    get_storage_balance(&ex, &user1.account_id(), to_yocto("0.00125"), false);

    // user register token to exchange
    call!(
        user1,
        ex.storage_deposit(None, None),
        deposit = to_yocto("0.00024")
    )
    .assert_success();
    call!(
        user1,
        ex.register_tokens(vec![to_va(usdt())]),
        deposit = 1
    )
    .assert_success();
    // delta = 24 bytes, cause usdt is 1 byte long than eth and dai
    get_storage_balance(&ex, &user1.account_id(), to_yocto("0.00149"), false);

    // user register token to exchange
    call!(
        user1,
        ex.storage_deposit(None, None),
        deposit = to_yocto("0.00024")
    )
    .assert_success();
    call!(
        user1,
        ex.register_tokens(vec![to_va(usdc())]),
        deposit = 1
    )
    .assert_success();
    // delta = 24 bytes
    get_storage_balance(&ex, &user1.account_id(), to_yocto("0.00173"), true);
}
