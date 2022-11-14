/// Test for cases that should panic or throw an error
// ERR66 could not be tested with this approach
// ERR67 could not be tested with this approach
// ERR70 could not be tested with this approach
// ERR81 could not be tested with this approach

use near_sdk::json_types::{U128, U64};
use near_sdk_sim::{init_simulator, call, view, to_yocto, ExecutionResult, runtime};

use ref_exchange::SwapAction;
use crate::common::utils::*;
pub mod common;

const ONE_LPT: u128 = 1000000000000000000;
const ONE_DAI: u128 = 1000000000000000000;
const ONE_USDT: u128 = 1000000;
const ONE_USDC: u128 = 1000000;

fn assert_failure(outcome: ExecutionResult, error_message: &str) {
    assert!(!outcome.is_ok());
    let exe_status = format!("{:?}", outcome.promise_errors()[0].as_ref().unwrap().status());
    println!("{}", exe_status);
    assert!(exe_status.contains(error_message));
}

#[test]
fn sim_stable_e100 () {
    let root = init_simulator(None);
    let (_, ex) = setup_exchange(&root, 2000);
    let token1 = test_token(&root, dai(), vec![swap()]);
    let token2 = test_token(&root, usdt(), vec![swap()]);
    let outcome = call!(
        root,
        ex.add_stable_swap_pool(
            vec![token1.valid_account_id(), token2.valid_account_id()], 
            vec![18, 6],
            25,
            1000
        ),
        deposit = to_yocto("1")
    );
    assert_failure(outcome, "E100: no permission to invoke this");
}

#[test]
fn sim_stable_e61 () {
    let root = init_simulator(None);
    let (owner, ex) = setup_exchange(&root, 2000);
    let token1 = test_token(&root, dai(), vec![swap()]);
    let token2 = test_token(&root, usdt(), vec![swap()]);
    call!(
        owner,
        ex.extend_whitelisted_tokens(
            vec![token1.valid_account_id(), token2.valid_account_id()]
        )
    );

    // small amp
    let outcome = call!(
        owner,
        ex.add_stable_swap_pool(
            vec![token1.valid_account_id(), token2.valid_account_id()], 
            vec![18, 6],
            25,
            0
        ),
        deposit = to_yocto("1")
    );
    assert_failure(outcome, "E61: illegal amp");

    // large amp
    let outcome = call!(
        owner,
        ex.add_stable_swap_pool(
            vec![token1.valid_account_id(), token2.valid_account_id()], 
            vec![18, 6],
            25,
            100_000_000
        ),
        deposit = to_yocto("1")
    );
    assert_failure(outcome, "E61: illegal amp");
}

#[test]
fn sim_stable_e62 () {
    let root = init_simulator(None);
    let (owner, ex) = setup_exchange(&root, 2000);
    let token1 = test_token(&root, dai(), vec![swap()]);
    let token2 = test_token(&root, usdt(), vec![swap()]);
    call!(
        owner,
        ex.extend_whitelisted_tokens(
            vec![token1.valid_account_id(), token2.valid_account_id()]
        )
    );

    // invalid fee
    let outcome = call!(
        owner,
        ex.add_stable_swap_pool(
            vec![token1.valid_account_id(), token2.valid_account_id()], 
            vec![18, 6],
            100_000,
            10000
        ),
        deposit = to_yocto("1")
    );
    assert_failure(outcome, "E62: illegal fee");
}

#[test]
fn sim_stable_e63 () {
    let root = init_simulator(None);
    let (owner, ex) = setup_exchange(&root, 2000);
    let token1 = test_token(&root, dai(), vec![ex.account_id()]);
    let token2 = test_token(&root, usdt(), vec![ex.account_id()]);
    whitelist_token(&owner, &ex, vec![token1.valid_account_id(), token2.valid_account_id()]);
    deposit_token(&root, &ex, vec![&token1, &token2], vec![1*ONE_DAI, 1*ONE_USDT]);

    call!(
        owner,
        ex.add_stable_swap_pool(
            vec![token1.valid_account_id(), token2.valid_account_id()], 
            vec![18, 6],
            25,
            10000
        ),
        deposit = to_yocto("1")
    ).assert_success();

    call!(
        root,
        ex.add_stable_liquidity(0, vec![U128(1*ONE_DAI), U128(1*ONE_USDT)], U128(1)),
        deposit = to_yocto("0.01")
    )
    .assert_success();

    let token3 = test_token(&root, usdc(), vec![ex.account_id()]);
    whitelist_token(&owner, &ex, vec![token3.valid_account_id()]);
    deposit_token(&root, &ex, vec![&token3], vec![1*ONE_USDC]);

    let outcome = call!(
        root,
        ex.swap(
            vec![SwapAction {
                pool_id: 0,
                token_in: token3.account_id(),
                amount_in: Some(U128(100)),
                token_out: usdt(),
                min_amount_out: U128(1)
            }],
            None
        ),
        deposit = 1
    );
    assert_failure(outcome, "E63: missing token");
}

