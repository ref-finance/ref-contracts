use near_sdk::serde::{Deserialize, Serialize};
use near_sdk_sim::UserAccount;
use near_sdk::json_types::U128;
use uint::construct_uint;

construct_uint! {
    /// 256-bit unsigned integer.
    pub struct U256(4);
}

#[derive(Debug)]
pub enum Preference {
    Stake,
    Unstake,
    Claim,
}

#[derive(Debug)]
pub struct Operator {
    pub user: UserAccount,
    pub preference: Preference
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct StorageBalance {
    pub total: U128,
    pub available: U128,
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