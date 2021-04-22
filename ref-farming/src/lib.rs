/*!
* Ref-Farming
*
* lib.rs is the main entry point.
*/
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::{ValidAccountId};
use near_sdk::collections::{LookupMap, UnorderedMap};
use near_sdk::{env, near_bindgen, AccountId, PanicOnDefault};

use crate::farm::{Farm, FarmId};
use crate::farm_seed::{FarmSeed, SeedId};
use crate::farmer::Farmer;


mod utils;
mod errors;
mod farmer;
mod token_receiver;
mod farm_seed;
mod farm;
mod simple_farm;
mod storage_impl;

mod actions_of_farm;
mod actions_of_seed;
mod actions_of_reward;
mod view;

/// sodu module is used to debug and testing,
/// remove this module in release version
mod sudo;

near_sdk::setup_alloc!();

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
    // owner of this contract
    owner_id: AccountId,
    
    // record seeds and the farms
    seeds: UnorderedMap::<SeedId, FarmSeed>,

    farmers: LookupMap<AccountId, Farmer>,
    // for statistic
    farmer_count: u64,
    farm_count: u64,
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new(owner_id: ValidAccountId) -> Self {
        assert!(!env::state_exists(), "Already initialized");
        Self {
            owner_id: owner_id.into(),
            // farms: Vector::new(b"f".to_vec()),
            seeds: UnorderedMap::new(b"s".to_vec()),
            farmers: LookupMap::new(b"u".to_vec()),
            farmer_count: 0,
            farm_count: 0,
        }
    }
}

