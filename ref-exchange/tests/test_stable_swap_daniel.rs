use std::convert::TryFrom;

use near_sdk::json_types::{ValidAccountId, U128, U64};
use near_sdk::AccountId;
use near_sdk_sim::transaction::ExecutionStatus;
use near_sdk_sim::{
    call, deploy, init_simulator, to_yocto, view, ContractAccount, ExecutionResult, UserAccount,
};

use ref_exchange::{ContractContract as Exchange, SwapAction};
use test_token::ContractContract as TestToken;

pub mod common;

near_sdk_sim::lazy_static_include::lazy_static_include_bytes! {
    TEST_TOKEN_WASM_BYTES => "../res/test_token.wasm",
    EXCHANGE_WASM_BYTES => "../res/ref_exchange_release.wasm",
}

const ONE_DAI: u128 = 1000000000000000000;
const ONE_USDT: u128 = 1000000;
const ONE_USDC: u128 = 1000000;

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

fn assert_failure(outcome: ExecutionResult, error_message: &str) {
    assert!(!outcome.is_ok());
    let exe_status = format!("{:?}", outcome.promise_errors()[0].as_ref().unwrap().status());
    println!("{}", exe_status);
    assert!(exe_status.contains(error_message));
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
        t.mint(root.valid_account_id(), to_yocto("10000000").into())
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

fn dai() -> AccountId {
    "dai".to_string()
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

fn setup_stable_pool_with_liquidity() -> (
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
        init_method: new(owner.valid_account_id(), 1600, 400)
    );
    let token1 = test_token(&root, dai(), vec![swap()]);
    let token2 = test_token(&root, usdt(), vec![swap()]);
    let token3 = test_token(&root, usdc(), vec![swap()]);
    call!(
        owner,
        pool.extend_whitelisted_tokens(
            vec![
                token1.valid_account_id(), 
                token2.valid_account_id(),
                token3.valid_account_id()
            ]
        )
    );
    call!(
        owner,
        pool.add_stable_swap_pool(
            vec![
                token1.valid_account_id(), 
                token2.valid_account_id(),
                token3.valid_account_id()
            ], 
            vec![18, 6, 6],
            25,
            10000
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

    call!(
        root,
        token1.ft_transfer_call(pool.valid_account_id(), U128(100000*ONE_DAI), None, "".to_string()),
        deposit = 1
    )
    .assert_success();
    call!(
        root,
        token2.ft_transfer_call(pool.valid_account_id(), U128(100000*ONE_USDT), None, "".to_string()),
        deposit = 1
    )
    .assert_success();
    call!(
        root,
        token3.ft_transfer_call(pool.valid_account_id(), U128(100000*ONE_USDC), None, "".to_string()),
        deposit = 1
    )
    .assert_success();
    call!(
        root,
        pool.add_stable_liquidity(0, vec![U128(100000*ONE_DAI), U128(100000*ONE_USDT), U128(100000*ONE_USDC)], U128(1)),
        deposit = to_yocto("0.0007")
    )
    .assert_success();
    (root, owner, pool, token1, token2, token3)
}


/// Test for cases that should panic or throw an error
// ERR66 could not be tested with this approach
// ERR67 could not be tested with this approach
// ERR70 could not be tested with this approach
// ERR81 could not be tested with this approach

#[test]
fn test_stable_e61 () {
    let root = init_simulator(None);
    let owner = root.create_user("owner".to_string(), to_yocto("100"));
    let pool = deploy!(
        contract: Exchange,
        contract_id: swap(),
        bytes: &EXCHANGE_WASM_BYTES,
        signer_account: root,
        init_method: new(owner.valid_account_id(), 1600, 400)
    );
    let token1 = test_token(&root, dai(), vec![swap()]);
    let token2 = test_token(&root, usdt(), vec![swap()]);
    let token3 = test_token(&root, usdc(), vec![swap()]);
    call!(
        owner,
        pool.extend_whitelisted_tokens(
            vec![
                token1.valid_account_id(), 
                token2.valid_account_id(),
                token3.valid_account_id()
            ]
        )
    );

    // small amp
    let outcome = call!(
        owner,
        pool.add_stable_swap_pool(
            vec![
                token1.valid_account_id(), 
                token2.valid_account_id(),
                token3.valid_account_id()
            ], 
            vec![18, 6, 6],
            25,
            0
        ),
        deposit = to_yocto("1")
    );
    assert_failure(outcome, "E61: illegal amp");

    // large amp
    let outcome = call!(
        owner,
        pool.add_stable_swap_pool(
            vec![
                token1.valid_account_id(), 
                token2.valid_account_id(),
                token3.valid_account_id()
            ], 
            vec![18, 6, 6],
            25,
            100_000_000
        ),
        deposit = to_yocto("1")
    );
    assert_failure(outcome, "E61: illegal amp");
}

#[test]
fn test_stable_e62() {
    let root = init_simulator(None);
    let owner = root.create_user("owner".to_string(), to_yocto("100"));
    let pool = deploy!(
        contract: Exchange,
        contract_id: swap(),
        bytes: &EXCHANGE_WASM_BYTES,
        signer_account: root,
        init_method: new(owner.valid_account_id(), 1600, 400)
    );
    let token1 = test_token(&root, dai(), vec![swap()]);
    let token2 = test_token(&root, usdt(), vec![swap()]);
    let token3 = test_token(&root, usdc(), vec![swap()]);
    call!(
        owner,
        pool.extend_whitelisted_tokens(
            vec![
                token1.valid_account_id(), 
                token2.valid_account_id(),
                token3.valid_account_id()
            ]
        )
    );

    // invalid fee
    let outcome = call!(
        owner,
        pool.add_stable_swap_pool(
            vec![
                token1.valid_account_id(), 
                token2.valid_account_id(),
                token3.valid_account_id()
            ], 
            vec![18, 6, 6],
            100_000,
            10000
        ),
        deposit = to_yocto("1")
    );
    assert_failure(outcome, "E62: illegal fee");
}

#[test]
fn test_stable_e63() {
    let (root, owner, pool, _, _, _) = setup_stable_pool_with_liquidity();
    let invalid_token_id = "invalid-token".to_string();

    call!(
        owner,
        pool.extend_whitelisted_tokens(
            vec![
                to_va(invalid_token_id.clone())
            ]
        )
    );

    let invalid_token = test_token(&root, invalid_token_id.clone(), vec![swap()]);
    call!(
        root,
        invalid_token.ft_transfer_call(
            pool.valid_account_id(), 
            U128(100), 
            None, 
            "".to_string()
        ),
        deposit = 1
    )
    .assert_success();

    // invalid token id
    let outcome = call!(
        root,
        pool.swap(
            vec![SwapAction {
                pool_id: 0,
                token_in: invalid_token_id.clone(),
                amount_in: Some(U128(100)),
                token_out: usdc(),
                min_amount_out: U128(1)
            }],
            None
        ),
        deposit = 1
    );
    assert_failure(outcome, "E63: missing token");
}

#[test]
fn test_stable_e64() {
    let root = init_simulator(None);
    let owner = root.create_user("owner".to_string(), to_yocto("100"));
    let pool = deploy!(
        contract: Exchange,
        contract_id: swap(),
        bytes: &EXCHANGE_WASM_BYTES,
        signer_account: root,
        init_method: new(owner.valid_account_id(), 1600, 400)
    );
    let token1 = test_token(&root, dai(), vec![swap()]);
    let token2 = test_token(&root, usdt(), vec![swap()]);
    let token3 = test_token(&root, usdc(), vec![swap()]);
    call!(
        owner,
        pool.extend_whitelisted_tokens(
            vec![
                token1.valid_account_id(), 
                token2.valid_account_id(),
                token3.valid_account_id()
            ]
        )
    );
    call!(
        owner,
        pool.add_stable_swap_pool(
            vec![
                token1.valid_account_id(), 
                token2.valid_account_id(),
                token3.valid_account_id()
            ], 
            vec![18, 6, 6],
            25,
            10000
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

    call!(
        root,
        token1.ft_transfer_call(pool.valid_account_id(), U128(100000*ONE_DAI), None, "".to_string()),
        deposit = 1
    )
    .assert_success();
    call!(
        root,
        token2.ft_transfer_call(pool.valid_account_id(), U128(100000*ONE_USDT), None, "".to_string()),
        deposit = 1
    )
    .assert_success();
    call!(
        root,
        token3.ft_transfer_call(pool.valid_account_id(), U128(100000*ONE_USDC), None, "".to_string()),
        deposit = 1
    )
    .assert_success();

    // invalid amount list length
    let mut outcome = call!(
        root,
        pool.add_stable_liquidity(0, vec![U128(100000), U128(100000), U128(100000), U128(100000)], U128(1)),
        deposit = to_yocto("0.0007")
    );
    assert_failure(outcome, "E64: illegal tokens count");

    call!(
        root,
        pool.add_stable_liquidity(0, vec![U128(100000), U128(100000), U128(100000)], U128(1)),
        deposit = to_yocto("0.0007")
    )
    .assert_success();

    outcome = call!(
        root,
        pool.remove_liquidity(0, U128(1), vec![U128(1), U128(1), U128(1), U128(1)]),
        deposit = 1
    );
    assert_failure(outcome, "E64: illegal tokens count");

    outcome = call!(
        root,
        pool.remove_liquidity_by_tokens(0, vec![U128(1), U128(1), U128(1), U128(1)], U128(1)),
        deposit = 1
    );
    assert_failure(outcome, "E64: illegal tokens count");
}

#[test]
fn test_stable_e65() {
    let root = init_simulator(None);
    let owner = root.create_user("owner".to_string(), to_yocto("100"));
    let pool = deploy!(
        contract: Exchange,
        contract_id: swap(),
        bytes: &EXCHANGE_WASM_BYTES,
        signer_account: root,
        init_method: new(owner.valid_account_id(), 1600, 400)
    );
    let token1 = test_token(&root, dai(), vec![swap()]);
    let token2 = test_token(&root, usdt(), vec![swap()]);
    let token3 = test_token(&root, usdc(), vec![swap()]);
    call!(
        owner,
        pool.extend_whitelisted_tokens(
            vec![
                token1.valid_account_id(), 
                token2.valid_account_id(),
                token3.valid_account_id()
            ]
        )
    );
    call!(
        owner,
        pool.add_stable_swap_pool(
            vec![
                token1.valid_account_id(), 
                token2.valid_account_id(),
                token3.valid_account_id()
            ], 
            vec![18, 6, 6],
            25,
            10000
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

    call!(
        root,
        token1.ft_transfer_call(pool.valid_account_id(), U128(100000*ONE_DAI), None, "".to_string()),
        deposit = 1
    )
    .assert_success();
    call!(
        root,
        token2.ft_transfer_call(pool.valid_account_id(), U128(100000*ONE_USDT), None, "".to_string()),
        deposit = 1
    )
    .assert_success();
    call!(
        root,
        token3.ft_transfer_call(pool.valid_account_id(), U128(100000*ONE_USDC), None, "".to_string()),
        deposit = 1
    )
    .assert_success();

    // invalid amount list length
    let outcome = call!(
        root,
        pool.add_stable_liquidity(0, vec![U128(0), U128(100000), U128(100000)], U128(1)),
        deposit = to_yocto("0.0007")
    );
    assert_failure(outcome, "E65: init token balance should be non-zero");
}


#[test]
fn test_stable_e13() {
    let (root, _, pool, _, _, _) = setup_stable_pool_with_liquidity();
    let user = root.create_user("user".to_string(), to_yocto("100"));

    let outcome = call!(
        user,
        pool.remove_liquidity(0, U128(1), vec![U128(1), U128(1), U128(1)]),
        deposit = 1
    );
    assert_failure(outcome, "E13: LP not registered");

    let outcome = call!(
        user,
        pool.remove_liquidity_by_tokens(0, vec![U128(1), U128(1), U128(1)], U128(1)),
        deposit = 1
    );
    assert_failure(outcome, "E13: LP not registered");

    let outcome = call!(
        user,
        pool.mft_transfer(":0".to_string(), root.valid_account_id(), U128(1), None),
        deposit = 1
    );
    assert_failure(outcome, "E13: LP not registered");
}

#[test]
fn test_stable_e34() {
    let (root, owner, pool, _, _, _) = setup_stable_pool_with_liquidity();
    let lp_balance = view!(pool.mft_balance_of(":0".to_string(), root.valid_account_id()))
        .unwrap_json::<U128>();

    let outcome = call!(
        root,
        pool.remove_liquidity(0, U128(lp_balance.0 + 1), vec![U128(1), U128(1), U128(1)]),
        deposit = 1
    );
    assert_failure(outcome, "E34: insufficient lp shares");

    call!(
        owner,
        pool.mft_register(":0".to_string(), owner.valid_account_id()),
        deposit = to_yocto("1")
    )
    .assert_success();

    call!(
        root,
        pool.mft_transfer(":0".to_string(), owner.valid_account_id(), U128(lp_balance.0 - 1), None),
        deposit = 1
    )
    .assert_success();

    let outcome = call!(
        root,
        pool.remove_liquidity_by_tokens(0, vec![U128(100000*ONE_DAI), U128(100000*ONE_USDT), U128(100000*ONE_USDC)], U128(1)),
        deposit = 1
    );
    assert_failure(outcome, "E34: insufficient lp shares");

    let outcome = call!(
        root,
        pool.mft_transfer(":0".to_string(), owner.valid_account_id(), U128(lp_balance.0), None),
        deposit = 1
    );
    assert_failure(outcome, "E34: insufficient lp shares");
}

#[test]
fn test_stable_e68() {
    let (root, _, pool, token_dai, _, _) = setup_stable_pool_with_liquidity();

    let outcome = call!(
        root,
        pool.remove_liquidity(0, U128(1), vec![U128(10000*ONE_DAI), U128(10000*ONE_USDT), U128(10000*ONE_USDC)]),
        deposit = 1
    );
    assert_failure(outcome, "E68: slippage error");

    let outcome = call!(
        root,
        pool.remove_liquidity_by_tokens(0, vec![U128(10000*ONE_DAI), U128(10000*ONE_USDT), U128(10000*ONE_USDC)], U128(1)),
        deposit = 1
    );
    assert_failure(outcome, "E68: slippage error");

    call!(
        root,
        token_dai.ft_transfer_call(pool.valid_account_id(), U128(ONE_DAI), None, "".to_string()),
        deposit = 1
    )
    .assert_success();

    let outcome = call!(
        root,
        pool.swap(
            vec![SwapAction {
                pool_id: 0,
                token_in: dai(),
                amount_in: Some(U128(ONE_DAI)),
                token_out: usdc(),
                min_amount_out: U128(2 * ONE_USDC)
            }],
            None
        ),
        deposit = 1
    );
    assert_failure(outcome, "E68: slippage error");
}

#[test]
fn test_stable_e69() {
    let (root, _, pool, token_dai, _, _) = setup_stable_pool_with_liquidity();
    let lp_balance = view!(pool.mft_balance_of(":0".to_string(), root.valid_account_id()))
        .unwrap_json::<U128>();

    // try to withdraw all from pool
    let outcome = call!(
        root,
        pool.remove_liquidity(0, lp_balance, vec![U128(1), U128(1), U128(1)]),
        deposit = 1
    );
    assert_failure(outcome, "E69: pool reserved token balance less than MIN_RESERVE");

    let outcome = call!(
        root,
        pool.remove_liquidity_by_tokens(0, vec![U128(100000*ONE_DAI), U128(100000*ONE_USDT), U128(100000*ONE_USDC)], lp_balance),
        deposit = 1
    );
    assert_failure(outcome, "E69: pool reserved token balance less than MIN_RESERVE");

    // remove liquidity so that the pool is small enough
    call!(
        root,
        pool.remove_liquidity_by_tokens(0, vec![U128(99999*ONE_DAI), U128(99999*ONE_USDT), U128(99999*ONE_USDC)], lp_balance),
        deposit = 1
    )
    .assert_success();

    call!(
        root,
        token_dai.ft_transfer_call(pool.valid_account_id(), U128(10_000_000*ONE_DAI), None, "".to_string()),
        deposit = 1
    )
    .assert_success();

    let outcome = call!(
        root,
        pool.swap(
            vec![SwapAction {
                pool_id: 0,
                token_in: dai(),
                amount_in: Some(U128(10_000_000*ONE_DAI)),
                token_out: usdc(),
                min_amount_out: U128(1)
            }],
            None
        ),
        deposit = 1
    );
    assert_failure(outcome, "E69: pool reserved token balance less than MIN_RESERVE");
}

#[test]
fn test_stable_e71() {
    let (root, _, pool, token_dai, _, _) = setup_stable_pool_with_liquidity();

    call!(
        root,
        token_dai.ft_transfer_call(pool.valid_account_id(), U128(1*ONE_DAI), None, "".to_string()),
        deposit = 1
    )
    .assert_success();

    let outcome = call!(
        root,
        pool.swap(
            vec![SwapAction {
                pool_id: 0,
                token_in: dai(),
                amount_in: Some(U128(1)),
                token_out: dai(),
                min_amount_out: U128(1)
            }],
            None
        ),
        deposit = 1
    );
    assert_failure(outcome, "E71: illegal swap with duplicated tokens");
}

#[test]
fn test_stable_e14() {
    let (root, _, pool, _, _, _) = setup_stable_pool_with_liquidity();

    let outcome = call!(
        root,
        pool.mft_register(":0".to_string(), root.valid_account_id()),
        deposit = to_yocto("1")
    );
    assert_failure(outcome, "E14: LP already registered");
}

#[test]
fn test_stable_e82() {
    let (_, owner, pool, _, _, _) = setup_stable_pool_with_liquidity();

    let outcome = call!(
        owner,
        pool.stable_swap_ramp_amp(0, 0, U64(0))
    );
    assert_failure(outcome, "E82: insufficient ramp time");
}

#[test]
fn test_stable_e83() {
    let (root, owner, pool, _, _, _) = setup_stable_pool_with_liquidity();

    let runtime = root.borrow_runtime().current_block().block_timestamp;
    println!("{}", runtime);

    let outcome = call!(
        owner,
        pool.stable_swap_ramp_amp(0, 0, U64(86400000000000))
    );
    assert_failure(outcome, "E83: invalid amp factor");

    let outcome = call!(
        owner,
        pool.stable_swap_ramp_amp(0, 1_000_001, U64(86400000000000))
    );
    assert_failure(outcome, "E83: invalid amp factor");
}

#[test]
fn test_stable_e84() {
    let (_, owner, pool, _, _, _) = setup_stable_pool_with_liquidity();

    let outcome = call!(
        owner,
        pool.stable_swap_ramp_amp(0, 1, U64(86400000000000))
    );
    assert_failure(outcome, "E84: amp factor change is too large");
}
