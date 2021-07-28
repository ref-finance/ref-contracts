
use super::utils::to_va;

use near_sdk::{AccountId};
use near_sdk_sim::{call, deploy, to_yocto, ContractAccount, UserAccount};

// use near_sdk_sim::transaction::ExecutionStatus;
use ref_exchange::{ContractContract as TestRef};

use test_token::ContractContract as TestToken;
use ref_farming::{ContractContract as Farming};


near_sdk_sim::lazy_static_include::lazy_static_include_bytes! {
    TEST_TOKEN_WASM_BYTES => "../res/test_token.wasm",
    EXCHANGE_WASM_BYTES => "../res/ref_exchange_release.wasm",
    FARM_WASM_BYTES => "../res/ref_farming_release.wasm",
}

pub fn deploy_farming(root: &UserAccount, farming_id: AccountId, owner_id: AccountId) -> ContractAccount<Farming> {
    let farming = deploy!(
        contract: Farming,
        contract_id: farming_id,
        bytes: &FARM_WASM_BYTES,
        signer_account: root,
        init_method: new(to_va(owner_id))
    );
    farming
}

pub fn deploy_pool(root: &UserAccount, contract_id: AccountId, owner_id: AccountId) -> ContractAccount<TestRef> {
    let pool = deploy!(
        contract: TestRef,
        contract_id: contract_id,
        bytes: &EXCHANGE_WASM_BYTES,
        signer_account: root,
        init_method: new(to_va(owner_id), 4, 1)
    );
    pool
}

pub fn deploy_token(
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
        t.mint(to_va(root.account_id.clone()), to_yocto("10000").into())
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

