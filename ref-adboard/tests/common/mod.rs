use std::convert::TryFrom;

use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::json_types::{ValidAccountId, U128, U64};
use near_sdk::{AccountId, Balance};
use near_sdk_sim::{call, deploy, to_yocto, view, ContractAccount, UserAccount};

use ref_exchange::{ContractContract as TestRef};
use test_token::ContractContract as TestToken;
use ref_adboard::{ContractContract as AdBoard};

near_sdk_sim::lazy_static_include::lazy_static_include_bytes! {
    TEST_TOKEN_WASM_BYTES => "../res/test_token.wasm",
    EXCHANGE_WASM_BYTES => "../res/ref_exchange_release.wasm",
    ADBOARD_WASM_BYTES => "../res/ref_adboard_local.wasm",
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct HumanReadableFrameMetadata {
    pub token_price: U128,
    pub token_id: AccountId,
    pub owner: AccountId,
    pub protected_ts: U64,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct HumanReadablePaymentItem {
    pub amount: U128,
    pub token_id: AccountId,
    pub receiver_id: AccountId,
}

pub(crate) fn chain_move_and_show(root: &UserAccount, move_blocks: u64) {
    if move_blocks > 0 {
        if root.borrow_runtime_mut().produce_blocks(move_blocks).is_ok() {
            println!("Chain goes {} blocks", move_blocks);
        } else {
            println!("produce_blocks failed!");
        }
    }

    println!("*** Chain Env *** now height: {}, ts: {}", 
        root.borrow_runtime().current_block().block_height,
        root.borrow_runtime().current_block().block_timestamp,
    );

}

pub(crate) fn prepair_pool(
    root: &UserAccount, 
    owner: &UserAccount, 
) -> (ContractAccount<TestRef>, ContractAccount<TestToken>, ContractAccount<TestToken>) {
    let pool = deploy_pool(&root, swap(), owner.account_id());
    let token1 = deploy_token(&root, dai(), vec![swap()]);
    let token2 = deploy_token(&root, eth(), vec![swap()]);
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
    (pool, token1, token2)
}

pub(crate) fn swap_deposit(
    user: &UserAccount, 
    pool: &ContractAccount<TestRef>, 
    token1: &ContractAccount<TestToken>, 
    token2: &ContractAccount<TestToken>, 
) {
    mint_token(&token1, user, to_yocto("105"));
    mint_token(&token2, user, to_yocto("105"));
    call!(
        user,
        pool.storage_deposit(None, None),
        deposit = to_yocto("1")
    )
    .assert_success();
    call!(
        user,
        token1.ft_transfer_call(to_va(swap()), to_yocto("100").into(), None, "".to_string()),
        deposit = 1
    )
    .assert_success();
    call!(
        user,
        token2.ft_transfer_call(to_va(swap()), to_yocto("100").into(), None, "".to_string()),
        deposit = 1
    )
    .assert_success();
}

pub(crate) fn mint_token(token: &ContractAccount<TestToken>, user: &UserAccount, amount: Balance) {
    call!(
        user,
        token.mint(to_va(user.account_id.clone()), amount.into())
    ).assert_success();
}

fn deploy_pool(root: &UserAccount, contract_id: AccountId, owner_id: AccountId) -> ContractAccount<TestRef> {
    let pool = deploy!(
        contract: TestRef,
        contract_id: contract_id,
        bytes: &EXCHANGE_WASM_BYTES,
        signer_account: root,
        init_method: new(to_va(owner_id), 4, 1)
    );
    pool
}

fn deploy_token(
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

pub(crate) fn deploy_adboard(root: &UserAccount, adboard_id: AccountId, owner_id: AccountId) -> ContractAccount<AdBoard> {
    let adboard = deploy!(
        contract: AdBoard,
        contract_id: adboard_id,
        bytes: &ADBOARD_WASM_BYTES,
        signer_account: root,
        init_method: new(
            to_va(owner_id), 
            to_va(swap()), 
            to_va(dai()),
            to_yocto("1").into(),
            30,  // 30 secs
            500,
            100  // 1% fee
        )
    );
    adboard
}

pub(crate) fn dai() -> AccountId {
    "dai".to_string()
}

pub(crate) fn eth() -> AccountId {
    "eth".to_string()
}

pub(crate) fn swap() -> AccountId {
    "swap".to_string()
}

pub(crate) fn adboard_id() -> AccountId {
    "adboard".to_string()
}

pub(crate) fn to_va(a: AccountId) -> ValidAccountId {
    ValidAccountId::try_from(a).unwrap()
}

pub(crate) fn get_frame_metadata(adboard: &ContractAccount<AdBoard>, frame_id: u16) -> Option<HumanReadableFrameMetadata> {
    view!(adboard.get_frame_metadata(frame_id)).unwrap_json::<Option<HumanReadableFrameMetadata>>()
}

pub(crate) fn get_user_token(swap: &ContractAccount<TestRef>, user_id: AccountId, token_id: AccountId) -> U128 {
    view!(swap.get_deposit(to_va(user_id), to_va(token_id))).unwrap_json::<U128>()
}

pub(crate) fn get_failed_payment(adboard: &ContractAccount<AdBoard>) -> Vec<HumanReadablePaymentItem> {
    view!(adboard.list_failed_payments(0, 100)).unwrap_json::<Vec<HumanReadablePaymentItem>>()
}