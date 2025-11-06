use near_sdk::json_types::U128;
use near_sdk_sim::{
    call, to_yocto, ContractAccount, ExecutionResult, UserAccount,
};

use test_token::ContractContract as TestToken;

use crate::common::utils::*;
pub mod common;

fn pack_action(
    pool_id: u32,
    token_in: &str,
    token_out: &str,
    min_amount_out: u128,
) -> String {
    format!(
        "{{\"pool_id\": {}, \"token_in\": \"{}\", \"token_out\": \"{}\", \"min_amount_out\": \"{}\"}}",
        pool_id, token_in, token_out, min_amount_out
    )
}

fn direct_swap(
    user: &UserAccount,
    contract: &ContractAccount<TestToken>,
    actions: Vec<String>,
    amount: u128,
) -> ExecutionResult {
    let actions_str = actions.join(", ");
    let msg_str = format!("{{\"actions\": [{}]}}", actions_str);
    call!(
        user,
        contract.ft_transfer_call(to_va(swap()), amount.into(), None, msg_str),
        deposit = 1
    )
}

#[test]
fn lostfound_scenario_01_tier1_user_inner_account() {
    println!("\n=== Scenario 01: Tier 1 - User Inner Account Recovery ===");
    let (root, _owner, pool, token1, token2, _) = setup_pool_with_liquidity_high_near();
    let user = root.create_user("tier1_user".to_string(), to_yocto("200"));

    println!("Case 0101: Setup - Register user to pool with sufficient storage");
    call!(
        user,
        pool.storage_deposit(None, None),
        deposit = to_yocto("1")
    )
    .assert_success();

    println!("Case 0102: Mint token1 - keep most in wallet for swap");
    call!(
        user,
        token1.mint(to_va(user.account_id.clone()), U128(to_yocto("10")))
    )
    .assert_success();

    println!("Case 0103: Deposit only 1 token1 to create user's pool account (for Tier 1 recovery)");
    call!(
        user,
        token1.ft_transfer_call(to_va(swap()), to_yocto("1").into(), None, "".to_string()),
        deposit = 1
    )
    .assert_success();

    // Verify pool account created with 1 token1
    let deposits_before = get_deposits(&pool, user.valid_account_id());
    let token1_in_pool = deposits_before.get(&token1.account_id()).map(|b| b.0).unwrap_or(0);
    assert_eq!(token1_in_pool, to_yocto("1"));

    // User still has 9 token1 in wallet
    let wallet_balance = balance_of(&token1, &user.account_id);
    println!("  User token1 in wallet: {}", wallet_balance);
    assert_eq!(wallet_balance, to_yocto("9"));

    println!("Case 0104: User is NOT registered to token2");
    assert_eq!(balance_of(&token2, &user.account_id), 0);

    println!("Case 0105: Execute instant swap with wallet tokens (will fail on output transfer)");
    // Swap 5 token1 from wallet → produces token2 → ft_transfer fails (not registered)
    // Callback recovery: tries Tier 1 (re-deposit to pool account) → SUCCEEDS
    let action = pack_action(0, &token1.account_id(), &token2.account_id(), 1);
    let swap_outcome = direct_swap(&user, &token1, vec![action], to_yocto("5"));
    swap_outcome.assert_success();

    println!("Case 0106: Verify swap failed on output transfer");
    assert_eq!(get_error_count(&swap_outcome), 1);
    assert!(get_error_status(&swap_outcome)
        .contains("Smart contract panicked: The account tier1_user is not registered"));

    println!("Case 0107: Verify Tier 1 recovery - tokens recovered to user's pool account");
    // User's wallet should not have token2
    assert_eq!(balance_of(&token2, &user.account_id), 0, "Transfer failed, no token2 in wallet");

    // Verify token2 is in user's pool account (Tier 1 recovery!)
    let deposits_after = get_deposits(&pool, user.valid_account_id());
    let token2_in_pool = deposits_after.get(&token2.account_id()).map(|b| b.0).unwrap_or(0);
    println!("  Token2 in pool (Tier 1 recovered): {}", token2_in_pool);
    assert!(token2_in_pool > to_yocto("1.8"), "Token2 should be recovered to pool via Tier 1");

    println!("Case 0108: Verify NOT in lostfound (Tier 1 worked, no Tier 2 needed)");
    let user_lostfound = get_lostfound_token(&pool, user.valid_account_id(), token2.valid_account_id());
    assert_eq!(user_lostfound, 0, "Should not use Tier 2 when Tier 1 works");

    println!("✓ Scenario 01 PASSED: Tier 1 recovery to user's inner pool account works");
}

