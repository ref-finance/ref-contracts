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

near_sdk_sim::lazy_static_include::lazy_static_include_bytes! {
    TEST_TOKEN_WASM_BYTES => "../res/test_token.wasm",
    EXCHANGE_WASM_BYTES => "../res/ref_exchange_release.wasm",
}

const ONE_LPT: u128 = 1000000000000000000;
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
        t.mint(root.valid_account_id(), to_yocto("1000").into())
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
        root,
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
        pool.add_liquidity(0, vec![U128(100000*ONE_DAI), U128(100000*ONE_USDT), U128(100000*ONE_USDC)], None),
        deposit = to_yocto("0.0007")
    )
    .assert_success();
    (root, owner, pool, token1, token2, token3)
}


#[test]
fn daniel_sim_stable_swap() {
    let (root, _owner, pool, token1, token2, token3) = setup_stable_pool_with_liquidity();
    assert_eq!(
        view!(pool.get_pool(0)).unwrap_json::<PoolInfo>(),
        PoolInfo {
            pool_kind: "STABLE_SWAP".to_string(),
            token_account_ids: vec![token1.account_id(), token2.account_id(), token3.account_id()],
            amounts: vec![U128(100000*ONE_DAI), U128(100000*ONE_USDT), U128(100000*ONE_USDC)],
            total_fee: 25,
            shares_total_supply: U128(300000*ONE_LPT),
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
        300000*ONE_LPT
    );
    let balances = view!(pool.get_deposits(root.valid_account_id()))
        .unwrap_json::<HashMap<AccountId, U128>>();
    let balances = balances.values().cloned().collect::<Vec<_>>();
    assert_eq!(balances, vec![U128(0), U128(0), U128(0)]);

    call!(
        root,
        token1.ft_transfer_call(pool.valid_account_id(), U128(ONE_DAI), None, "".to_string()),
        deposit = 1
    )
    .assert_success();

    call!(
        root,
        pool.swap(
            vec![SwapAction {
                pool_id: 0,
                token_in: dai(),
                amount_in: Some(U128(ONE_DAI)),
                token_out: usdc(),
                min_amount_out: U128(1)
            }],
            None
        ),
        deposit = 1
    )
    .assert_success();
}
