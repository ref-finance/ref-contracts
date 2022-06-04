//! This module captures all the code needed to migrate from previous version.
use near_sdk::collections::{UnorderedMap, LookupMap};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{AccountId, Balance};

use crate::farm::{Farm, FarmId};
use crate::farm_seed::{VersionedFarmSeed, SeedId};
use crate::farmer::VersionedFarmer;
use crate::{ContractData, RunningState};

#[derive(BorshDeserialize, BorshSerialize)]
pub struct ContractDataV0104 {
    pub owner_id: AccountId,
    pub seeds: UnorderedMap<SeedId, VersionedFarmSeed>,
    pub farmers: LookupMap<AccountId, VersionedFarmer>,
    pub farms: UnorderedMap<FarmId, Farm>,
    pub outdated_farms: UnorderedMap<FarmId, Farm>,
    pub farmer_count: u64,
    pub reward_info: UnorderedMap<AccountId, Balance>,
}

impl From<ContractDataV0104> for ContractData {
    fn from(a: ContractDataV0104) -> Self {
        let ContractDataV0104 {
            owner_id,
            seeds,
            farmers,
            farms,
            outdated_farms,
            farmer_count,
            reward_info,
        } = a;
        Self {
            owner_id,
            seeds,
            farmers,
            farms,
            outdated_farms,
            farmer_count,
            reward_info,
            state: RunningState::Running,
        }
    }
}