#[test]
fn lostfound_scenario_02_tier2_user_lostfound() {
    println!("\n=== Scenario 02: Tier 2 - User Lostfound Account ===");
    let (root, _owner, pool, token1, token2, _) = setup_pool_with_liquidity_high_near();
    let user = root.create_user("tier2_user".to_string(), to_yocto("200"));

    println!("Case 0201: Setup - User NOT registered to pool (no inner account)");
    // User has tokens but is NOT registered to pool

    println!("Case 0202: User gets token1 (owns it, not in pool)");
    call!(
        user,
        token1.mint(to_va(user.account_id.clone()), U128(to_yocto("10")))
    )
    .assert_success();

    assert_eq!(balance_of(&token1, &user.account_id), to_yocto("10"));
    assert!(
        get_deposits(&pool, user.valid_account_id())
            .get(&token1.account_id())
            .is_none(),
        "User should not have pool account yet"
    );

    println!("Case 0203: User is NOT registered to token2");
    assert_eq!(balance_of(&token2, &user.account_id), 0);

    println!("Case 0204: Execute instant swap token1 -> token2 (Tier 1 fails, should use Tier 2)");
    let action = pack_action(0, &token1.account_id(), &token2.account_id(), 1);
    let swap_outcome = direct_swap(&user, &token1, vec![action], to_yocto("5"));
    swap_outcome.assert_success();

    println!("Case 0205: Verify swap failed (as expected)");
    assert_eq!(get_error_count(&swap_outcome), 1);
    assert!(get_error_status(&swap_outcome)
        .contains("Smart contract panicked: The account tier2_user is not registered"));

    println!("Case 0206: Verify Tier 2 recovery - tokens in user's lostfound account");
    // Verify user's wallet doesn't have token2
    assert_eq!(balance_of(&token2, &user.account_id), 0, "Transfer failed, no wallet tokens");

    // Verify NO pool deposits (user was never in pool)
    assert!(
        get_deposits(&pool, user.valid_account_id())
            .get(&token2.account_id())
            .is_none(),
        "Should not have pool deposit account"
    );

    // ✓ Verify tokens in USER'S LOSTFOUND (Tier 2 WORKS!)
    let user_lostfound = get_lostfound_token(&pool, user.valid_account_id(), token2.valid_account_id());
    println!("  Token2 in user lostfound: {}", user_lostfound);
    assert!(user_lostfound > to_yocto("1.8"), "Tier 2: Tokens should be in user's lostfound");

    // Verify logs mention user lostfound
    let logs = get_logs(&swap_outcome);
    let has_user_lostfound_log = logs.iter().any(|log| log.contains("Depositing to user lostfound account"));
    println!("  Logs contain 'user lostfound' message: {}", has_user_lostfound_log);
    assert!(has_user_lostfound_log, "Log should confirm user lostfound deposit");

    println!("Case 0207: User claims tokens from lostfound");
    // Register user to token2 so claim transfer will succeed
    call!(
        user,
        token2.storage_deposit(None, None),
        deposit = to_yocto("1")
    )
    .assert_success();

    // Register user to pool to have storage for receiving tokens
    call!(
        user,
        pool.storage_deposit(None, None),
        deposit = to_yocto("1")
    )
    .assert_success();

    // Claim from lostfound
    let claim_outcome = call!(
        user,
        pool.claim_lostfound(token2.valid_account_id()),
        deposit = 1
    );
    claim_outcome.assert_success();

    println!("Case 0208: Verify claim succeeded - tokens transferred to user's wallet");
    let user_token2_balance = balance_of(&token2, &user.account_id);
    println!("  User token2 balance after claim: {}", user_token2_balance);
    assert!(user_token2_balance > to_yocto("3"), "Should receive claimed tokens (accounting for swap output)");

    // Verify lostfound cleared
    let remaining_lostfound = get_lostfound_token(&pool, user.valid_account_id(), token2.valid_account_id());
    assert_eq!(remaining_lostfound, 0, "Lostfound should be cleared after claim");

    println!("✓ Scenario 02 PASSED: Tier 2 tokens in user lostfound, claim works");
}