#[test]
fn sim_stable_e64 () {
    let root = init_simulator(None);
    let (owner, ex) = setup_exchange(&root, 2000);
    let token1 = test_token(&root, dai(), vec![ex.account_id()]);
    let token2 = test_token(&root, usdt(), vec![ex.account_id()]);
    whitelist_token(&owner, &ex, vec![token1.valid_account_id(), token2.valid_account_id()]);
    deposit_token(&root, &ex, vec![&token1, &token2], vec![1*ONE_DAI, 1*ONE_USDT]);

    call!(
        owner,
        ex.add_stable_swap_pool(
            vec![token1.valid_account_id(), token2.valid_account_id()], 
            vec![18, 6],
            25,
            10000
        ),
        deposit = to_yocto("1")
    ).assert_success();

    // invalid amount list length
    let outcome = call!(
        root,
        ex.add_stable_liquidity(0, vec![U128(1*ONE_DAI), U128(1*ONE_USDT), U128(100000)], U128(1)),
        deposit = to_yocto("0.01")
    );
    assert_failure(outcome, "E64: illegal tokens count");
    let outcome = call!(
        root,
        ex.add_stable_liquidity(0, vec![U128(1*ONE_DAI)], U128(1)),
        deposit = to_yocto("0.01")
    );
    assert_failure(outcome, "E64: illegal tokens count");

    call!(
        root,
        ex.add_stable_liquidity(0, vec![U128(1*ONE_DAI), U128(1*ONE_USDT)], U128(1)),
        deposit = to_yocto("0.01")
    )
    .assert_success();

    let outcome = call!(
        root,
        ex.remove_liquidity(0, U128(1), vec![U128(1), U128(1), U128(1)]),
        deposit = 1
    );
    assert_failure(outcome, "E64: illegal tokens count");
    let outcome = call!(
        root,
        ex.remove_liquidity(0, U128(1), vec![U128(1)]),
        deposit = 1
    );
    assert_failure(outcome, "E64: illegal tokens count");

    let outcome = call!(
        root,
        ex.remove_liquidity_by_tokens(0, vec![U128(1), U128(1), U128(1)], U128(1)),
        deposit = 1
    );
    assert_failure(outcome, "E64: illegal tokens count");
    let outcome = call!(
        root,
        ex.remove_liquidity_by_tokens(0, vec![U128(1)], U128(1)),
        deposit = 1
    );
    assert_failure(outcome, "E64: illegal tokens count");
}

#[test]
fn sim_stable_e65 () {
    let root = init_simulator(None);
    let (owner, ex) = setup_exchange(&root, 2000);
    let token1 = test_token(&root, dai(), vec![ex.account_id()]);
    let token2 = test_token(&root, usdt(), vec![ex.account_id()]);
    whitelist_token(&owner, &ex, vec![token1.valid_account_id(), token2.valid_account_id()]);
    deposit_token(&root, &ex, vec![&token1, &token2], vec![1*ONE_DAI, 1*ONE_USDT]);

    call!(
        owner,
        ex.add_stable_swap_pool(
            vec![token1.valid_account_id(), token2.valid_account_id()], 
            vec![18, 6],
            25,
            10000
        ),
        deposit = to_yocto("1")
    ).assert_success();

    // invalid amount list length
    let outcome = call!(
        root,
        ex.add_stable_liquidity(0, vec![U128(1*ONE_DAI), U128(0*ONE_USDT)], U128(1)),
        deposit = to_yocto("0.01")
    );
    assert_failure(outcome, "E65: init token balance should be non-zero");


    call!(
        root,
        ex.add_stable_liquidity(0, vec![U128(1*ONE_DAI), U128(1*ONE_USDT)], U128(1)),
        deposit = to_yocto("0.01")
    )
    .assert_success();
}

