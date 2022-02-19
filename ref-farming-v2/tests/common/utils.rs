#![allow(unused)] 
use std::convert::TryFrom;
use near_sdk::{AccountId};
use near_sdk::json_types::{ValidAccountId};


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

pub(crate) fn generate_cd_account_msg(index: u64, seed_id: String, cd_strategy: usize) -> String{
    format!("{{\"index\": {}, \"seed_id\": \"{}\", \"cd_strategy\": {}}}", index, seed_id, cd_strategy)
}

pub(crate) fn append_cd_account_msg(index: u64, seed_id: String) -> String{
    format!("{{\"index\": {}, \"seed_id\": \"{}\"}}", index, seed_id)
}

pub(crate) fn generate_reward_msg(farm_id: String) -> String{
    format!("{{\"farm_id\": \"{}\"}}", farm_id)
}

pub(crate) fn to_nano(timestamp: u64) -> u64 {
    timestamp * 10u64.pow(9)
}