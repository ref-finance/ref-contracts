use near_sdk::json_types::{U128};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk_sim::{view, ContractAccount};

use std::collections::HashMap;
use ref_farming::{ContractContract as Farming, FarmInfo};
use super::utils::to_va;


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

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct SeedInfo {
    pub seed_id: String,
    pub seed_type: String,
    pub farms: Vec<String>,
    pub next_index: u32,
    pub amount: U128,
    pub min_deposit: U128,
}

pub(crate) fn show_seedsinfo(farming: &ContractAccount<Farming>) -> HashMap<String, SeedInfo> {
    let ret = view!(farming.list_seeds_info(0, 100)).unwrap_json::<HashMap<String, SeedInfo>>();
    for (k, v) in &ret {
        println!("FarmSeed=>  {}: {:#?}", k, v);
    }
    ret
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