#[test]
fn sim_stable_e13 () {
    let root = init_simulator(None);
    let (owner, ex) = setup_exchange(&root, 2000);
    let token1 = test_token(&root, dai(), vec![ex.account_id()]);
    let token2 = test_token(&root, usdt(), vec![ex.account_id()]);
    whitelist_token(&owner, &ex, vec![token1.valid_account_id(), token2.valid_account_id()]);
    deposit_token(&root, &ex, vec![&token1, &token2], vec![1*ONE_DAI, 1*ONE_USDT]);

    call!(
        owner,
        ex.add_stable_swap_pool(
            vec![token1.valid_account_id(), token2.valid_account_id()], 
            vec![18, 6],
            25,
            10000
        ),
        deposit = to_yocto("1")
    ).assert_success();

    let user = root.create_user("user".to_string(), to_yocto("100"));

    let outcome = call!(
        user,
        ex.remove_liquidity(0, U128(1), vec![U128(1), U128(1)]),
        deposit = 1
    );
    assert_failure(outcome, "E13: LP not registered");

    let outcome = call!(
        user,
        ex.remove_liquidity_by_tokens(0, vec![U128(1), U128(1)], U128(1)),
        deposit = 1
    );
    assert_failure(outcome, "E13: LP not registered");

    let outcome = call!(
        user,
        ex.mft_transfer(":0".to_string(), root.valid_account_id(), U128(1), None),
        deposit = 1
    );
    assert_failure(outcome, "E13: LP not registered");
}

#[test]
fn sim_stable_e34 () {
    let root = init_simulator(None);
    let (owner, ex) = setup_exchange(&root, 2000);
    let token1 = test_token(&root, dai(), vec![ex.account_id()]);
    let token2 = test_token(&root, usdt(), vec![ex.account_id()]);
    whitelist_token(&owner, &ex, vec![token1.valid_account_id(), token2.valid_account_id()]);
    deposit_token(&root, &ex, vec![&token1, &token2], vec![1000*ONE_DAI, 1000*ONE_USDT]);

    call!(
        owner,
        ex.add_stable_swap_pool(
            vec![token1.valid_account_id(), token2.valid_account_id()], 
            vec![18, 6],
            25,
            10000
        ),
        deposit = to_yocto("1")
    ).assert_success();
    call!(
        root,
        ex.add_stable_liquidity(0, vec![U128(1000*ONE_DAI), U128(1000*ONE_USDT)], U128(1)),
        deposit = to_yocto("0.01")
    )
    .assert_success();

    let lp_shares = view!(
        ex.mft_balance_of(":0".to_string(), root.valid_account_id())
    ).unwrap_json::<U128>();
    let lp_shares = lp_shares.0;

    let outcome = call!(
        root,
        ex.remove_liquidity(0, U128(lp_shares + 1), vec![U128(1), U128(1)]),
        deposit = 1
    );
    assert_failure(outcome, "E34: insufficient lp shares");

    call!(
        owner,
        ex.mft_register(":0".to_string(), owner.valid_account_id()),
        deposit = to_yocto("1")
    )
    .assert_success();

    // transfer all lp token to others
    call!(
        root,
        ex.mft_transfer(":0".to_string(), owner.valid_account_id(), U128(lp_shares), None),
        deposit = 1
    )
    .assert_success();

    let outcome = call!(
        root,
        ex.remove_liquidity_by_tokens(0, vec![U128(1*ONE_DAI), U128(1*ONE_USDT)], U128(1)),
        deposit = 1
    );
    assert_failure(outcome, "E34: insufficient lp shares");

    let outcome = call!(
        root,
        ex.mft_transfer(":0".to_string(), owner.valid_account_id(), U128(2*ONE_LPT), None),
        deposit = 1
    );
    assert_failure(outcome, "E34: insufficient lp shares");
}

#[test]
fn sim_stable_e68 () {
    let root = init_simulator(None);
    let (owner, ex) = setup_exchange(&root, 2000);
    let token1 = test_token(&root, dai(), vec![ex.account_id()]);
    let token2 = test_token(&root, usdt(), vec![ex.account_id()]);
    whitelist_token(&owner, &ex, vec![token1.valid_account_id(), token2.valid_account_id()]);
    deposit_token(&root, &ex, vec![&token1, &token2], vec![101*ONE_DAI, 101*ONE_USDT]);

    call!(
        owner,
        ex.add_stable_swap_pool(
            vec![token1.valid_account_id(), token2.valid_account_id()], 
            vec![18, 6],
            25,
            10000
        ),
        deposit = to_yocto("1")
    ).assert_success();
    call!(
        root,
        ex.add_stable_liquidity(0, vec![U128(100*ONE_DAI), U128(100*ONE_USDT)], U128(1)),
        deposit = to_yocto("0.01")
    )
    .assert_success();

    let outcome = call!(
        root,
        ex.remove_liquidity(0, U128(100*ONE_LPT), vec![U128(51*ONE_DAI), U128(50*ONE_USDT)]),
        deposit = 1
    );
    assert_failure(outcome, "E68: slippage error");

    let outcome = call!(
        root,
        ex.remove_liquidity_by_tokens(0, vec![U128(50*ONE_DAI), U128(50*ONE_USDT)], U128(99*ONE_LPT)),
        deposit = 1
    );
    assert_failure(outcome, "E68: slippage error");

    let outcome = call!(
        root,
        ex.swap(
            vec![SwapAction {
                pool_id: 0,
                token_in: dai(),
                amount_in: Some(U128(ONE_DAI)),
                token_out: usdt(),
                min_amount_out: U128(2 * ONE_USDT)
            }],
            None
        ),
        deposit = 1
    );
    assert_failure(outcome, "E68: slippage error");
}

