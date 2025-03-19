use std::collections::HashMap;
use std::convert::TryFrom;

use near_contract_standards::fungible_token::metadata::FungibleTokenMetadata;
use near_sdk::json_types::{ValidAccountId, U128};
use near_sdk::AccountId;
use near_sdk_sim::transaction::ExecutionStatus;
use near_sdk_sim::{
    call, deploy, init_simulator, to_yocto, view, ContractAccount, ExecutionResult, UserAccount,
};

use ref_exchange::{Action, ContractContract as Exchange, PoolInfo, SwapAction, SwapVolume, SwapVolumeU256View};
use test_token::ContractContract as TestToken;
use mock_wnear::ContractContract as MockWnear;

use crate::common::utils::*;
pub mod common;

near_sdk_sim::lazy_static_include::lazy_static_include_bytes! {
    TEST_TOKEN_WASM_BYTES => "../res/test_token.wasm",
    EXCHANGE_WASM_BYTES => "../res/ref_exchange.wasm",
    MOCK_WNEAR_WASM_BYTES => "../res/mock_wnear.wasm",
}

pub fn should_fail(r: ExecutionResult) {
    println!("{:?}", r.status());
    match r.status() {
        ExecutionStatus::Failure(_) => {}
        _ => panic!("Should fail"),
    }
}

pub fn show_promises(r: ExecutionResult) {
    for promise in r.promise_results() {
        println!("{:?}", promise);
    }
}

