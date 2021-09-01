//! Wrapper of different types of farms 

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{AccountId, Balance};

use crate::simple_farm::{SimpleFarm, RPS};
use crate::SeedId;

pub(crate) type FarmId = String;

/// Generic Farm, providing wrapper around different implementations of farms.
/// Allows to add new types of farms just by adding extra item in the enum 
/// without needing to migrate the storage.
#[derive(BorshSerialize, BorshDeserialize)]
pub enum Farm {
    SimpleFarm(SimpleFarm),
}

impl Farm {
    /// Returns farm kind.
    pub fn kind(&self) -> String {
        match self {
            Farm::SimpleFarm(_) => "SIMPLE_FARM".to_string(),
        }
    }

    /// return None if the farm can not accept reward anymore
    /// else return amount of undistributed reward 
    pub fn add_reward(&mut self, amount: &Balance) -> Option<Balance> {
        match self {
            Farm::SimpleFarm(farm) => farm.add_reward(amount),
        }
    }

    /// Returns seed id this farm accepted.
    pub fn get_seed_id(&self) -> SeedId {
        match self {
            Farm::SimpleFarm(farm) => farm.terms.seed_id.clone(),
        }
    }

    /// Returns token contract id this farm used for reward.
    pub fn get_reward_token(&self) -> AccountId {
        match self {
            Farm::SimpleFarm(farm) => farm.terms.reward_token.clone(),
        }
    }

    pub fn get_farm_id(&self) -> FarmId {
        match self {
            Farm::SimpleFarm(farm) => farm.farm_id.clone(),
        }
    }

    /// Returns how many reward tokens can given farmer claim.
    pub fn view_farmer_unclaimed_reward(
        &self,
        user_rps: &RPS,
        user_seeds: &Balance,
        total_seeds: &Balance,
    ) -> Balance {
        match self {
            Farm::SimpleFarm(farm) 
                => farm.view_farmer_unclaimed_reward(user_rps, user_seeds, total_seeds),
        }
    }

    /// return the new user reward per seed 
    /// and amount of reward as (user_rps, reward_amount) 
    pub fn claim_user_reward(&mut self, 
        user_rps: &RPS,
        user_seeds: &Balance, 
        total_seeds: &Balance, 
        silent: bool,
    ) -> (RPS, Balance) {
        match self {
            Farm::SimpleFarm(farm) 
                => farm.claim_user_reward(user_rps, user_seeds, total_seeds, silent),
        }
    }

    pub fn can_be_removed(&self, total_seeds: &Balance) -> bool {
        match self {
            Farm::SimpleFarm(farm) => farm.can_be_removed(total_seeds),
        }
    }

    pub fn move_to_clear(&mut self, total_seeds: &Balance) -> bool {
        match self {
            Farm::SimpleFarm(farm) => farm.move_to_clear(total_seeds),
        }
    }

}
