/*!
* Ref's Boost Farming
*
* lib.rs is the main entry point.
*/
mod actions_of_farmer_reward;
mod actions_of_farmer_seed;
mod actions_of_seed;
mod big_decimal;
mod booster;
mod errors;
mod events;
mod farmer;
mod farmer_seed;
mod legacy;
mod management;
mod owner;
mod seed;
mod seed_farm;
mod storage_impl;
mod token_receiver;
mod utils;
mod view;


pub use crate::big_decimal::*;
pub use crate::booster::*;
pub use crate::errors::*;
pub use crate::events::*;
pub use crate::farmer::*;
pub use crate::farmer_seed::*;
pub use crate::legacy::*;
pub use crate::owner::{ImportFarmerInfo, ImportSeedInfo};
pub use crate::seed::*;
pub use crate::seed_farm::*;
pub use crate::utils::*;

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LazyOption, LookupMap, UnorderedMap, UnorderedSet};
use near_sdk::json_types::U128;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::BorshStorageKey;
use near_sdk::{
    assert_one_yocto, env, near_bindgen, AccountId, Balance, PanicOnDefault, Promise,
    PromiseResult, Timestamp, log
};

use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(BorshStorageKey, BorshSerialize)]
pub(crate) enum StorageKeys {
    Operator,
    Config,
    Seed,
    Farmer,
    FarmerSeed { account_id: AccountId },
    OutdatedFarm,
    SeedSlashed,
    SeedLostfound,
}

/// Contract config
#[derive(BorshSerialize, BorshDeserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(feature = "test", derive(Deserialize, Clone))]
pub struct Config {
    pub seed_slash_rate: u32,

    /// Key is boosterID, support multiple booster
    pub booster_seeds: HashMap<SeedId, BoosterInfo>,

    pub max_num_farms_per_booster: u32,

    pub max_num_farms_per_seed: u32,

    /// The maximum duration to stake booster token in seconds.
    pub maximum_locking_duration_sec: DurationSec,

    /// The rate of x for the amount of seed given for the maximum locking duration.
    /// Assuming the 100% multiplier at the 0 duration. Should be no less than 100%.
    /// E.g. 20000 means 200% multiplier (or 2X).
    pub max_locking_multiplier: u32,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            seed_slash_rate: DEFAULT_SEED_SLASH_RATE,
            booster_seeds: HashMap::new(),
            max_num_farms_per_booster: DEFAULT_MAX_NUM_FARMS_PER_BOOSTER,
            max_num_farms_per_seed: DEFAULT_MAX_NUM_FARMS_PER_SEED,
            maximum_locking_duration_sec: DEFAULT_MAX_LOCKING_DURATION_SEC,
            max_locking_multiplier: DEFAULT_MAX_LOCKING_REWARD_RATIO,
        }
    }
}

impl Config {
    pub fn assert_valid(&self) {
        assert!(
            self.max_locking_multiplier > MIN_LOCKING_REWARD_RATIO,
            "{}", E200_INVALID_RATIO
        );
    }

    pub fn get_affected_seeds_from_booster(&self, booster_id: &SeedId) -> Option<&BoosterInfo> {
        self.booster_seeds.get(booster_id)
    }

    /// return Vec<(booster, booster_decimal, log_base)> for the given seed
    pub fn get_boosters_from_seed(&self, seed_id: &SeedId) -> Vec<(SeedId, u32, u32)> {
        self.booster_seeds
            .iter()
            .filter(|(k, v)| k.clone() != seed_id && v.affected_seeds.contains_key(seed_id))
            .map(|(k, v)| (k.clone(), v.booster_decimal, v.affected_seeds.get(seed_id).unwrap_or(&0_u32).clone()))
            .collect()
    }
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Eq, PartialEq, Clone)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug))]
pub enum RunningState {
    Running, Paused
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct ContractData {
    pub owner_id: AccountId,
    pub state: RunningState,
    pub operators: UnorderedSet<AccountId>,
    pub config: LazyOption<Config>,
    pub seeds: UnorderedMap<SeedId, VSeed>,
    pub farmers: LookupMap<AccountId, VFarmer>,
    pub outdated_farms: UnorderedMap<FarmId, VSeedFarm>,
    // all slashed seed would recorded in here
    pub seeds_slashed: UnorderedMap<SeedId, Balance>,
    // if unstake seed encounter error, the seed would go to here
    pub seeds_lostfound: UnorderedMap<SeedId, Balance>,

    // for statistic
    farmer_count: u64,
    farm_count: u64,
}

/// Versioned contract data. Allows to easily upgrade contracts.
#[derive(BorshSerialize, BorshDeserialize)]
pub enum VersionedContractData {
    V0100(ContractDataV0100),
    V0101(ContractData),
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
    data: VersionedContractData,
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new(owner_id: AccountId) -> Self {
        assert!(!env::state_exists(), "{}", E000_ALREADY_INIT);
        Self {
            data: VersionedContractData::V0101(ContractData {
                owner_id: owner_id.into(),
                state: RunningState::Running,
                operators: UnorderedSet::new(StorageKeys::Operator),
                config: LazyOption::new(StorageKeys::Config, Some(&Config::default())),
                seeds: UnorderedMap::new(StorageKeys::Seed),
                farmers: LookupMap::new(StorageKeys::Farmer),
                outdated_farms: UnorderedMap::new(StorageKeys::OutdatedFarm),
                seeds_slashed: UnorderedMap::new(StorageKeys::SeedSlashed),
                seeds_lostfound: UnorderedMap::new(StorageKeys::SeedLostfound),
                farmer_count: 0,
                farm_count: 0,
            }),
        }
    }
}

impl Contract {
    pub fn internal_config(&self) -> Config {
        self.data().config.get().unwrap()
    }

    #[allow(unreachable_patterns)]
    fn data(&self) -> &ContractData {
        match &self.data {
            VersionedContractData::V0101(data) => data,
            _ => unimplemented!(),
        }
    }

    #[allow(unreachable_patterns)]
    fn data_mut(&mut self) -> &mut ContractData {
        match &mut self.data {
            VersionedContractData::V0101(data) => data,
            _ => unimplemented!(),
        }
    }

    fn is_owner_or_operators(&self) -> bool {
        env::predecessor_account_id() == self.data().owner_id
            || self
                .data()
                .operators
                .contains(&env::predecessor_account_id())
    }
}