fn test_token(
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

fn test_wnear(
    root: &UserAccount,
    accounts_to_register: Vec<AccountId>,
) -> ContractAccount<MockWnear> {
    let t = deploy!(
        contract: MockWnear,
        contract_id: "wnear".to_string(),
        bytes: &MOCK_WNEAR_WASM_BYTES,
        signer_account: root,
        init_method: new()
    );
    call!(
        root,
        t.near_deposit(),
        deposit = to_yocto("1000")
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

fn balance_of(token: &ContractAccount<TestToken>, account_id: &AccountId) -> u128 {
    view!(token.ft_balance_of(to_va(account_id.clone())))
        .unwrap_json::<U128>()
        .0
}

fn wnear_balance_of(token: &ContractAccount<MockWnear>, account_id: &AccountId) -> u128 {
    view!(token.ft_balance_of(to_va(account_id.clone())))
        .unwrap_json::<U128>()
        .0
}

fn mft_balance_of(
    pool: &ContractAccount<Exchange>,
    token_or_pool: &str,
    account_id: &AccountId,
) -> u128 {
    view!(pool.mft_balance_of(token_or_pool.to_string(), to_va(account_id.clone())))
        .unwrap_json::<U128>()
        .0
}

fn dai() -> AccountId {
    "dai".to_string()
}

fn eth() -> AccountId {
    "eth".to_string()
}

fn wnear() -> AccountId {
    "wnear".to_string()
}

fn swap() -> AccountId {
    "swap".to_string()
}

fn to_va(a: AccountId) -> ValidAccountId {
    ValidAccountId::try_from(a).unwrap()
}

fn setup_pool_with_liquidity() -> (
    UserAccount,
    UserAccount,
    ContractAccount<Exchange>,
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
        init_method: new(to_va("owner".to_string()), to_va("boost_farm".to_string()), to_va("burrowland".to_string()), 5, 0)
    );
    let token1 = test_token(&root, dai(), vec![swap()]);
    let token2 = test_token(&root, eth(), vec![swap()]);
    call!(
        owner,
        pool.extend_whitelisted_tokens(vec![to_va(dai()), to_va(eth())]),
        deposit=1
    );
    call!(
        root,
        pool.add_simple_pool(vec![to_va(dai()), to_va(eth())], 25),
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
        pool.add_liquidity(0, vec![U128(to_yocto("5")), U128(to_yocto("10"))], None),
        deposit = to_yocto("0.0007")
    )
    .assert_success();
    (root, owner, pool, token1, token2)
}

#[test]
fn test_swap() {
    let (root, _owner, pool, token1, token2) = setup_pool_with_liquidity();
    assert_eq!(
        view!(pool.get_pool(0)).unwrap_json::<PoolInfo>(),
        PoolInfo {
            pool_kind: "SIMPLE_POOL".to_string(),
            amp: 0,
            token_account_ids: vec![dai(), eth()],
            amounts: vec![to_yocto("5").into(), to_yocto("10").into()],
            total_fee: 25,
            shares_total_supply: to_yocto("1").into(),
        }
    );
    assert_eq!(
        view!(pool.mft_metadata(":0".to_string()))
            .unwrap_json::<FungibleTokenMetadata>()
            .name,
        "ref-pool-0"
    );
    assert_eq!(
        view!(pool.mft_balance_of(":0".to_string(), to_va(root.account_id.clone())))
            .unwrap_json::<U128>()
            .0,
        to_yocto("1")
    );
    let balances = view!(pool.get_deposits(to_va(root.account_id.clone())))
        .unwrap_json::<HashMap<AccountId, U128>>();
    let balances = balances.values().cloned().collect::<Vec<_>>();
    assert_eq!(balances, vec![U128(to_yocto("100")), U128(to_yocto("100"))]);

    call!(
        root,
        pool.swap(
            vec![SwapAction {
                pool_id: 0,
                token_in: dai(),
                amount_in: Some(U128(to_yocto("1"))),
                token_out: eth(),
                min_amount_out: U128(1)
            }],
            None,
            None
        ),
        deposit = 1
    )
    .assert_success();

    let balances = view!(pool.get_deposits(to_va(root.account_id.clone())))
        .unwrap_json::<HashMap<AccountId, U128>>();
    assert_eq!(
        balances.get(&eth()).unwrap(),
        &U128(to_yocto("100") + 1663192997082117548978741)
    );
    assert_eq!(balances.get(&dai()).unwrap(), &U128(to_yocto("99")));

    call!(
        root,
        pool.withdraw(to_va(eth()), U128(to_yocto("101")), None, None),
        deposit = 1
    );
    call!(
        root,
        pool.withdraw(to_va(dai()), U128(to_yocto("99")), None, None),
        deposit = 1
    );

    let balance1 = view!(token1.ft_balance_of(to_va(root.account_id.clone())))
        .unwrap_json::<U128>()
        .0;
    assert_eq!(balance1, to_yocto("994"));
    let balance2 = view!(token2.ft_balance_of(to_va(root.account_id.clone())))
        .unwrap_json::<U128>()
        .0;
    assert_eq!(balance2, to_yocto("991"));
}

#[test]
fn test_withdraw_failure() {
    let root = init_simulator(None);
    let owner = root.create_user("owner".to_string(), to_yocto("100"));
    // Deploy exchange contract and call init method setting owner to "owner"
    let pool = deploy!(
        contract: Exchange,
        contract_id: swap(),
        bytes: &EXCHANGE_WASM_BYTES,
        signer_account: root,
        init_method: new(to_va("owner".to_string()), to_va("boost_farm".to_string()), to_va("burrowland".to_string()), 5, 0)
    );
    // Deploy DAI and wETH fungible tokens
    let dai_contract = test_token(&root, dai(), vec![swap()]);
    let weth_contract = test_token(&root, eth(), vec![swap()]);
    // Add DAI and ETH to token whitelist
    call!(
        owner,
        pool.extend_whitelisted_tokens(vec![to_va(dai()), to_va(eth())]),
        deposit=1
    );
    call!(
        root,
        pool.add_simple_pool(vec![to_va(dai()), to_va(eth())], 25),
        deposit = to_yocto("1")
    )
    .assert_success();

    // Deposit 1 NEAR storage balance for root account
    call!(
        root,
        pool.storage_deposit(None, None),
        deposit = to_yocto("1")
    )
    .assert_success();

    call!(
        root,
        dai_contract.ft_transfer_call(to_va(swap()), to_yocto("105").into(), None, "".to_string()),
        deposit = 1
    )
    .assert_success();
    call!(
        root,
        weth_contract.ft_transfer_call(to_va(swap()), to_yocto("110").into(), None, "".to_string()),
        deposit = 1
    )
    .assert_success();

    // Check exchange balance before user unregisters from fungible token
    let balances_before = view!(pool.get_deposits(to_va(root.account_id.clone())))
        .unwrap_json::<HashMap<AccountId, U128>>();
    assert_eq!(
        balances_before.get(&dai()).unwrap(),
        &to_yocto("105").into()
    );

    // See how much root account has on each fungible token
    let mut dai_amount: U128 =
        view!(dai_contract.ft_balance_of(to_va("root".to_string()))).unwrap_json();
    assert_eq!(dai_amount, to_yocto("895").into());

    // User (perhaps accidentally) unregisters account from fungible token.
    call!(
        root,
        dai_contract.storage_unregister(Some(true)),
        deposit = 1
    )
    .assert_success();

    // Now DAI balance for root is now 0, with no storage either
    dai_amount = view!(dai_contract.ft_balance_of(to_va("root".to_string()))).unwrap_json();
    assert_eq!(dai_amount, U128(0));

    // Root tries to withdraw and the transfer fails
    let withdrawal_result = call!(
        root,
        pool.withdraw(to_va(dai()), to_yocto("30").into(), None, None),
        deposit = 1
    );

    let promise_errors = withdrawal_result.promise_errors();
    assert_eq!(promise_errors.clone().len(), 1, "Expected 1 failed promise when withdrawing to a fungible token to an unregistered account.");
    let promise_failure_opt = promise_errors.get(0).unwrap();

    let promise_failure = promise_failure_opt.as_ref().unwrap();

    if let ExecutionStatus::Failure(err) = promise_failure.status() {
        // At this time, this is the way to check for error messages.
        // This error comes from the fungible token contract.
        assert_eq!(
            err.to_string(),
            "Action #0: Smart contract panicked: The account root is not registered"
        );
    } else {
        panic!("Expected failure when withdrawing to unregistered account.");
    }

    // Check the exchange balances after this failure and ensure it's the same.
    let balances_after = view!(pool.get_deposits(to_va(root.account_id.clone())))
        .unwrap_json::<HashMap<AccountId, U128>>();
    assert_eq!(balances_after.get(&dai()).unwrap(), &to_yocto("105").into());
}

fn direct_swap(user: &UserAccount, contract: &ContractAccount<TestToken>) {
    call!(
        user,
        contract.ft_transfer_call(
            to_va(swap()),
            to_yocto("1").into(),
            None,
            format!("{{\"actions\": [{{\"pool_id\": 0, \"token_in\": \"dai\", \"token_out\": \"eth\", \"min_amount_out\": \"1\"}}]}}")
        ),
        deposit = 1
    ).assert_success();
}

/// Test swap without deposit/withdraw.
#[test]
fn test_direct_swap() {
    let (root, owner, pool, token1, token2) = setup_pool_with_liquidity();
    let new_user = root.create_user("new_user".to_string(), to_yocto("100"));
    call!(
        new_user,
        token1.mint(to_va(new_user.account_id.clone()), U128(to_yocto("10")))
    )
    .assert_success();

    // Test wrong format and that it returns all tokens back.
    call!(
        new_user,
        token1.ft_transfer_call(to_va(swap()), to_yocto("10").into(), None, "{}".to_string()),
        deposit = 1
    )
    .assert_success();
    assert_eq!(balance_of(&token1, &new_user.account_id), to_yocto("10"));
    assert_eq!(balance_of(&token2, &new_user.account_id), to_yocto("0"));

    // Test that token2 account doesn't exist, the balance of token1 is taken, owner received token2.
    direct_swap(&new_user, &token1);
    assert_eq!(balance_of(&token1, &new_user.account_id), to_yocto("9"));
    assert_eq!(balance_of(&token2, &new_user.account_id), to_yocto("0"));
    assert!(mft_balance_of(&pool, &token2.account_id(), &owner.account_id) > to_yocto("1"));

    // Test that token2 account exists, everything works.
    call!(
        new_user,
        token2.storage_deposit(None, None),
        deposit = to_yocto("1")
    )
    .assert_success();
    direct_swap(&new_user, &token1);
    assert_eq!(balance_of(&token1, &new_user.account_id), to_yocto("8"));
    assert!(balance_of(&token2, &new_user.account_id) > to_yocto("1"));

    // Test that account in pool and token2 account exist, everything works.
    call!(
        new_user,
        pool.storage_deposit(None, None),
        deposit = to_yocto("1")
    )
    .assert_success();
    direct_swap(&new_user, &token1);
    assert_eq!(balance_of(&token1, &new_user.account_id), to_yocto("7"));
    assert!(balance_of(&token2, &new_user.account_id) > to_yocto("2"));

    // Test that account in pool exists but token2 account doesn't exist, final balance is in the pool.
    call!(new_user, token2.storage_unregister(Some(true)), deposit = 1).assert_success();
    direct_swap(&new_user, &token1);
    assert_eq!(balance_of(&token1, &new_user.account_id), to_yocto("6"));
    assert_eq!(balance_of(&token2, &new_user.account_id), 0);
    assert!(mft_balance_of(&pool, &token2.account_id(), &new_user.account_id) > to_yocto("0.5"));
}

#[test]
fn test_direct_swap_wnear() {
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
        owner,
        pool.modify_wnear_id(wnear()),
        deposit = 1
    )
    .assert_success();
    let token1 = test_token(&root, dai(), vec![swap()]);
    let token2 = test_wnear(&root, vec![swap()]);
    call!(
        owner,
        pool.extend_whitelisted_tokens(vec![to_va(dai()), to_va(wnear())]),
        deposit=1
    );
    call!(
        root,
        pool.add_simple_pool(vec![to_va(dai()), to_va(wnear())], 25),
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
        pool.add_liquidity(0, vec![U128(to_yocto("5")), U128(to_yocto("10"))], None),
        deposit = to_yocto("0.0007")
    )
    .assert_success();


    let new_user = root.create_user("new_user".to_string(), to_yocto("100"));
    call!(
        new_user,
        token1.mint(to_va(new_user.account_id.clone()), U128(to_yocto("10")))
    )
    .assert_success();

    // skip unwrap near
    call!(
        new_user,
        token2.storage_deposit(None, None),
        deposit = to_yocto("1")
    )
    .assert_success();
    let new_user_wnear_balance = wnear_balance_of(&token2, &new_user.account_id);
    call!(
        new_user,
        token1.ft_transfer_call(
            to_va(swap()),
            to_yocto("1").into(),
            None,
            format!("{{\"actions\": [{{\"pool_id\": 0, \"token_in\": \"dai\", \"token_out\": \"wnear\", \"min_amount_out\": \"1\"}}]}}")
        ),
        deposit = 1
    ).assert_success();
    assert_eq!(balance_of(&token1, &new_user.account_id), to_yocto("9"));
    assert!(wnear_balance_of(&token2, &new_user.account_id) > new_user_wnear_balance + to_yocto("1"));

    let new_user_near_balance = new_user.account().unwrap().amount;
    let new_user_wnear_balance = wnear_balance_of(&token2, &new_user.account_id);
    call!(
        new_user,
        token1.ft_transfer_call(
            to_va(swap()),
            to_yocto("1").into(),
            None,
            format!("{{\"skip_unwrap_near\": false, \"actions\": [{{\"pool_id\": 0, \"token_in\": \"dai\", \"token_out\": \"wnear\", \"min_amount_out\": \"1\"}}]}}")
        ),
        deposit = 1
    ).assert_success();
    assert_eq!(balance_of(&token1, &new_user.account_id), to_yocto("8"));
    assert_eq!(wnear_balance_of(&token2, &new_user.account_id), new_user_wnear_balance);
    assert!(new_user.account().unwrap().amount > new_user_near_balance + to_yocto("1"));
}

#[test]
fn test_direct_swap_wnear_by_output() {
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
        owner,
        pool.modify_wnear_id(wnear()),
        deposit = 1
    )
    .assert_success();
    let token0 = test_token(&root, eth(), vec![swap()]);
    let token1 = test_token(&root, dai(), vec![swap()]);
    let token2 = test_wnear(&root, vec![swap()]);
    call!(
        owner,
        pool.extend_whitelisted_tokens(vec![to_va(eth()), to_va(dai()), to_va(wnear())]),
        deposit=1
    );
    call!(
        root,
        pool.add_simple_pool(vec![to_va(dai()), to_va(wnear())], 25),
        deposit = to_yocto("1")
    )
    .assert_success();

    call!(
        root,
        pool.add_simple_pool(vec![to_va(wnear()), to_va(eth())], 25),
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
        token0.ft_transfer_call(to_va(swap()), to_yocto("105").into(), None, "".to_string()),
        deposit = 1
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
        pool.add_liquidity(0, vec![U128(to_yocto("5")), U128(to_yocto("10"))], None),
        deposit = to_yocto("0.0007")
    )
    .assert_success();
    call!(
        root,
        pool.add_liquidity(1, vec![U128(to_yocto("10")), U128(to_yocto("10"))], None),
        deposit = to_yocto("0.0007")
    )
    .assert_success();


    let new_user = root.create_user("new_user".to_string(), to_yocto("100"));
    call!(
        new_user,
        token1.mint(to_va(new_user.account_id.clone()), U128(to_yocto("100")))
    )
    .assert_success();

    call!(
        new_user,
        token0.mint(to_va(new_user.account_id.clone()), U128(to_yocto("100")))
    )
    .assert_success();

    // skip unwrap near
    call!(
        new_user,
        token0.storage_deposit(None, None),
        deposit = to_yocto("1")
    )
    .assert_success();
    call!(
        new_user,
        token2.storage_deposit(None, None),
        deposit = to_yocto("1")
    )
    .assert_success();
    let new_user_wnear_balance = wnear_balance_of(&token2, &new_user.account_id);
    call!(
        new_user,
        token1.ft_transfer_call(
            to_va(swap()),
            to_yocto("1").into(),
            None,
            format!("{{\"actions\": [{{\"pool_id\": 0, \"token_in\": \"dai\", \"amount_out\": \"{}\", \"token_out\": \"wnear\", \"max_amount_in\": \"{}\"}}]}}", "1663192997082117548978741",  to_yocto("1"))
        ),
        deposit = 1
    ).assert_success();
    assert_eq!(balance_of(&token1, &new_user.account_id), to_yocto("99"));
    assert_eq!(wnear_balance_of(&token2, &new_user.account_id), new_user_wnear_balance + to_yocto("1.663192997082117548978741"));

    let new_user_near_balance = new_user.account().unwrap().amount;
    let new_user_wnear_balance = wnear_balance_of(&token2, &new_user.account_id);
    call!(
        new_user,
        token1.ft_transfer_call(
            to_va(swap()),
            to_yocto("1").into(),
            None,
            format!("{{\"skip_unwrap_near\": false, \"actions\": [{{\"pool_id\": 0, \"token_in\": \"dai\", \"amount_out\": \"{}\", \"token_out\": \"wnear\"}}]}}", "1188419433427736726672912")
        ),
        deposit = 1
    ).assert_success();
    assert_eq!(balance_of(&token1, &new_user.account_id), to_yocto("98"));
    assert_eq!(wnear_balance_of(&token2, &new_user.account_id), new_user_wnear_balance);
    assert!(new_user.account().unwrap().amount > new_user_near_balance + to_yocto("1"));

    let new_user_wnear_balance = wnear_balance_of(&token2, &new_user.account_id);
    let outcome = call!(
        new_user,
        token1.ft_transfer_call(
            to_va(swap()),
            to_yocto("10").into(),
            None,
            format!("{{\"skip_unwrap_near\": false, \"actions\": [
                {{\"pool_id\": 1, \"token_in\": \"wnear\", \"amount_out\": \"{}\", \"token_out\": \"eth\", \"max_amount_in\": \"{}\"}},
                {{\"pool_id\": 0, \"token_in\": \"dai\", \"token_out\": \"wnear\", \"max_amount_in\": \"{}\"}}
                ]}}", "907024323709934075926346",  to_yocto("10"),  to_yocto("10"))
        ),
        deposit = 1
    );
    outcome.assert_success();
    println!("{:#?}", get_logs(&outcome));
    assert_eq!(balance_of(&token0, &new_user.account_id), to_yocto("100") + 907024323709934075926346);
    assert_eq!(balance_of(&token1, &new_user.account_id), to_yocto("98") - 1141363289209670261579317);
    assert_eq!(wnear_balance_of(&token2, &new_user.account_id), new_user_wnear_balance);
    
    println!("{:?}", view!(pool.get_pool(1)).unwrap_json::<PoolInfo>());

    let new_user_wnear_balance = wnear_balance_of(&token2, &new_user.account_id);
    let outcome = call!(
        new_user,
        token0.ft_transfer_call(
            to_va(swap()),
            to_yocto("10").into(),
            None,
            format!("{{\"skip_unwrap_near\": true, \"actions\": [
                {{\"pool_id\": 1, \"token_in\": \"eth\", \"amount_out\": \"{}\", \"token_out\": \"wnear\", \"max_amount_in\": \"{}\"}},
                {{\"pool_id\": 1, \"token_in\": \"eth\", \"amount_out\": \"{}\", \"token_out\": \"wnear\", \"max_amount_in\": \"{}\"}}
                ]}}", "1087411570277351408187381", to_yocto("10"), "1087411570277351408187381", to_yocto("10"))
        ),
        deposit = 1
    );
    outcome.assert_success();
    println!("{:#?}", get_logs(&outcome));
    assert_eq!(balance_of(&token0, &new_user.account_id), to_yocto("100") + 907024323709934075926346 - 2246742758532429976723435);
    assert_eq!(wnear_balance_of(&token2, &new_user.account_id), new_user_wnear_balance + 1087411570277351408187381 * 2);

    let outcome = call!(
        new_user,
        token0.ft_transfer_call(
            to_va(swap()),
            to_yocto("1").into(),
            None,
            format!("{{\"skip_unwrap_near\": true, \"actions\": [
                {{\"pool_id\": 1, \"token_in\": \"eth\", \"amount_out\": \"{}\", \"token_out\": \"wnear\", \"max_amount_in\": \"{}\"}},
                {{\"pool_id\": 1, \"token_in\": \"eth\", \"amount_out\": \"{}\", \"token_out\": \"wnear\", \"max_amount_in\": \"{}\"}}
                ]}}", "1087411570277351408187381", to_yocto("10"), "1087411570277351408187381", to_yocto("10"))
        ),
        deposit = 1
    );
    let exe_status = format!("{:?}", outcome.promise_errors()[0].as_ref().unwrap().status());
    assert!(exe_status.contains("E22: not enough tokens in deposit"));

    let outcome = call!(
        new_user,
        token1.ft_transfer_call(
            to_va(swap()),
            to_yocto("1").into(),
            None,
            format!("{{\"skip_unwrap_near\": false, \"actions\": [{{\"pool_id\": 0, \"token_in\": \"dai\", \"amount_out\": \"{}\", \"token_out\": \"wnear\", \"max_amount_in\": \"{}\"}}]}}", "1188419433427736726672912",  1)
        ),
        deposit = 1
    );
    let exe_status = format!("{:?}", outcome.promise_errors()[0].as_ref().unwrap().status());
    assert!(exe_status.contains("E68: slippage error"));

    let outcome = call!(
        new_user,
        token1.ft_transfer_call(
            to_va(swap()),
            to_yocto("1").into(),
            None,
            format!("{{\"skip_unwrap_near\": false, \"actions\": [
                {{\"pool_id\": 0, \"token_in\": \"dai\", \"amount_out\": \"{}\", \"token_out\": \"wnear\", \"max_amount_in\": \"{}\"}},
                {{\"pool_id\": 0, \"token_in\": \"dai\", \"token_out\": \"wnear\", \"min_amount_out\": \"1\"}}
                ]}}", "1188419433427736726672912",  to_yocto("1"))
        ),
        deposit = 1
    );
    let exe_status = format!("{:?}", outcome.promise_errors()[0].as_ref().unwrap().status());
    assert!(exe_status.contains("E77: all action types must be the same"));
}

#[test]
fn test_execute_actions_in_va() {
    const ONE_USDT: u128 = 10u128.pow(6);

    let root = init_simulator(None);
    let owner = root.create_user("owner".to_string(), to_yocto("100"));
    let pool = deploy!(
        contract: Exchange,
        contract_id: swap(),
        bytes: &EXCHANGE_WASM_BYTES,
        signer_account: root,
        init_method: new(to_va("owner".to_string()), to_va("boost_farm".to_string()), to_va("burrowland".to_string()), 30, 0)
    );
    call!(
        owner,
        pool.modify_wnear_id(wnear()),
        deposit = 1
    )
    .assert_success();
    let token0 = deploy!(
        contract: TestToken,
        contract_id: to_va(eth()),
        bytes: &TEST_TOKEN_WASM_BYTES,
        signer_account: root
    );
    call!(root, token0.new()).assert_success();
    call!(
        root,
        token0.mint(to_va(root.account_id.clone()), to_yocto("10000000").into())
    )
    .assert_success();
    call!(
        root,
        token0.storage_deposit(Some(to_va(swap())), None),
        deposit = to_yocto("1")
    )
    .assert_success();


    let token1 = deploy!(
        contract: TestToken,
        contract_id: usdt(),
        bytes: &TEST_TOKEN_WASM_BYTES,
        signer_account: root
    );
    call!(root, token1.new()).assert_success();
    call!(
        root,
        token1.mint(to_va(root.account_id.clone()), to_yocto("10000000").into())
    )
    .assert_success();
    call!(
        root,
        token1.storage_deposit(Some(to_va(swap())), None),
        deposit = to_yocto("1")
    )
    .assert_success();

    call!(
        owner,
        pool.extend_whitelisted_tokens(vec![to_va(eth()), to_va(usdt())]),
        deposit=1
    );
    
    call!(
        root,
        pool.add_simple_pool(vec![to_va(eth()), to_va(usdt())], 25),
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
        token0.ft_transfer_call(to_va(swap()), (144459999999687970893 * 10).into(), None, "".to_string()),
        deposit = 1
    )
    .assert_success();
    call!(
        root,
        token1.ft_transfer_call(to_va(swap()), (500007198063 * 10).into(), None, "".to_string()),
        deposit = 1
    )
    .assert_success();


    let out_come = call!(
        root,
        pool.add_liquidity(0, vec![144459999999687970893, 500007198063].into_iter().map(|x| U128(x)).collect(), Some(vec![U128(1), U128(1)])),
        deposit = to_yocto("0.0007")
    );
    out_come.assert_success();
    println!("{:#?}", get_logs(&out_come));

    let max_use_tokens = HashMap::from([(usdt(), U128(ONE_USDT * 4000))]);
    let out_come = call!(
        root,
        pool.execute_actions_in_va(
            max_use_tokens,
            vec![Action::Swap(SwapAction {
                pool_id: 0,
                token_in: usdt(),
                amount_in: Some(U128(ONE_USDT * 3000)),
                token_out: eth(),
                min_amount_out: U128(1)
            })],
            None,
            None
        ),
        gas = 300000000000000
    );
    out_come.assert_success();
    println!("{:#?}", get_logs(&out_come));
    println!("{:#?}", out_come.unwrap_json::<HashMap<AccountId, U128>>());
}

#[test]
fn test_simple_swap_volume() {
    let root = init_simulator(None);
    let owner = root.create_user("owner".to_string(), to_yocto("100"));
    let pool = deploy!(
        contract: Exchange,
        contract_id: swap(),
        bytes: &EXCHANGE_WASM_BYTES,
        signer_account: root,
        init_method: new(to_va("owner".to_string()), to_va("boost_farm".to_string()), to_va("burrowland".to_string()), 30, 0)
    );
    call!(
        owner,
        pool.modify_wnear_id(wnear()),
        deposit = 1
    )
    .assert_success();
    let token0 = deploy!(
        contract: TestToken,
        contract_id: to_va(eth()),
        bytes: &TEST_TOKEN_WASM_BYTES,
        signer_account: root
    );
    call!(root, token0.new()).assert_success();
    call!(
        root,
        token0.mint(to_va(root.account_id.clone()), u128::MAX.into())
    )
    .assert_success();
    call!(
        root,
        token0.storage_deposit(Some(to_va(swap())), None),
        deposit = to_yocto("1")
    )
    .assert_success();


    let token1 = deploy!(
        contract: TestToken,
        contract_id: usdt(),
        bytes: &TEST_TOKEN_WASM_BYTES,
        signer_account: root
    );
    call!(root, token1.new()).assert_success();
    call!(
        root,
        token1.mint(to_va(root.account_id.clone()), u128::MAX.into())
    )
    .assert_success();
    call!(
        root,
        token1.storage_deposit(Some(to_va(swap())), None),
        deposit = to_yocto("1")
    )
    .assert_success();

    call!(
        owner,
        pool.extend_whitelisted_tokens(vec![to_va(eth()), to_va(usdt())]),
        deposit=1
    );
    
    call!(
        root,
        pool.add_simple_pool(vec![to_va(eth()), to_va(usdt())], 25),
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
        token0.ft_transfer_call(to_va(swap()), (u128::MAX / 2).into(), None, "".to_string()),
        deposit = 1
    )
    .assert_success();
    call!(
        root,
        token1.ft_transfer_call(to_va(swap()), (u128::MAX / 2).into(), None, "".to_string()),
        deposit = 1
    )
    .assert_success();


    let out_come = call!(
        root,
        pool.add_liquidity(0, vec![u128::MAX / 2, u128::MAX / 2].into_iter().map(|x| U128(x)).collect(), Some(vec![U128(1), U128(1)])),
        deposit = to_yocto("0.0007")
    );
    out_come.assert_success();
    println!("{:#?}", get_logs(&out_come));

    let outcome = call!(
        root,
        token0.ft_transfer_call(
            to_va(swap()),
            (100).into(),
            None,
            format!("{{\"actions\": [
                {{\"pool_id\": 0, \"token_in\": \"eth\", \"amount_in\": \"{}\", \"token_out\": \"usdt\", \"min_amount_out\": \"{}\"}}
                ]}}", 100, 0)
        ),
        deposit = 1
    );
    outcome.assert_success();

    let sv = view!(pool.get_pool_volumes(0)).unwrap_json::<Vec<SwapVolume>>();
    assert_eq!(100, sv[0].input.0);
    assert_eq!(99, sv[0].output.0);
    assert_eq!(0, sv[1].input.0);
    assert_eq!(0, sv[1].output.0);
    let sv_u256 = view!(pool.get_pool_u256_volumes(0)).unwrap_json::<Vec<SwapVolumeU256View>>();
    assert_eq!("100".to_string(), sv_u256[0].input);
    assert_eq!("99".to_string(), sv_u256[0].output);
    assert_eq!("0".to_string(), sv_u256[1].input);
    assert_eq!("0".to_string(), sv_u256[1].output);

    let outcome = call!(
        root,
        token0.ft_transfer_call(
            to_va(swap()),
            (100).into(),
            None,
            format!("{{\"actions\": [
                {{\"pool_id\": 0, \"token_in\": \"eth\", \"amount_in\": \"{}\", \"token_out\": \"usdt\", \"min_amount_out\": \"{}\"}}
                ]}}", 100, 0)
        ),
        deposit = 1
    );
    outcome.assert_success();

    let sv = view!(pool.get_pool_volumes(0)).unwrap_json::<Vec<SwapVolume>>();
    assert_eq!(200, sv[0].input.0);
    assert_eq!(198, sv[0].output.0);
    assert_eq!(0, sv[1].input.0);
    assert_eq!(0, sv[1].output.0);
    let sv_u256 = view!(pool.get_pool_u256_volumes(0)).unwrap_json::<Vec<SwapVolumeU256View>>();
    assert_eq!("200".to_string(), sv_u256[0].input);
    assert_eq!("198".to_string(), sv_u256[0].output);
    assert_eq!("0".to_string(), sv_u256[1].input);
    assert_eq!("0".to_string(), sv_u256[1].output);
}
