/*!
* Ref-Farming
*
* lib.rs is the main entry point.
*/
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::{ValidAccountId};
use near_sdk::collections::{LookupMap, UnorderedMap};
use near_sdk::{env, near_bindgen, Balance, AccountId, PanicOnDefault};
// use near_sdk::BorshStorageKey;

use crate::farm::{Farm, FarmId};
use crate::simple_farm::{RPS};
use crate::farm_seed::{VersionedFarmSeed, SeedId};
use crate::farmer::{VersionedFarmer, Farmer};

// for simulator test
pub use crate::simple_farm::HRSimpleFarmTerms;
pub use crate::view::FarmInfo;


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

mod owner;

near_sdk::setup_alloc!();

// wait for main project upgrade to near-sdk 3.1.0
// #[derive(BorshStorageKey, BorshSerialize)]
// pub enum StorageKeys {
//     Seed,
//     Farmer,
//     RewardInfo,
// }

#[derive(BorshDeserialize, BorshSerialize)]
pub struct ContractData {

    // owner of this contract
    owner_id: AccountId,
    
    // record seeds and the farms under it.
    // seeds: UnorderedMap<SeedId, FarmSeed>,
    seeds: UnorderedMap<SeedId, VersionedFarmSeed>,

    // each farmer has a structure to describe
    // farmers: LookupMap<AccountId, Farmer>,
    farmers: LookupMap<AccountId, VersionedFarmer>,

    // for statistic
    farmer_count: u64,
    farm_count: u64,
    reward_info: UnorderedMap<AccountId, Balance>,
}

/// Versioned contract data. Allows to easily upgrade contracts.
#[derive(BorshSerialize, BorshDeserialize)]
pub enum VersionedContractData {
    Current(ContractData),
}

impl VersionedContractData {}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {

    data: VersionedContractData,
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new(owner_id: ValidAccountId) -> Self {
        assert!(!env::state_exists(), "Already initialized");
        Self {
            data: VersionedContractData::Current(ContractData {
                owner_id: owner_id.into(),
                farmer_count: 0,
                farm_count: 0,
                seeds: UnorderedMap::new(b"v".to_vec()),
                farmers: LookupMap::new(b"f".to_vec()),
                reward_info: UnorderedMap::new(b"r".to_vec()),
                // seeds: UnorderedMap::new(StorageKeys::Seed),
                // farmers: LookupMap::new(StorageKeys::Farmer),
                // reward_info: UnorderedMap::new(StorageKeys::RewardInfo),
            }),
        }
    }
}

impl Contract {
    fn data(&self) -> &ContractData {
        match &self.data {
            VersionedContractData::Current(data) => data,
        }
    }

    fn data_mut(&mut self) -> &mut ContractData {
        match &mut self.data {
            VersionedContractData::Current(data) => data,
        }
    }
}

#[cfg(test)]
mod tests {

    use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::{testing_env, Balance, MockedBlockchain, BlockHeight};
    use near_sdk::json_types::{ValidAccountId, U64, U128};
    use simple_farm::{HRSimpleFarmTerms};
    use near_contract_standards::storage_management::{StorageBalance, StorageManagement};

    use super::*;

    fn setup_contract() -> (VMContextBuilder, Contract) {
        let mut context = VMContextBuilder::new();
        testing_env!(context.predecessor_account_id(accounts(0)).build());
        let contract = Contract::new(accounts(0));
        (context, contract)
    }

    fn create_farm(
        context: &mut VMContextBuilder,
        contract: &mut Contract,
        seed: ValidAccountId,
        reward: ValidAccountId,
        session_amount: Balance,
        session_interval: BlockHeight,
    ) -> FarmId {
        // storage needed: 341
        testing_env!(context
            .predecessor_account_id(accounts(0))
            .attached_deposit(env::storage_byte_cost() * 500)
            .build());
        contract.create_simple_farm(HRSimpleFarmTerms {
            seed_id: seed.into(),
            reward_token: reward.into(),
            start_at: U64(0),
            reward_per_session: U128(session_amount),
            session_interval: U64(session_interval),
        }, Some(U128(10)))
    }

