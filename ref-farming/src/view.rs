//! View functions for the contract.

use std::collections::HashMap;

use near_sdk::json_types::{ValidAccountId, U128, U64};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{near_bindgen, AccountId};

use crate::farm_seed::SeedInfo;
use crate::utils::parse_farm_id;
use crate::simple_farm::DENOM;
use crate::*;

use uint::construct_uint;

construct_uint! {
    /// 256-bit unsigned integer.
    pub struct U256(4);
}

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
    pub start_at: u32,
    pub reward_per_session: U128,
    pub session_interval: u32,

    pub total_reward: U128,
    pub cur_round: u32,
    pub last_round: u32,
    pub claimed_reward: U128,
    pub unclaimed_reward: U128,
    pub beneficiary_reward: U128,
}

impl From<&Farm> for FarmInfo {
    fn from(farm: &Farm) -> Self {
        let farm_kind = farm.kind();
        match farm {
            Farm::SimpleFarm(farm) => {
                if let Some(dis) = farm.try_distribute(&DENOM) {
                    let mut farm_status: String = (&farm.status).into();
                    if farm_status == "Running".to_string()
                        && dis.undistributed == 0
                    {
                        farm_status = "Ended".to_string();
                    }
                    Self {
                        farm_id: farm.farm_id.clone(),
                        farm_kind,
                        farm_status,
                        seed_id: farm.terms.seed_id.clone(),
                        reward_token: farm.terms.reward_token.clone(),
                        start_at: farm.terms.start_at,
                        reward_per_session: farm.terms.reward_per_session.into(),
                        session_interval: farm.terms.session_interval,

                        total_reward: farm.amount_of_reward.into(),
                        cur_round: dis.rr.into(),
                        last_round: farm.last_distribution.rr.into(),
                        claimed_reward: farm.amount_of_claimed.into(),
                        unclaimed_reward: dis.unclaimed.into(),
                        beneficiary_reward: farm.amount_of_beneficiary.into(),
                    }
                } else {
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
                        cur_round: farm.last_distribution.rr.into(),
                        last_round: farm.last_distribution.rr.into(),
                        claimed_reward: farm.amount_of_claimed.into(),
                        // unclaimed_reward: (farm.amount_of_reward - farm.amount_of_claimed).into(),
                        unclaimed_reward: farm.last_distribution.unclaimed.into(),
                        beneficiary_reward: farm.amount_of_beneficiary.into(),
                    }
                }                
            }
        }
    }
}

