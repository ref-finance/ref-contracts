//! FarmSeed stores information per seed about 
//! staked seed amount and farms under it.

use std::collections::HashMap;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{Balance};
use crate::errors::*;
use crate::farm::{Farm, FarmId};


/// For MFT, SeedId composes of token_contract_id 
/// and token's inner_id in that contract. 
/// For FT, SeedId is the token_contract_id.
pub(crate) type SeedId = String;

#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub enum SeedType {
    FT,
    MFT,
}


#[derive(BorshSerialize, BorshDeserialize)]
#[cfg_attr(feature = "test", derive(Clone))]
pub struct FarmSeed {
    /// The Farming Token this FarmSeed represented for
    pub seed_id: SeedId,
    /// The seed is a FT or MFT, enum size is 2 bytes?
    pub seed_type: SeedType,
    /// all farms that accepted this seed
    /// FarmId = {seed_id}#{next_index}
    pub farms: HashMap<FarmId, Farm>,
    pub next_index: u32,
    /// total (staked) balance of this seed (Farming Token)
    pub amount: Balance,
}

impl FarmSeed {
    pub fn new(seed_id: &SeedId,) -> Self {
        Self {
            seed_id: seed_id.clone(),
            seed_type: SeedType::FT,
            // farms: Vec::new(),
            farms: HashMap::new(),
            next_index: 0,
            amount: 0,
        }
    }

    pub fn add_amount(&mut self, amount: Balance) {
        self.amount += amount;
    }

    /// return seed amount remains.
    pub fn sub_amount(&mut self, amount: Balance) -> Balance {
        assert!(self.amount >= amount, "{}", ERR500);
        self.amount -= amount;
        self.amount
    }

}

/// Versioned FarmSeed, used for lazy upgrade.
/// Which means this structure would upgrade automatically when used.
/// To achieve that, each time the new version comes in, 
/// each function of this enum should be carefully re-code!
#[derive(BorshSerialize, BorshDeserialize)]
pub enum VersionedFarmSeed {
    V101(FarmSeed),
}

impl VersionedFarmSeed {

    pub fn new(seed_id: &SeedId) -> Self {
        VersionedFarmSeed::V101(FarmSeed::new(seed_id))
    }

    /// Upgrades from other versions to the currently used version.
    pub fn upgrade(self) -> Self {
        match self {
            VersionedFarmSeed::V101(farm_seed) => VersionedFarmSeed::V101(farm_seed),
        }
    }

    #[inline]
    #[allow(unreachable_patterns)]
    pub fn need_upgrade(&self) -> bool {
        match self {
            VersionedFarmSeed::V101(_) => false,
            _ => true,
        }
    }

    #[inline]
    #[allow(unreachable_patterns)]
    pub fn get_ref(&self) -> &FarmSeed {
        match self {
            VersionedFarmSeed::V101(farm_seed) => farm_seed,
            _ => unimplemented!(),
        }
    }

    #[inline]
    #[allow(unreachable_patterns)]
    pub fn get_ref_mut(&mut self) -> &mut FarmSeed {
        match self {
            VersionedFarmSeed::V101(farm_seed) => farm_seed,
            _ => unimplemented!(),
        }
    }
}