#[test]
fn sim_stable_e69 () {
    let root = init_simulator(None);
    let (owner, ex) = setup_exchange(&root, 2000);
    let token1 = test_token(&root, dai(), vec![ex.account_id()]);
    let token2 = test_token(&root, usdt(), vec![ex.account_id()]);
    whitelist_token(&owner, &ex, vec![token1.valid_account_id(), token2.valid_account_id()]);
    deposit_token(&root, &ex, vec![&token1, &token2], vec![101*ONE_DAI, 1001*ONE_USDT]);

    call!(
        owner,
        ex.add_stable_swap_pool(
            vec![token1.valid_account_id(), token2.valid_account_id()], 
            vec![18, 6],
            25,
            10000
        ),
        deposit = to_yocto("1")
    ).assert_success();
    call!(
        root,
        ex.add_stable_liquidity(0, vec![U128(100*ONE_DAI), U128(100*ONE_USDT)], U128(1)),
        deposit = to_yocto("0.01")
    )
    .assert_success();

    // try to withdraw all from pool
    let outcome = call!(
        root,
        ex.remove_liquidity(0, U128(200*ONE_LPT), vec![U128(1), U128(1)]),
        deposit = 1
    );
    assert_failure(outcome, "E69: pool reserved token balance less than MIN_RESERVE");

    let outcome = call!(
        root,
        ex.remove_liquidity_by_tokens(0, vec![U128(100*ONE_DAI), U128(100*ONE_USDT)], U128(200*ONE_LPT)),
        deposit = 1
    );
    assert_failure(outcome, "E69: pool reserved token balance less than MIN_RESERVE");

    // remove liquidity so that the pool is small enough
    call!(
        root,
        ex.remove_liquidity_by_tokens(0, vec![U128(99*ONE_DAI), U128(99*ONE_USDT)], U128(200*ONE_LPT)),
        deposit = 1
    )
    .assert_success();
    call!(
        root,
        ex.swap(
            vec![SwapAction {
                pool_id: 0,
                token_in: usdt(),
                amount_in: Some(U128(99*ONE_USDT)),
                token_out: dai(),
                min_amount_out: U128(1)
            }],
            None
        ),
        deposit = 1
    ).assert_success();
    let outcome = call!(
        root,
        ex.swap(
            vec![SwapAction {
                pool_id: 0,
                token_in: usdt(),
                amount_in: Some(U128(99*ONE_USDT)),
                token_out: dai(),
                min_amount_out: U128(1)
            }],
            None
        ),
        deposit = 1
    );
    assert_failure(outcome, "E69: pool reserved token balance less than MIN_RESERVE");
}