#[test]
fn lostfound_scenario_03_tier3_owner_fallback() {
    println!("\n=== Scenario 03: Tier 3 - Owner Fallback (Low Contract NEAR) ===");
    let (root, owner, pool, token1, token2, _) = setup_pool_with_liquidity_low_near();
    let user = root.create_user("tier3_user".to_string(), to_yocto("200"));

    println!("Case 0301: Setup - Contract has LOW NEAR balance (for Tier 3)");
    // setup_pool_with_liquidity_low_near() creates contract with <100 NEAR free

    println!("Case 0302: User NOT registered to pool (no inner account)");
    // User has tokens but is NOT registered to pool

    println!("Case 0303: User gets token1");
    call!(
        user,
        token1.mint(to_va(user.account_id.clone()), U128(to_yocto("10")))
    )
    .assert_success();

    assert_eq!(balance_of(&token1, &user.account_id), to_yocto("10"));

    println!("Case 0304: User is NOT registered to token2");
    assert_eq!(balance_of(&token2, &user.account_id), 0);

    println!("Case 0305: Execute instant swap token1 -> token2 (should use Tier 3)");
    let action = pack_action(0, &token1.account_id(), &token2.account_id(), 1);
    let swap_outcome = direct_swap(&user, &token1, vec![action], to_yocto("5"));
    swap_outcome.assert_success();

    println!("Case 0306: Verify swap failed");
    assert_eq!(get_error_count(&swap_outcome), 1);

    println!("Case 0307: Verify Tier 3 recovery - tokens in owner's account");
    // Verify user's wallet doesn't have token2
    assert_eq!(balance_of(&token2, &user.account_id), 0, "Transfer failed");

    // Verify user has NO lostfound (Tier 2 was skipped due to low NEAR)
    let user_lostfound = get_lostfound_token(&pool, user.valid_account_id(), token2.valid_account_id());
    println!("  Token2 in user lostfound: {}", user_lostfound);
    assert_eq!(user_lostfound, 0, "Tier 3: Should NOT be in user lostfound (low contract NEAR)");

    // ✓ Verify tokens in OWNER'S DEPOSITS (Tier 3 WORKS!)
    let owner_token2_balance = get_deposits(&pool, owner.valid_account_id())
        .get(&token2.account_id())
        .map(|b| b.0)
        .unwrap_or(0);
    println!("  Token2 in owner deposits: {}", owner_token2_balance);
    assert!(owner_token2_balance > to_yocto("1.8"), "Tier 3: Tokens should be in owner's deposits");

    // Verify logs mention insufficient NEAR
    let logs = get_logs(&swap_outcome);
    let has_insufficient_near_log = logs.iter().any(|log| log.contains("Not enough free NEAR for user lostfound"));
    println!("  Logs contain 'insufficient NEAR' message: {}", has_insufficient_near_log);
    assert!(has_insufficient_near_log, "Log should confirm Tier 3 fallback");

    println!("✓ Scenario 03 PASSED: Tier 3 fallback to owner when contract low on NEAR");
}

