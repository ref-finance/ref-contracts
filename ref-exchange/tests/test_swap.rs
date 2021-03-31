use std::convert::TryFrom;

use near_sdk::json_types::{ValidAccountId, U128};
use near_sdk::AccountId;
use near_sdk_sim::{call, deploy, init_simulator, to_yocto, view, ContractAccount, UserAccount};

use ref_exchange::{ContractContract as Exchange, PoolInfo, SwapAction};
use std::collections::HashMap;
use test_token::ContractContract as TestToken;

near_sdk_sim::lazy_static_include::lazy_static_include_bytes! {
    TEST_TOKEN_WASM_BYTES => "../res/test_token.wasm",
    EXCHANGE_WASM_BYTES => "../res/ref_exchange.wasm",
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

fn dai() -> AccountId {
    "dai".to_string()
}

fn eth() -> AccountId {
    "eth".to_string()
}

fn swap() -> AccountId {
    "swap".to_string()
}

fn to_va(a: AccountId) -> ValidAccountId {
    ValidAccountId::try_from(a).unwrap()
}

#[test]
fn test_swap() {
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
    call!(
        owner,
        pool.extend_whitelisted_tokens(vec![to_va(dai()), to_va(eth())])
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
        pool.add_liquidity(0, vec![U128(to_yocto("5")), U128(to_yocto("10"))]),
        deposit = 1
    )
    .assert_success();
    assert_eq!(
        view!(pool.get_pool(0)).unwrap_json::<PoolInfo>(),
        PoolInfo {
            pool_kind: "SIMPLE_POOL".to_string(),
            token_account_ids: vec![dai(), eth()],
            amounts: vec![to_yocto("5").into(), to_yocto("10").into()],
            total_fee: 30,
            shares_total_supply: to_yocto("1").into(),
        }
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
            None
        ),
        deposit = 1
    )
    .assert_success();

    let balances = view!(pool.get_deposits(to_va(root.account_id.clone())))
        .unwrap_json::<HashMap<AccountId, U128>>();
    assert_eq!(
        balances.get(&eth()).unwrap(),
        &U128(to_yocto("100") + 1662497915624478906119726)
    );
    assert_eq!(balances.get(&dai()).unwrap(), &U128(to_yocto("99")));

    call!(
        root,
        pool.withdraw(to_va(eth()), U128(to_yocto("101")), None),
        deposit = 1
    );
    call!(
        root,
        pool.withdraw(to_va(dai()), U128(to_yocto("99")), None),
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
