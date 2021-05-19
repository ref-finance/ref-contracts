use std::convert::TryFrom;

use near_sdk::json_types::{ValidAccountId, U128, U64};
use near_sdk::{AccountId, Balance};
use near_sdk_sim::{call, deploy, to_yocto, view, ContractAccount, UserAccount};

// use near_sdk_sim::transaction::ExecutionStatus;
use ref_exchange::{ContractContract as TestRef};
use std::collections::HashMap;
use test_token::ContractContract as TestToken;
use ref_farming::{ContractContract as Farming, FarmInfo};
use ref_farming::{HRSimpleFarmTerms};
use near_sdk::serde_json::Value;

near_sdk_sim::lazy_static_include::lazy_static_include_bytes! {
    TEST_TOKEN_WASM_BYTES => "../res/test_token.wasm",
    EXCHANGE_WASM_BYTES => "../res/ref_exchange_release.wasm",
    FARM_WASM_BYTES => "../res/ref_farming_local.wasm",
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

pub(crate) fn prepair_farm(
    root: &UserAccount, 
    owner: &UserAccount,
    token: &ContractAccount<TestToken>,
) -> (ContractAccount<Farming>, String) {
    // create farm
    let farming = deploy_farming(&root, farming_id(), owner.account_id());
    let out_come = call!(
        root,
        farming.create_simple_farm(HRSimpleFarmTerms{
            seed_id: format!("{}@0", swap()),
            reward_token: to_va(dai()),
            start_at: U64(0),
            reward_per_session: to_yocto("1").into(),
            session_interval: U64(60),
        }),
        deposit = to_yocto("1")
    );
    out_come.assert_success();
    let farm_id: String;
    if let Value::String(farmid) = out_come.unwrap_json_value() {
        farm_id = farmid.clone();
    } else {
        farm_id = String::from("N/A");
    }
    println!("Farm {} created at Height#{}", farm_id.clone(), root.borrow_runtime().current_block().block_height);
    
    // deposit reward token
    call!(
        root,
        token.storage_deposit(Some(to_va(farming_id())), None),
        deposit = to_yocto("1")
    )
    .assert_success();
    call!(
        root,
        token.ft_transfer_call(to_va(farming_id()), to_yocto("500").into(), None, farm_id.clone()),
        deposit = 1
    )
    .assert_success();
    println!("Farm running at Height#{}", root.borrow_runtime().current_block().block_height);

    (farming, farm_id)
}

pub(crate) fn add_liqudity(
    user: &UserAccount, 
    pool: &ContractAccount<TestRef>, 
    token1: &ContractAccount<TestToken>, 
    token2: &ContractAccount<TestToken>, 
    pool_id: u64,
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
    call!(
        user,
        pool.add_liquidity(pool_id, vec![U128(to_yocto("100")), U128(to_yocto("100"))], None),
        deposit = to_yocto("0.01")
    )
    .assert_success();
}

pub(crate) fn mint_token(token: &ContractAccount<TestToken>, user: &UserAccount, amount: Balance) {
    // call!(
    //     user,
    //     token.storage_deposit(None, None),
    //     deposit = to_yocto("1")
    // )
    // .assert_success();
    call!(
        user,
        token.mint(to_va(user.account_id.clone()), amount.into())
    ).assert_success();
}

pub(crate) fn show_farminfo(farming: &ContractAccount<Farming>, farm_id: String) -> FarmInfo {
    let farm_info = get_farminfo(farming, farm_id);
    println!("Farm Info ===>");
    println!("  ID:{}, Status:{}, Seed:{}, Reward:{}", 
        farm_info.farm_id, farm_info.farm_status, farm_info.seed_id, farm_info.reward_token);
    println!("  StartAt:{}, SessionReward:{}, SessionInterval:{}", 
        farm_info.start_at.0, farm_info.reward_per_session.0, farm_info.session_interval.0);
    println!("  TotalReward:{}, Claimed:{}, Unclaimed:{}, LastRound:{}, CurRound:{}", 
        farm_info.total_reward.0, farm_info.claimed_reward.0, farm_info.unclaimed_reward.0, 
        farm_info.last_round.0, farm_info.cur_round.0);
    
    farm_info
}

pub(crate) fn show_userseeds(farming: &ContractAccount<Farming>, user_id: String) -> HashMap<String, U128> {
    let ret = view!(farming.list_user_seeds(to_va(user_id.clone()))).unwrap_json::<HashMap<String, U128>>();
    println!("User Seeds for {}: {:#?}", user_id, ret);
    ret
}

pub(crate) fn show_unclaim(farming: &ContractAccount<Farming>, user_id: String, farm_id: String) -> U128 {
    let farm_info = get_farminfo(farming, farm_id.clone());
    let ret = view!(farming.get_unclaimed_reward(to_va(user_id.clone()), farm_id.clone())).unwrap_json::<U128>();
    println!("User Unclaimed for {}@{}:[CRR:{}, LRR:{}] {}", 
        user_id, farm_id, farm_info.cur_round.0, farm_info.last_round.0 , ret.0);
    ret
}

pub(crate) fn show_reward(farming: &ContractAccount<Farming>, user_id: String, reward_id: String) -> U128 {
    let ret = view!(
        farming.get_reward(to_va(user_id.clone()), to_va(reward_id.clone()))
    ).unwrap_json::<U128>();
    println!("Reward {} for {}: {}", reward_id, user_id, ret.0);
    ret
}


// =============  internal methods ================
fn get_farminfo(farming: &ContractAccount<Farming>, farm_id: String) -> FarmInfo {
    view!(farming.get_farm(farm_id)).unwrap_json::<FarmInfo>()
}

fn deploy_farming(root: &UserAccount, farming_id: AccountId, owner_id: AccountId) -> ContractAccount<Farming> {
    let farming = deploy!(
        contract: Farming,
        contract_id: farming_id,
        bytes: &FARM_WASM_BYTES,
        signer_account: root,
        init_method: new(to_va(owner_id))
    );
    farming
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


pub(crate) fn dai() -> AccountId {
    "dai".to_string()
}

pub(crate) fn eth() -> AccountId {
    "eth".to_string()
}

pub(crate) fn swap() -> AccountId {
    "swap".to_string()
}

pub(crate) fn farming_id() -> AccountId {
    "farming".to_string()
}

pub(crate) fn to_va(a: AccountId) -> ValidAccountId {
    ValidAccountId::try_from(a).unwrap()
}


