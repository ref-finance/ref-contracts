use near_sdk::json_types::{U128};
use near_sdk::{Balance, AccountId};
use near_sdk_sim::{call, deploy, view, init_simulator, to_yocto, ContractAccount, UserAccount};
// use near_sdk_sim::transaction::ExecutionStatus;
use ref_exchange::{ContractContract as TestRef};
use test_token::ContractContract as TestToken;
use ref_farming::{HRSimpleFarmTerms, ContractContract as Farming, FarmInfo};
use near_sdk::serde_json::Value;
use near_sdk::json_types::{ValidAccountId};
use std::convert::TryFrom;
use std::collections::HashMap;
use crate::fuzzy::{
    constant::*,
    types::*,
};



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


pub fn dai() -> AccountId {
    "dai".to_string()
}

pub fn eth() -> AccountId {
    "eth".to_string()
}

pub fn swap() -> AccountId {
    "swap".to_string()
}

pub fn farming_id() -> AccountId {
    "farming".to_string()
}

pub fn to_va(a: AccountId) -> ValidAccountId {
    ValidAccountId::try_from(a).unwrap()
}

pub fn prepair_env(
) -> (UserAccount, UserAccount, ContractAccount<Farming>, ContractAccount<TestRef>, Vec<Operator>) {

    let root = init_simulator(None);

    println!("----->> Prepare accounts.");
    let owner = root.create_user("owner".to_string(), to_yocto("100"));
    let farmer_stake = root.create_user("farmer_stake".to_string(), to_yocto("100"));
    let farmer_unstake = root.create_user("farmer_unstake".to_string(), to_yocto("100"));
    let farmer_claim = root.create_user("farmer_claim".to_string(), to_yocto("100"));
    println!("<<----- owner and 3 farmers prepared.");

    println!("----->> Deploy farming and register farmers.");
    let farming = deploy_farming(&root, farming_id(), owner.account_id());
    call!(farmer_stake, farming.storage_deposit(None, None), deposit = to_yocto("1")).assert_success();
    call!(farmer_unstake, farming.storage_deposit(None, None), deposit = to_yocto("1")).assert_success();
    call!(farmer_claim, farming.storage_deposit(None, None), deposit = to_yocto("1")).assert_success();
    println!("<<----- farming deployed, farmers registered.");

    let pool = deploy_pool(&root, swap(), owner.account_id());
    let token1 = deploy_token(&root, dai(), vec![swap()]);
    let token2 = deploy_token(&root, eth(), vec![swap()]);
    call!(owner, pool.extend_whitelisted_tokens(vec![to_va(dai()), to_va(eth())]), deposit=1)
    .assert_success();

    call!(root,
        pool.add_simple_pool(vec![to_va(dai()), to_va(eth())], 25),
        deposit = to_yocto("1")
    ).assert_success();

    call!(root, pool.mft_register(":0".to_string(), to_va(farming_id())), deposit = to_yocto("1"))
    .assert_success();

    add_liqudity(&farmer_stake, &pool, &token1, &token2, 0);
    add_liqudity(&farmer_unstake, &pool, &token1, &token2, 0);
    add_liqudity(&farmer_claim, &pool, &token1, &token2, 0);
    call!(
        farmer_stake,
        pool.add_liquidity(0, vec![to_yocto(&(10 * OPERATION_NUM).to_string()).into(), to_yocto(&(10 * OPERATION_NUM).to_string()).into()], None),
        deposit = to_yocto("0.01")
    )
    .assert_success();

    println!("----->> Create farm.");
    let farm_id = FARM_ID.to_string();
    let out_come = call!(
        owner,
        farming.create_simple_farm(HRSimpleFarmTerms{
            seed_id: format!("{}@0", pool.account_id()),
            reward_token: token1.valid_account_id(),
            start_at: 0,
            reward_per_session: to_yocto("1").into(),
            session_interval: 60,
        }, None),
        deposit = to_yocto("1")
    );
    out_come.assert_success();
    assert_eq!(Value::String(farm_id.clone()), out_come.unwrap_json_value());
    println!("<<----- Farm {} created at #{}, ts:{}.", 
    farm_id,
    root.borrow_runtime().current_block().block_height, 
    root.borrow_runtime().current_block().block_timestamp);

    println!("----->> Deposit reward to turn farm Running.");
    call!(
        root,
        token1.storage_deposit(Some(to_va(farming_id())), None),
        deposit = to_yocto("1")
    )
    .assert_success();
    mint_token(&token1, &root, to_yocto(&OPERATION_NUM.to_string()));
    call!(
        root,
        token1.ft_transfer_call(to_va(farming_id()), U128(to_yocto(&OPERATION_NUM.to_string())), None, farm_id.clone()),
        deposit = 1
    )
    .assert_success();
    show_farminfo(&farming, farm_id.clone(), true);
    println!("<<----- Farm {} deposit reward at #{}, ts:{}.", 
    farm_id,
    root.borrow_runtime().current_block().block_height, 
    root.borrow_runtime().current_block().block_timestamp);

    (root, owner, farming, pool, vec![Operator{user: farmer_stake, preference: Preference::Stake}, Operator{user: farmer_unstake, preference: Preference::Unstake}, Operator{user: farmer_claim, preference: Preference::Claim}])
}