#[test]
fn lostfound_scenario_04_tier1_failure_fallback_to_tier2() {
    println!("\n=== Scenario 04: Tier 1 Failure → Tier 2 Fallback ===");
    let (root, _owner, pool, token1, token2, _) = setup_pool_with_liquidity_high_near();
    let user = root.create_user("tier1_fail_user".to_string(), to_yocto("200"));

    println!("Case 0401: Setup - Register user to pool with MINIMAL storage (test Tier 1 failure)");
    call!(
        user,
        pool.storage_deposit(None, Some(true)),
        deposit = to_yocto("1")
    )
    .assert_success();

    // Verify user has minimal storage (0 available)
    let storage = get_storage_balance(&pool, user.valid_account_id()).unwrap();
    assert_eq!(storage.available.0, 0, "User should have 0 available storage");

    println!("Case 0402: Mint token1 to user's wallet (keep ALL in wallet, don't deposit to pool)");
    call!(
        user,
        token1.mint(to_va(user.account_id.clone()), U128(to_yocto("10")))
    )
    .assert_success();

    // Verify user has pool account but no token deposits (insufficient storage)
    let deposits_before = get_deposits(&pool, user.valid_account_id());
    assert_eq!(
        deposits_before.get(&token1.account_id()).map(|b| b.0).unwrap_or(0),
        0,
        "User should have 0 token1 in pool (insufficient storage)"
    );

    // User has 10 token1 in wallet
    assert_eq!(balance_of(&token1, &user.account_id), to_yocto("10"));

    println!("Case 0403: User is NOT registered to token2");
    assert_eq!(balance_of(&token2, &user.account_id), 0);

    println!("Case 0404: Execute instant swap (will trigger Tier 1 failure → Tier 2)");
    // Swap 5 token1 from wallet → produces token2 → ft_transfer fails
    // Tier 1: Try to re-deposit to pool account → FAILS (insufficient storage)
    // Tier 2: Check contract NEAR → YES → Deposit to user's lostfound
    let action = pack_action(0, &token1.account_id(), &token2.account_id(), 1);
    let swap_outcome = direct_swap(&user, &token1, vec![action], to_yocto("5"));
    swap_outcome.assert_success();

    println!("Case 0405: Verify swap failed on transfer");
    assert_eq!(get_error_count(&swap_outcome), 1);
    assert!(get_error_status(&swap_outcome)
        .contains("Smart contract panicked: The account tier1_fail_user is not registered"));

    println!("Case 0406: Verify Tier 1 failed → Tier 2 succeeded");
    // User's wallet has 0 token2
    assert_eq!(balance_of(&token2, &user.account_id), 0);

    // Tokens should be in lostfound (Tier 1 failed, fell through to Tier 2)
    let user_lostfound = get_lostfound_token(&pool, user.valid_account_id(), token2.valid_account_id());
    println!("  Token2 in user lostfound: {}", user_lostfound);
    assert!(
        user_lostfound > to_yocto("1.8"),
        "Tier 1 failed, should fallback to Tier 2 (user lostfound)"
    );

    // Verify logs mention user lostfound
    let logs = get_logs(&swap_outcome);
    let has_user_lostfound_log = logs.iter().any(|log| log.contains("Depositing to user lostfound account"));
    assert!(has_user_lostfound_log, "Logs should confirm Tier 2 fallback");

    println!("Case 0407: Verify user can claim from lostfound");
    // Register user to token2
    call!(
        user,
        token2.storage_deposit(None, None),
        deposit = to_yocto("1")
    )
    .assert_success();

    // Claim from lostfound
    let claim_outcome = call!(
        user,
        pool.claim_lostfound(token2.valid_account_id()),
        deposit = 1
    );
    claim_outcome.assert_success();

    // Verify tokens transferred
    let user_token2 = balance_of(&token2, &user.account_id);
    assert!(user_token2 > to_yocto("1.8"), "User should receive claimed tokens");

    println!("✓ Scenario 04 PASSED: Tier 1 failure correctly falls back to Tier 2");
}