    fn deposit_reward(
        context: &mut VMContextBuilder,
        contract: &mut Contract,
    ) {
        testing_env!(context
            .predecessor_account_id(accounts(2))
            .block_index(100)
            .attached_deposit(1)
            .build());
        contract.ft_on_transfer(accounts(0), U128(10000), String::from("bob#0"));
    }

    fn register_farmer(
        context: &mut VMContextBuilder,
        contract: &mut Contract,
        farmer: ValidAccountId,
        height: BlockHeight,
    ) -> StorageBalance {
        testing_env!(context
            .predecessor_account_id(farmer.clone())
            .is_view(false)
            .block_index(height)
            .attached_deposit(env::storage_byte_cost() * 1852)
            .build());
        contract.storage_deposit(Some(farmer), Some(true))
    }

    fn deposit_seed(
        context: &mut VMContextBuilder,
        contract: &mut Contract,
        farmer: ValidAccountId,
        height: BlockHeight,
        amount: Balance,
    ) {
        testing_env!(context
            .predecessor_account_id(accounts(1))
            .is_view(false)
            .block_index(height)
            .attached_deposit(1)
            .build());
        contract.ft_on_transfer(farmer, U128(amount), String::from(""));
    }    

    #[test]
    fn test_basics() {

        let (mut context, mut contract) = setup_contract();

        let farm_id = create_farm(&mut context, &mut contract,
            accounts(1), accounts(2), 5000, 60);
        assert_eq!(farm_id, String::from("bob#0"));
        let farm_info = contract.get_farm(farm_id.clone()).expect("Error");
        assert_eq!(farm_info.farm_kind, String::from("SIMPLE_FARM"));
        assert_eq!(farm_info.farm_status, String::from("Created"));
        assert_eq!(farm_info.seed_id, String::from("bob"));
        assert_eq!(farm_info.reward_token, String::from("charlie"));
        assert_eq!(farm_info.reward_per_session, U128(5000));
        assert_eq!(farm_info.session_interval, U64(60));

        deposit_reward(&mut context, &mut contract);
        let farm_info = contract.get_farm(farm_id.clone()).expect("Error");
        assert_eq!(farm_info.farm_status, String::from("Running"));
        assert_eq!(farm_info.start_at, U64(100));

        // Farmer accounts(0) come in 
        let sb = register_farmer(&mut context, &mut contract, accounts(0), 105);
        println!("accounts(0) total used: {}, deposited: {}", sb.total.0, sb.available.0);
        // deposit seed and still in round 0, no unclaimed
        deposit_seed(&mut context, &mut contract, accounts(0), 110, 10);
        let unclaimed = contract.get_unclaimed_reward(accounts(0), farm_id.clone());
        assert_eq!(unclaimed, U128(0));
        // move to round 1, 5k unclaimed for accounts(0)
        testing_env!(context
            .predecessor_account_id(accounts(0))
            .block_index(180)
            .is_view(true)
            .build());
        let unclaimed = contract.get_unclaimed_reward(accounts(0), farm_id.clone());
        assert_eq!(unclaimed, U128(5000));

        // Farmer accounts(3) come in 
        let sb = register_farmer(&mut context, &mut contract, accounts(3), 190);
        println!("accounts(3) total used: {}, deposited: {}", sb.total.0, sb.available.0);
        // deposit seed and in round 1 height 200
        deposit_seed(&mut context, &mut contract, accounts(3), 200, 10);
        let unclaimed = contract.get_unclaimed_reward(accounts(3), farm_id.clone());
        assert_eq!(unclaimed, U128(0));
        // move to round 2, 7.5k unclaimed for accounts(0), 2.5k unclaimed for accounts(3)
        testing_env!(context
            .predecessor_account_id(accounts(0))
            .block_index(230)
            .is_view(true)
            .build());
        let unclaimed = contract.get_unclaimed_reward(accounts(0), farm_id.clone());
        assert_eq!(unclaimed, U128(7500));
        let unclaimed = contract.get_unclaimed_reward(accounts(3), farm_id.clone());
        assert_eq!(unclaimed, U128(2500));
    }

}