pub fn add_liqudity(
    user: &UserAccount, 
    pool: &ContractAccount<TestRef>, 
    token1: &ContractAccount<TestToken>, 
    token2: &ContractAccount<TestToken>, 
    pool_id: u64,
) {
    mint_token(&token1, user, to_yocto(&(100 * OPERATION_NUM).to_string()));
    mint_token(&token2, user, to_yocto(&(100 * OPERATION_NUM).to_string()));
    call!(
        user,
        pool.storage_deposit(None, None),
        deposit = to_yocto("1")
    )
    .assert_success();
    call!(
        user,
        token1.ft_transfer_call(to_va(swap()), to_yocto(&(100 * OPERATION_NUM).to_string()).into(), None, "".to_string()),
        deposit = 1
    )
    .assert_success();
    call!(
        user,
        token2.ft_transfer_call(to_va(swap()), to_yocto(&(100 * OPERATION_NUM).to_string()).into(), None, "".to_string()),
        deposit = 1
    )
    .assert_success();
    call!(
        user,
        pool.add_liquidity(pool_id, vec![U128(to_yocto("10")), U128(to_yocto("10"))], None),
        deposit = to_yocto("0.01")
    )
    .assert_success();
}

pub fn mint_token(token: &ContractAccount<TestToken>, user: &UserAccount, amount: Balance) {
    call!(
        user,
        token.mint(to_va(user.account_id.clone()), amount.into())
    ).assert_success();
}

pub fn show_farminfo(
    farming: &ContractAccount<Farming>,
    farm_id: String,
    show_print: bool,
) -> FarmInfo {
    let farm_info = get_farminfo(farming, farm_id);
    if show_print {
        println!("Farm Info ===>");
        println!(
            "  ID:{}, Status:{}, Seed:{}, Reward:{}",
            farm_info.farm_id, farm_info.farm_status, farm_info.seed_id, farm_info.reward_token
        );
        println!(
            "  StartAt:{}, SessionReward:{}, SessionInterval:{}",
            farm_info.start_at, farm_info.reward_per_session.0, farm_info.session_interval
        );
        println!(
            "  TotalReward:{}, Claimed:{}, Unclaimed:{}, LastRound:{}, CurRound:{}",
            farm_info.total_reward.0,
            farm_info.claimed_reward.0,
            farm_info.unclaimed_reward.0,
            farm_info.last_round,
            farm_info.cur_round
        );
    }
    farm_info
}

fn get_farminfo(farming: &ContractAccount<Farming>, farm_id: String) -> FarmInfo {
    view!(farming.get_farm(farm_id)).unwrap_json::<FarmInfo>()
}

pub fn show_userseeds(
    farming: &ContractAccount<Farming>,
    user_id: String,
    show_print: bool,
) -> HashMap<String, U128> {
    let ret = view!(farming.list_user_seeds(to_va(user_id.clone())))
        .unwrap_json::<HashMap<String, U128>>();
    if show_print {
        println!("User Seeds for {}: {:#?}", user_id, ret);
    }
    ret
}

pub(crate) fn show_unclaim(
    farming: &ContractAccount<Farming>,
    user_id: String,
    farm_id: String,
    show_print: bool,
) -> U128 {
    let farm_info = get_farminfo(farming, farm_id.clone());
    let ret = view!(farming.get_unclaimed_reward(to_va(user_id.clone()), farm_id.clone()))
        .unwrap_json::<U128>();
    if show_print {
        println!(
            "User Unclaimed for {}@{}:[CRR:{}, LRR:{}] {}",
            user_id, farm_id, farm_info.cur_round, farm_info.last_round, ret.0
        );
    }
    ret
}

pub fn assert_farming(
    farm_info: &FarmInfo,
    farm_status: String,
    total_reward: u128,
    cur_round: u32,
    last_round: u32,
    claimed_reward: u128,
    unclaimed_reward: u128,
    beneficiary_reward: u128,
) {
    assert_eq!(farm_info.farm_status, farm_status);
    assert_eq!(farm_info.total_reward.0, total_reward);
    assert_eq!(farm_info.cur_round, cur_round);
    assert_eq!(farm_info.last_round, last_round);
    assert_eq!(farm_info.claimed_reward.0, claimed_reward);
    assert_eq!(farm_info.unclaimed_reward.0, unclaimed_reward);
    assert_eq!(farm_info.beneficiary_reward.0, beneficiary_reward);
}