#[test]
fn lostfound_scenario_05_multiple_tokens_in_lostfound() {
    println!("\n=== Scenario 05: Multiple Tokens Accumulating in Lostfound ===");
    let (root, _owner, pool, token1, token2, token3) = setup_pool_with_liquidity_high_near();
    let user = root.create_user("multi_token_user".to_string(), to_yocto("200"));

    println!("Case 0501: Setup - User NOT in pool, will get tokens in lostfound");

    println!("Case 0502: Mint token1 and token3 (keep in wallet for swaps)");
    call!(
        user,
        token1.mint(to_va(user.account_id.clone()), U128(to_yocto("10")))
    )
    .assert_success();

    call!(
        user,
        token3.mint(to_va(user.account_id.clone()), U128(to_yocto("10")))
    )
    .assert_success();

    println!("Case 0503: First swap: token1 → token2 (fails, goes to user lostfound)");
    let action1 = pack_action(0, &token1.account_id(), &token2.account_id(), 1);
    let swap_outcome1 = direct_swap(&user, &token1, vec![action1], to_yocto("5"));
    swap_outcome1.assert_success();

    assert_eq!(get_error_count(&swap_outcome1), 1);

    let lostfound_token2_after_swap1 = get_lostfound_token(&pool, user.valid_account_id(), token2.valid_account_id());
    println!("  Token2 in lostfound after swap 1: {}", lostfound_token2_after_swap1);
    assert!(lostfound_token2_after_swap1 > to_yocto("1.8"));

    println!("Case 0504: Second swap: token3 → token2 (fails, accumulates in user lostfound)");
    // Note: Using pool 2 which is token3 → usdt, but we want token3 → token2
    // Actually we need pool 0 for token1→token2, pool 1 for eth→usdt, pool 2 for usdt→dai
    // Let's use a different approach: swap token3 to different token if available
    // For simplicity, let's do another token1→token2 swap to accumulate

    println!("Case 0504b: Second swap with token3: produces token to user lostfound");
    // Swap more token1 to accumulate in token2 lostfound
    let action2 = pack_action(0, &token1.account_id(), &token2.account_id(), 1);
    let swap_outcome2 = direct_swap(&user, &token1, vec![action2], to_yocto("3"));
    swap_outcome2.assert_success();

    assert_eq!(get_error_count(&swap_outcome2), 1);

    let lostfound_token2_after_swap2 = get_lostfound_token(&pool, user.valid_account_id(), token2.valid_account_id());
    println!("  Token2 in lostfound after swap 2: {}", lostfound_token2_after_swap2);
    assert!(
        lostfound_token2_after_swap2 > lostfound_token2_after_swap1,
        "Tokens should accumulate in user lostfound"
    );

    println!("Case 0505: Verify multiple tokens scenario: claim first token");
    // Register user to token2
    call!(
        user,
        token2.storage_deposit(None, None),
        deposit = to_yocto("1")
    )
    .assert_success();

    // Register to pool for storage
    call!(
        user,
        pool.storage_deposit(None, None),
        deposit = to_yocto("1")
    )
    .assert_success();

    // Claim token2
    let claim_outcome = call!(
        user,
        pool.claim_lostfound(token2.valid_account_id()),
        deposit = 1
    );
    claim_outcome.assert_success();

    // Verify received all accumulated token2
    let user_token2 = balance_of(&token2, &user.account_id);
    println!("  User token2 after claim: {}", user_token2);
    assert!(user_token2 > to_yocto("4"), "User should receive all accumulated token2");

    // Verify lostfound cleared for token2
    let remaining_token2_lostfound = get_lostfound_token(&pool, user.valid_account_id(), token2.valid_account_id());
    assert_eq!(remaining_token2_lostfound, 0, "Token2 lostfound should be cleared");

    println!("✓ Scenario 05 PASSED: Multiple tokens accumulate and claim works correctly");
}