#[test]
fn sim_stable_e71 () {
    let root = init_simulator(None);
    let (owner, ex) = setup_exchange(&root, 2000);
    let token1 = test_token(&root, dai(), vec![ex.account_id()]);
    let token2 = test_token(&root, usdt(), vec![ex.account_id()]);
    whitelist_token(&owner, &ex, vec![token1.valid_account_id(), token2.valid_account_id()]);
    deposit_token(&root, &ex, vec![&token1, &token2], vec![101*ONE_DAI, 101*ONE_USDT]);

    call!(
        owner,
        ex.add_stable_swap_pool(
            vec![token1.valid_account_id(), token2.valid_account_id()], 
            vec![18, 6],
            25,
            10000
        ),
        deposit = to_yocto("1")
    ).assert_success();
    call!(
        root,
        ex.add_stable_liquidity(0, vec![U128(100*ONE_DAI), U128(100*ONE_USDT)], U128(1)),
        deposit = to_yocto("0.01")
    )
    .assert_success();

    let outcome = call!(
        root,
        ex.swap(
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
fn sim_stable_e14 () {
    let root = init_simulator(None);
    let (owner, ex) = setup_exchange(&root, 2000);
    let token1 = test_token(&root, dai(), vec![ex.account_id()]);
    let token2 = test_token(&root, usdt(), vec![ex.account_id()]);
    whitelist_token(&owner, &ex, vec![token1.valid_account_id(), token2.valid_account_id()]);
    deposit_token(&root, &ex, vec![&token1, &token2], vec![101*ONE_DAI, 101*ONE_USDT]);

    call!(
        owner,
        ex.add_stable_swap_pool(
            vec![token1.valid_account_id(), token2.valid_account_id()], 
            vec![18, 6],
            25,
            10000
        ),
        deposit = to_yocto("1")
    ).assert_success();

    let outcome = call!(
        root,
        ex.mft_register(":0".to_string(), ex.valid_account_id()),
        deposit = to_yocto("1")
    );
    assert_failure(outcome, "E14: LP already registered");
}

#[test]
fn sim_stable_e82 () {
    let mut gc = runtime::GenesisConfig::default();
    gc.genesis_time = 86400 * 1_000_000_000;
    let root = init_simulator(Some(gc));
    let (owner, ex) = setup_exchange(&root, 2000);
    let token1 = test_token(&root, dai(), vec![ex.account_id()]);
    let token2 = test_token(&root, usdt(), vec![ex.account_id()]);
    whitelist_token(&owner, &ex, vec![token1.valid_account_id(), token2.valid_account_id()]);
    deposit_token(&root, &ex, vec![&token1, &token2], vec![101*ONE_DAI, 101*ONE_USDT]);

    call!(
        owner,
        ex.add_stable_swap_pool(
            vec![token1.valid_account_id(), token2.valid_account_id()], 
            vec![18, 6],
            25,
            10000
        ),
        deposit = to_yocto("1")
    ).assert_success();

    let outcome = call!(
        owner,
        ex.stable_swap_ramp_amp(0, 0, U64(0)),
        deposit=1
    );
    assert_failure(outcome, "E82: insufficient ramp time");
}

#[test]
fn sim_stable_e83 () {
    let mut gc = runtime::GenesisConfig::default();
    gc.genesis_time = 86400 * 1_000_000_000;
    let root = init_simulator(Some(gc));
    let (owner, ex) = setup_exchange(&root, 2000);
    let token1 = test_token(&root, dai(), vec![ex.account_id()]);
    let token2 = test_token(&root, usdt(), vec![ex.account_id()]);
    whitelist_token(&owner, &ex, vec![token1.valid_account_id(), token2.valid_account_id()]);
    deposit_token(&root, &ex, vec![&token1, &token2], vec![101*ONE_DAI, 101*ONE_USDT]);

    call!(
        owner,
        ex.add_stable_swap_pool(
            vec![token1.valid_account_id(), token2.valid_account_id()], 
            vec![18, 6],
            25,
            10000
        ),
        deposit = to_yocto("1")
    ).assert_success();

    let runtime = root.borrow_runtime().current_block().block_timestamp;
    println!("{}", runtime);

    let outcome = call!(
        owner,
        ex.stable_swap_ramp_amp(0, 0, U64(3 * 86400 * 1_000_000_000)),
        deposit=1
    );
    assert_failure(outcome, "E83: invalid amp factor");

    let outcome = call!(
        owner,
        ex.stable_swap_ramp_amp(0, 1_000_001, U64(3 * 86400 * 1_000_000_000)),
        deposit=1
    );
    assert_failure(outcome, "E83: invalid amp factor");
}

#[test]
fn sim_stable_e84 () {
    let mut gc = runtime::GenesisConfig::default();
    gc.genesis_time = 86400 * 1_000_000_000;
    let root = init_simulator(Some(gc));
    let (owner, ex) = setup_exchange(&root, 2000);
    let token1 = test_token(&root, dai(), vec![ex.account_id()]);
    let token2 = test_token(&root, usdt(), vec![ex.account_id()]);
    whitelist_token(&owner, &ex, vec![token1.valid_account_id(), token2.valid_account_id()]);
    deposit_token(&root, &ex, vec![&token1, &token2], vec![101*ONE_DAI, 101*ONE_USDT]);

    call!(
        owner,
        ex.add_stable_swap_pool(
            vec![token1.valid_account_id(), token2.valid_account_id()], 
            vec![18, 6],
            25,
            10000
        ),
        deposit = to_yocto("1")
    ).assert_success();

    let outcome = call!(
        owner,
        ex.stable_swap_ramp_amp(0, 1, U64(3 * 86400 * 1_000_000_000)),
        deposit=1
    );
    assert_failure(outcome, "E84: amp factor change is too large");
}
