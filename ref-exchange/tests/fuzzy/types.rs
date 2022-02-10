#![allow(unused)] 
use near_sdk::serde::{Deserialize, Serialize};
use std::collections::HashMap;
use near_sdk::AccountId;
use near_sdk_sim::{
    ContractAccount, UserAccount,
};
use near_sdk::json_types::U128;
use test_token::ContractContract as TestToken;

#[derive(Default)]
pub struct OperationContext {
    pub token_contract_account: HashMap<AccountId, ContractAccount<TestToken>>
}

#[derive(Debug)]
pub enum Preference {
    CreateSamplePool, 
    DirectSwap,
    PoolSwap,
    AddLiquidity
}

#[derive(Debug)]
pub enum Scenario {
    Normal,
    Token1NotRegistered, 
    Token2NotRegistered, 
    Token1NoAccount,
    Token2NoAccount,
    NoStorageDeposit,
}

#[derive(Debug)]
pub enum DSScenario {
    Normal,
    LiquidityEmpty,
    TokenInZero,
    TokenOutZero,
}

#[derive(Debug)]
pub struct Operator {
    pub user: UserAccount,
    pub preference: Preference
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct RefStorageState {
    pub deposit: U128,
    pub usage: U128,
}

/**
 * Related to stable swap
 */

#[derive(Debug)]
pub enum StablePreference {
    RemoveLiquidityByToken,
    RemoveLiquidityByShare,
    PoolSwap,
    AddLiquidity
}

#[derive(Debug)]
pub struct StableOperator {
    pub user: UserAccount,
    pub preference: StablePreference
}

#[derive(Debug, PartialEq, Eq)]
pub enum StableScenario {
    Normal,
    Slippage,
    InsufficientLpShares
}
