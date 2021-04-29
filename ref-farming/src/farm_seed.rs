//! FarmSeed is information per LPT about balances distribution among users.

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{env, Balance};
use crate::errors::*;
use crate::farm::Farm;
use crate::simple_farm::SimpleFarmRewardDistribution;

const MAX_ACCOUNT_LENGTH: u128 = 64;

/// The SeedId contains infomation about <exchange_id, pool_id>, 
/// where exchange_id is ref_exchange in this case.
pub(crate) type SeedId = String;

#[derive(BorshSerialize, BorshDeserialize)]
pub enum SeedType {
    FT,
    MFT,
}

/// record LP token's distribution and farms
#[derive(BorshSerialize, BorshDeserialize)]
#[cfg_attr(feature = "test", derive(Clone))]
pub struct FarmSeed {
    /// The Farming Token this FarmSeed represented for
    pub seed_id: SeedId,
    /// The seed is a FT or MFT
    pub seed_type: SeedType,
    /// all farms that accepted this seed
    /// may change to HashMap<GlobalIndex, Farm> 
    /// to enable whole life-circle (especially for removing of farm). 
    pub xfarms: Vec<Farm>,
    /// total (staked) balance of this seed (Farming Token)
    pub amount: Balance,
}

impl FarmSeed {
    pub fn new(seed_id: &SeedId,) -> Self {
        Self {
            seed_id: seed_id.clone(),
            seed_type: SeedType::FT,
            xfarms: Vec::new(),
            amount: 0,
        }
    }

    // /// all farm that start_at less than cur block height, 
    // /// should reset the start_at to cur block height.
    // pub fn first_farmer_in(&mut self) {
    //     for farm in &mut self.xfarms {
    //         farm.update_start_at(env::block_index());
    //     }
    // }

    pub fn add_amount(&mut self, amount: Balance) {
        self.amount += amount;
    }

    /// return seed amount remains.
    pub fn sub_amount(&mut self, amount: Balance) -> Balance {
        assert!(self.amount >= amount, "{}", ERR500);
        self.amount -= amount;
        self.amount
    }

    /// Returns amount of $NEAR necessary to cover storage used by this data structure.
    pub fn storage_usage(&self) -> Balance {
        (MAX_ACCOUNT_LENGTH + 16) * (self.xfarms.len() as u128)
            * env::storage_byte_cost()
    }

    /// for debug
    pub fn get_farm_dis(&self, index: usize) -> SimpleFarmRewardDistribution {
        self.xfarms.get(index).unwrap().get_farm_dis()
    }
}

