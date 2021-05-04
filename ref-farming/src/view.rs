//! View functions for the contract.

use std::collections::HashMap;

use near_sdk::json_types::{ValidAccountId, U64, U128};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{near_bindgen, AccountId};

use crate::utils::parse_farm_id;
use crate::farm_seed::SeedType;
use crate::*;

#[derive(Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct Metadata {
    pub version: String,
    pub owner_id: AccountId,
    pub farmer_count: U64,
    pub farm_count: U64,
    pub seed_count: U64,
    pub reward_count: U64,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct FarmInfo {
    pub farm_id: FarmId,
    pub farm_kind: String,
    pub farm_status: String,
    pub seed_id: SeedId,
    pub reward_token: AccountId,
    pub start_at: U64,
    pub reward_per_session: U128,
    pub session_interval: U64, 

    pub total_reward: U128,
    pub cur_round: U64,
    pub last_round: U64,
    pub claimed_reward: U128,
    pub unclaimed_reward: U128,
}

impl From<&Farm> for FarmInfo {
    fn from(farm: &Farm) -> Self {
        let farm_kind = farm.kind();
        match farm {
            Farm::SimpleFarm(farm) => {
                let dis = farm.try_distribute(&100_000_000).unwrap_or_default();
                Self {
                    farm_id: farm.farm_id.clone(),
                    farm_kind,
                    farm_status: (&farm.status).into(),
                    seed_id: farm.terms.seed_id.clone(),
                    reward_token: farm.terms.reward_token.clone(),
                    start_at: farm.terms.start_at.into(),
                    reward_per_session: farm.terms.reward_per_session.into(),
                    session_interval: farm.terms.session_interval.into(),

                    total_reward: farm.amount_of_reward.into(),
                    cur_round: dis.rr.into(),
                    last_round: farm.last_distribution.rr.into(),
                    claimed_reward: farm.amount_of_claimed.into(),
                    unclaimed_reward: dis.unclaimed.into(),
                }
            },
        }
    }
}

#[near_bindgen]
impl Contract {
    pub fn get_metadata(&self) -> Metadata {
        Metadata {
            owner_id: self.owner_id.clone(),
            version: String::from("0.1.5"),
            farmer_count: self.farmer_count.into(),
            farm_count: self.farm_count.into(),
            seed_count: self.seeds.len().into(),
            reward_count: self.reward_info.len().into(),
        }
    }

    /// Returns number of farms.
    pub fn get_number_of_farms(&self) -> u64 {
        self.seeds.values().fold(0_u64, |acc, farm_seed| acc + farm_seed.farms.len() as u64)
    }

    /// Returns list of farms of given length from given start index.
    #[allow(unused_variables)]
    pub fn list_farms(&self, from_index: u64, limit: u64) -> Vec<FarmInfo> {
        // TODO: how to page that
        let mut res = vec![];
        for farm_seed in self.seeds.values() {
            let sf = self.list_farms_by_seed(farm_seed.seed_id);
            res.push(sf);
        }
        res.concat()
    }

    pub fn list_farms_by_seed(&self, seed_id: SeedId) -> Vec<FarmInfo> {
        self.seeds.get(&seed_id).unwrap_or(
            FarmSeed {
                seed_id: seed_id.clone(),
                seed_type: SeedType::FT,
                farms: Vec::new(),
                amount: 0,
            })
            .farms.iter().map(|farm| farm.into())
            .collect()
    }

    /// Returns information about specified farm.
    pub fn get_farm(&self, farm_id: FarmId) -> Option<FarmInfo> {
        let (seed_id, index) = parse_farm_id(&farm_id);
        if let Some(farm_seed) = self.seeds.get(&seed_id) {
            if let Some(farm) = farm_seed.farms.get(index) {
                Some(farm.into())
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn list_rewards_info(&self, from_index: u64, limit: u64) -> HashMap<AccountId, U128> {
        let keys = self.reward_info.keys_as_vector();
        (from_index..std::cmp::min(from_index + limit, keys.len()))
            .map(|index| 
                (
                    keys.get(index).unwrap(),
                    self.reward_info.get(&keys.get(index).unwrap()).unwrap_or(0).into(),
                )
            )
            .collect()
    }

    /// Returns reward token claimed for given user outside of any farms.
    /// Returns empty list if no rewards claimed.
    pub fn list_rewards(&self, account_id: ValidAccountId) -> HashMap<AccountId, U128> {
        self.farmers
            .get(account_id.as_ref())
            .map(|d| {
                d.rewards
                    .into_iter()
                    .map(|(acc, bal)| (acc, U128(bal)))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Returns balance of amount of given reward token that ready to withdraw.
    pub fn get_reward(&self, account_id: ValidAccountId, token_id: ValidAccountId) -> U128 {
        self.internal_get_reward(account_id.as_ref(), token_id.as_ref())
            .into()
    }

    pub fn get_unclaimed_reward(&self, account_id: ValidAccountId, farm_id: FarmId) -> U128 {
        let (seed_id, index) = parse_farm_id(&farm_id);

        if let (Some(farmer), Some(farm_seed)) = 
            (self.farmers.get(account_id.as_ref()), self.seeds.get(&seed_id)) {
                if let Some(farm) = farm_seed.farms.get(index) {
                    let reward_amount = farm.view_farmer_unclaimed_reward(
                        &farmer.get_rps(&farm.get_farm_id()),
                        farmer.seeds.get(&seed_id).unwrap_or(&0_u128), 
                        &farm_seed.amount
                    );
                    reward_amount.into()
                } else {
                    0.into()
                }
        } else {
            0.into()
        }
    }

    /// return all seed and its amount staked in this contract in a hashmap
    pub fn list_seeds(&self, from_index: u64, limit: u64) -> HashMap<SeedId, U128> {
        let keys = self.seeds.keys_as_vector();
        (from_index..std::cmp::min(from_index + limit, keys.len()))
            .map(|index| 
                (
                    keys.get(index).unwrap(),
                    self.seeds.get(&keys.get(index).unwrap()).unwrap().amount.into(),
                )
            )
            .collect()
    }

    /// return user staked seeds and its amount in a hashmap
    pub fn list_user_seeds(&self, account_id: ValidAccountId) -> HashMap<SeedId, U128> {
        if let Some(farmer) = self.farmers.get(account_id.as_ref()) {
            farmer.seeds.into_iter().map(|(seed, bal)| (seed, U128(bal))).collect()
        } else {
            HashMap::new()
        }
    }
}
