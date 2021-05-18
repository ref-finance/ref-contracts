//! FarmSeed stores information per seed about 
//! staked seed amount and farms under it.

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{env, Balance};
use crate::errors::*;
use crate::farm::Farm;
use crate::utils::MAX_ACCOUNT_LENGTH;

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
    /// The seed is a FT or MFT
    pub seed_type: SeedType,
    /// all farms that accepted this seed
    /// Future Work: may change to HashMap<GlobalIndex, Farm> 
    /// to enable whole life-circle (especially for removing of farm). 
    pub farms: Vec<Farm>,
    /// total (staked) balance of this seed (Farming Token)
    pub amount: Balance,
}

impl FarmSeed {
    pub fn new(seed_id: &SeedId,) -> Self {
        Self {
            seed_id: seed_id.clone(),
            seed_type: SeedType::FT,
            farms: Vec::new(),
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

    /// Returns amount of yocto near necessary to cover storage used by this data structure.
    pub fn storage_usage(&self) -> Balance {
        (MAX_ACCOUNT_LENGTH + 16) * (self.farms.len() as u128)
            * env::storage_byte_cost()
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

