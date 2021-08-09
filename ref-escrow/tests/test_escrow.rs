use std::convert::TryFrom;

use near_sdk::json_types::{ValidAccountId, U128};
use near_sdk::serde_json;
use near_sdk::AccountId;
use near_sdk_sim::{
    call, deploy, init_simulator, to_yocto, view, ContractAccount, ExecutionResult, UserAccount,
};

use ref_escrow::{Account, ContractContract as Escrow, Offer, ReceiverMessage};
use test_token::ContractContract as TestToken;

near_sdk_sim::lazy_static_include::lazy_static_include_bytes! {
    TEST_TOKEN_WASM_BYTES => "../res/test_token.wasm",
    ESCROW_WASM_BYTES => "../res/ref_escrow_local.wasm",
}

fn to_va(a: AccountId) -> ValidAccountId {
    ValidAccountId::try_from(a).unwrap()
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

fn balance_of(token: &ContractAccount<TestToken>, account_id: &AccountId) -> u128 {
    view!(token.ft_balance_of(to_va(account_id.clone())))
        .unwrap_json::<U128>()
        .0
}

fn accounts(i: usize) -> AccountId {
    vec![
        "escrow".to_string(),
        "dai".to_string(),
        "usdt".to_string(),
        "user1".to_string(),
        "user2".to_string(),
    ][i]
        .clone()
}

#[test]
fn test_basics() {
    let root = init_simulator(None);
    let escrow = deploy!(
        contract: Escrow,
        contract_id: accounts(0),
        bytes: &ESCROW_WASM_BYTES,
        signer_account: root,
        init_method: new()
    );
    let user1 = root.create_user(accounts(3), to_yocto("1000"));
    let user2 = root.create_user(accounts(4), to_yocto("1000"));
    let token1 = test_token(&root, accounts(1), vec![accounts(0), accounts(4)]);
    call!(
        user1,
        token1.mint(to_va(accounts(3)), U128(to_yocto("1000")))
    )
    .assert_success();
    let token2 = test_token(&root, accounts(2), vec![accounts(0), accounts(3)]);
    call!(
        user2,
        token2.mint(to_va(accounts(4)), U128(to_yocto("1000")))
    )
    .assert_success();
    call!(
        user1,
        escrow.storage_deposit(None, None),
        deposit = to_yocto("1")
    )
    .assert_success();
    call!(
        user2,
        escrow.storage_deposit(None, None),
        deposit = to_yocto("1")
    )
    .assert_success();
    call!(
        user1,
        token1.ft_transfer_call(
            to_va(accounts(0)),
            U128(to_yocto("500")),
            None,
            serde_json::to_string(&ReceiverMessage::Offer {
                taker: None,
                take_token_id: to_va(accounts(2)),
                take_min_amount: U128(to_yocto("50")),
                min_offer_time: 100.into(),
                max_offer_time: 1000000000000000.into()
            })
            .unwrap()
        ),
        deposit = 1
    )
    .assert_success();
    let offer: Offer = view!(escrow.get_offer(0)).unwrap_json();
    assert_eq!(offer.offerer, accounts(3));
    call!(
        user2,
        token2.ft_transfer_call(
            to_va(accounts(0)),
            U128(to_yocto("50")),
            None,
            serde_json::to_string(&ReceiverMessage::Take { offer_id: 0 }).unwrap()
        ),
        deposit = 1
    )
    .assert_success();
    let account1: Account = view!(escrow.get_account(to_va(accounts(3)))).unwrap_json();
    let account2: Account = view!(escrow.get_account(to_va(accounts(4)))).unwrap_json();
    assert_eq!(
        account1.amounts.into_iter().collect::<Vec<_>>(),
        vec![(accounts(2), U128(to_yocto("50")))]
    );
    assert_eq!(
        account2.amounts.into_iter().collect::<Vec<_>>(),
        vec![(accounts(1), U128(to_yocto("500")))]
    );
}