#[near_bindgen]
impl Contract {
    pub fn get_metadata(&self) -> Metadata {
        Metadata {
            owner_id: self.data().owner_id.clone(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            farmer_count: self.data().farmer_count.into(),
            farm_count: self.data().farms.len().into(),
            seed_count: self.data().seeds.len().into(),
            reward_count: self.data().reward_info.len().into(),
        }
    }

    /// Returns number of farms.
    pub fn get_number_of_farms(&self) -> u64 {
        self.data().farms.len()
    }

    pub fn get_number_of_outdated_farms(&self) -> u64 {
        self.data().outdated_farms.len()
    }

    /// Returns list of farms of given length from given start index.
    pub fn list_farms(&self, from_index: u64, limit: u64) -> Vec<FarmInfo> {
        let keys = self.data().farms.keys_as_vector();

        (from_index..std::cmp::min(from_index + limit, keys.len()))
            .map(|index| 
                (&self.data().farms.get(&keys.get(index).unwrap()).unwrap()).into()
            )
            .collect()
    }

    pub fn list_outdated_farms(&self, from_index: u64, limit: u64) -> Vec<FarmInfo> {
        let keys = self.data().outdated_farms.keys_as_vector();

        (from_index..std::cmp::min(from_index + limit, keys.len()))
            .map(|index| 
                (&self.data().outdated_farms.get(&keys.get(index).unwrap()).unwrap()).into()
            )
            .collect()
    }

    pub fn list_farms_by_seed(&self, seed_id: SeedId) -> Vec<FarmInfo> {
        self.get_seed(&seed_id)
            .get_ref()
            .farms
            .iter()
            .map(|farm_id| 
                (&self.data().farms.get(&farm_id).unwrap()).into()
            )
            .collect()
    }

    /// Returns information about specified farm.
    pub fn get_farm(&self, farm_id: FarmId) -> Option<FarmInfo> {
        if let Some(farm) = self.data().farms.get(&farm_id) {
            Some((&farm).into())
        } else {
            None
        }
    }

    pub fn get_outdated_farm(&self, farm_id: FarmId) -> Option<FarmInfo> {
        if let Some(farm) = self.data().outdated_farms.get(&farm_id) {
            Some((&farm).into())
        } else {
            None
        }
    }

    pub fn list_rewards_info(&self, from_index: u64, limit: u64) -> HashMap<AccountId, U128> {
        let keys = self.data().reward_info.keys_as_vector();
        (from_index..std::cmp::min(from_index + limit, keys.len()))
            .map(|index| {
                (
                    keys.get(index).unwrap(),
                    self.data()
                        .reward_info
                        .get(&keys.get(index).unwrap())
                        .unwrap_or(0)
                        .into(),
                )
            })
            .collect()
    }

    /// Returns reward token claimed for given user outside of any farms.
    /// Returns empty list if no rewards claimed.
    pub fn list_rewards(&self, account_id: ValidAccountId) -> HashMap<AccountId, U128> {
        self.get_farmer_default(account_id.as_ref())
            .get()
            .rewards
            .into_iter()
            .map(|(acc, bal)| (acc, U128(bal)))
            .collect()
    }

    /// Returns balance of amount of given reward token that ready to withdraw.
    pub fn get_reward(&self, account_id: ValidAccountId, token_id: ValidAccountId) -> U128 {
        self.internal_get_reward(account_id.as_ref(), token_id.as_ref())
            .into()
    }

    pub fn get_unclaimed_reward(&self, account_id: ValidAccountId, farm_id: FarmId) -> U128 {
        let (seed_id, _) = parse_farm_id(&farm_id);

        if let (Some(farmer), Some(farm_seed)) = (
            self.get_farmer_wrapped(account_id.as_ref()),
            self.get_seed_wrapped(&seed_id),
        ) {
            if let Some(farm) = self.data().farms.get(&farm_id) {
                let reward_amount = farm.view_farmer_unclaimed_reward(
                    &farmer.get_ref().get_rps(&farm.get_farm_id()),
                    farmer.get_ref().seeds.get(&seed_id).unwrap_or(&0_u128),
                    &farm_seed.get_ref().amount,
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
        let keys = self.data().seeds.keys_as_vector();
        (from_index..std::cmp::min(from_index + limit, keys.len()))
            .map(|index| {
                (
                    keys.get(index).unwrap(),
                    self.get_seed(&keys.get(index).unwrap())
                        .get_ref()
                        .amount
                        .into(),
                )
            })
            .collect()
    }

    /// return user staked seeds and its amount in a hashmap
    pub fn list_user_seeds(&self, account_id: ValidAccountId) -> HashMap<SeedId, U128> {
        if let Some(farmer) = self.get_farmer_wrapped(account_id.as_ref()) {
            farmer
                .get()
                .seeds
                .into_iter()
                .map(|(seed, bal)| (seed.clone(), U128(bal)))
                .collect()
        } else {
            HashMap::new()
        }
    }

    pub fn get_seed_info(&self, seed_id: SeedId) -> Option<SeedInfo> {
        if let Some(farm_seed) = self.get_seed_wrapped(&seed_id) {
            Some(farm_seed.get_ref().into())
        } else {
            None
        }
    }

    pub fn list_seeds_info(&self, from_index: u64, limit: u64) -> HashMap<SeedId, SeedInfo> {
        let keys = self.data().seeds.keys_as_vector();
        (from_index..std::cmp::min(from_index + limit, keys.len()))
            .map(|index| {
                (
                    keys.get(index).unwrap(),
                    self.get_seed(&keys.get(index).unwrap()).get_ref().into(),
                )
            })
            .collect()
    }

    pub fn get_user_rps(&self, account_id: ValidAccountId, farm_id: FarmId) -> String {
        let farmer = self.get_farmer(account_id.as_ref());
        if let Some(rps) = farmer.get().user_rps.get(&farm_id) {
            format!("{}", U256::from_little_endian(&rps))
        } else {
            String::from("0")
        }
    }
}
