/*!
* Ref-Farming
*
* lib.rs is the main entry point.
*/
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::{ValidAccountId};
use near_sdk::collections::{LookupMap, UnorderedMap};
use near_sdk::{env, near_bindgen, Balance, AccountId, PanicOnDefault};
use near_sdk::BorshStorageKey;

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


#[derive(BorshStorageKey, BorshSerialize)]
pub enum StorageKeys {
    Seed,
    OutdatedFarm,
    Farmer,
    RewardInfo,
    UserRps { account_id: AccountId },
}

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

    outdated_farms: UnorderedMap<FarmId, Farm>,

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
                seeds: UnorderedMap::new(StorageKeys::Seed),
                farmers: LookupMap::new(StorageKeys::Farmer),
                outdated_farms: UnorderedMap::new(StorageKeys::OutdatedFarm),
                reward_info: UnorderedMap::new(StorageKeys::RewardInfo),
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

    use super::utils::*;
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
        session_interval: u32,
    ) -> FarmId {
        // storage needed: 341
        testing_env!(context
            .predecessor_account_id(accounts(0))
            .attached_deposit(env::storage_byte_cost() * 500)
            .build());
        contract.create_simple_farm(HRSimpleFarmTerms {
            seed_id: seed.into(),
            reward_token: reward.into(),
            start_at: 0,
            reward_per_session: U128(session_amount),
            session_interval: session_interval,
        }, Some(U128(10)))
    }

    fn deposit_reward(
        context: &mut VMContextBuilder,
        contract: &mut Contract,
        time_stamp: u32,
    ) {
        testing_env!(context
            .predecessor_account_id(accounts(2))
            .block_timestamp(to_nano(time_stamp))
            .attached_deposit(1)
            .build());
        contract.ft_on_transfer(accounts(0), U128(50000), String::from("bob#0"));
    }

    fn register_farmer(
        context: &mut VMContextBuilder,
        contract: &mut Contract,
        farmer: ValidAccountId,
    ) -> StorageBalance {
        testing_env!(context
            .predecessor_account_id(farmer.clone())
            .is_view(false)
            .attached_deposit(env::storage_byte_cost() * 1852)
            .build());
        contract.storage_deposit(Some(farmer), Some(true))
    }

    fn deposit_seed(
        context: &mut VMContextBuilder,
        contract: &mut Contract,
        farmer: ValidAccountId,
        time_stamp: u32,
        amount: Balance,
    ) {
        testing_env!(context
            .predecessor_account_id(accounts(1))
            .is_view(false)
            .block_timestamp(to_nano(time_stamp))
            .attached_deposit(1)
            .build());
        contract.ft_on_transfer(farmer, U128(amount), String::from(""));
    }    

    fn withdraw_seed(
        context: &mut VMContextBuilder,
        contract: &mut Contract,
        farmer: ValidAccountId,
        time_stamp: u32,
        amount: Balance,
    ) {
        testing_env!(context
            .predecessor_account_id(farmer)
            .is_view(false)
            .block_timestamp(to_nano(time_stamp))
            .attached_deposit(1)
            .build());
        contract.withdraw_seed(accounts(1).into(), U128(amount));
    } 

    fn claim_reward(
        context: &mut VMContextBuilder,
        contract: &mut Contract,
        farmer: ValidAccountId,
        time_stamp: u32
    ) {
        testing_env!(context
            .predecessor_account_id(farmer)
            .is_view(false)
            .block_timestamp(to_nano(time_stamp))
            .attached_deposit(1)
            .build());
        contract.claim_reward_by_farm(String::from("bob#0"));
    }

    fn remove_farm(context: &mut VMContextBuilder, contract: &mut Contract, time_stamp: u32) {
        testing_env!(context
            .predecessor_account_id(accounts(0))
            .is_view(false)
            .block_timestamp(to_nano(time_stamp))
            .build());
        contract.clean_farm_by_seed(accounts(1).into());
    }

    fn remove_user_rps(context: &mut VMContextBuilder, contract: &mut Contract, farmer: ValidAccountId, farm_id: String, time_stamp: u32) -> bool {
        testing_env!(context
            .predecessor_account_id(farmer)
            .is_view(false)
            .block_timestamp(to_nano(time_stamp))
            .build());
        contract.remove_user_rps_by_farm(farm_id)
    }

    #[test]
    fn test_basics() {

        let (mut context, mut contract) = setup_contract();
        // seed is bob, reward is charlie
        let farm_id = create_farm(&mut context, &mut contract,
            accounts(1), accounts(2), 5000, 50);
        assert_eq!(farm_id, String::from("bob#0"));
        let farm_info = contract.get_farm(farm_id.clone()).expect("Error");
        assert_eq!(farm_info.farm_kind, String::from("SIMPLE_FARM"));
        assert_eq!(farm_info.farm_status, String::from("Created"));
        assert_eq!(farm_info.seed_id, String::from("bob"));
        assert_eq!(farm_info.reward_token, String::from("charlie"));
        assert_eq!(farm_info.reward_per_session, U128(5000));
        assert_eq!(farm_info.session_interval, 50);

        // deposit 50k, can last 10 rounds from 0 to 9
        deposit_reward(&mut context, &mut contract, 100);
        let farm_info = contract.get_farm(farm_id.clone()).expect("Error");
        assert_eq!(farm_info.farm_status, String::from("Running"));
        assert_eq!(farm_info.start_at, 100);

        // Farmer accounts(0) come in round 1
        let sb = register_farmer(&mut context, &mut contract, accounts(0));
        deposit_seed(&mut context, &mut contract, accounts(0), 160, 10);
        let unclaimed = contract.get_unclaimed_reward(accounts(0), farm_id.clone());
        assert_eq!(unclaimed, U128(0));
        let farm_info = contract.get_farm(farm_id.clone()).expect("Error");
        assert_eq!(farm_info.beneficiary_reward, U128(5000));
        assert_eq!(farm_info.cur_round, 1);
        assert_eq!(farm_info.last_round, 1);

        // move to round 2, 5k unclaimed for accounts(0)
        testing_env!(context
            .predecessor_account_id(accounts(0))
            .block_timestamp(to_nano(210))
            .is_view(true)
            .build());
        let unclaimed = contract.get_unclaimed_reward(accounts(0), farm_id.clone());
        assert_eq!(unclaimed, U128(5000));
        let farm_info = contract.get_farm(farm_id.clone()).expect("Error");
        assert_eq!(farm_info.cur_round, 2);
        assert_eq!(farm_info.last_round, 1);

        // Farmer accounts(3) come in 
        let sb = register_farmer(&mut context, &mut contract, accounts(3));
        // deposit seed
        deposit_seed(&mut context, &mut contract, accounts(3), 260, 10);
        let unclaimed = contract.get_unclaimed_reward(accounts(3), farm_id.clone());
        assert_eq!(unclaimed, U128(0));
        let unclaimed = contract.get_unclaimed_reward(accounts(0), farm_id.clone());
        assert_eq!(unclaimed, U128(10000));
        let farm_info = contract.get_farm(farm_id.clone()).expect("Error");
        assert_eq!(farm_info.cur_round, 3);
        assert_eq!(farm_info.last_round, 3);

        // move to round 4, 
        testing_env!(context
            .predecessor_account_id(accounts(0))
            .block_timestamp(to_nano(320))
            .is_view(true)
            .build());
        let unclaimed = contract.get_unclaimed_reward(accounts(0), farm_id.clone());
        assert_eq!(unclaimed, U128(12500));
        let unclaimed = contract.get_unclaimed_reward(accounts(3), farm_id.clone());
        assert_eq!(unclaimed, U128(2500));
        let farm_info = contract.get_farm(farm_id.clone()).expect("Error");
        assert_eq!(farm_info.cur_round, 4);
        assert_eq!(farm_info.last_round, 3);

        // remove all seeds at round 5
        println!("----> remove all seeds at round 5");
        withdraw_seed(&mut context, &mut contract, accounts(0), 360, 10);
        withdraw_seed(&mut context, &mut contract, accounts(3), 370, 10);
        testing_env!(context.predecessor_account_id(accounts(0))
            .block_timestamp(to_nano(380)).is_view(true).build());
        let unclaimed = contract.get_unclaimed_reward(accounts(0), farm_id.clone());
        assert_eq!(unclaimed, U128(0));
        let unclaimed = contract.get_unclaimed_reward(accounts(3), farm_id.clone());
        assert_eq!(unclaimed, U128(0));
        let rewarded = contract.get_reward(accounts(0), accounts(2));
        assert_eq!(rewarded, U128(15000));
        let rewarded = contract.get_reward(accounts(3), accounts(2));
        assert_eq!(rewarded, U128(5000));
        let farm_info = contract.get_farm(farm_id.clone()).expect("Error");
        assert_eq!(farm_info.cur_round, 5);
        assert_eq!(farm_info.last_round, 5);


        // move to round 7, account3 come in again
        println!("----> move to round 7, account3 come in again");
        deposit_seed(&mut context, &mut contract, accounts(3), 460, 10);
        let unclaimed = contract.get_unclaimed_reward(accounts(3), farm_id.clone());
        assert_eq!(unclaimed, U128(0));
        let unclaimed = contract.get_unclaimed_reward(accounts(0), farm_id.clone());
        assert_eq!(unclaimed, U128(0));
        let farm_info = contract.get_farm(farm_id.clone()).expect("Error");
        assert_eq!(farm_info.beneficiary_reward, U128(15000));
        assert_eq!(farm_info.cur_round, 7);
        assert_eq!(farm_info.last_round, 7);

        // move to round 8, account0 come in again
        println!("----> move to round 8, account0 come in again");
        deposit_seed(&mut context, &mut contract, accounts(0), 520, 10);
        let unclaimed = contract.get_unclaimed_reward(accounts(3), farm_id.clone());
        assert_eq!(unclaimed, U128(5000));
        let unclaimed = contract.get_unclaimed_reward(accounts(0), farm_id.clone());
        assert_eq!(unclaimed, U128(0));
        let farm_info = contract.get_farm(farm_id.clone()).expect("Error");
        assert_eq!(farm_info.cur_round, 8);
        assert_eq!(farm_info.last_round, 8);

        // move to round 9,
        testing_env!(context
            .predecessor_account_id(accounts(0))
            .block_timestamp(to_nano(580))
            .is_view(true)
            .build());
        let unclaimed = contract.get_unclaimed_reward(accounts(0), farm_id.clone());
        assert_eq!(unclaimed, U128(2500));
        let unclaimed = contract.get_unclaimed_reward(accounts(3), farm_id.clone());
        assert_eq!(unclaimed, U128(7500));
        let farm_info = contract.get_farm(farm_id.clone()).expect("Error");
        assert_eq!(farm_info.cur_round, 9);
        assert_eq!(farm_info.last_round, 8);
        assert_eq!(farm_info.farm_status, String::from("Running"));

        // move to round 10,
        testing_env!(context
            .predecessor_account_id(accounts(0))
            .block_timestamp(to_nano(610))
            .is_view(true)
            .build());
        let unclaimed = contract.get_unclaimed_reward(accounts(0), farm_id.clone());
        assert_eq!(unclaimed, U128(5000));
        let unclaimed = contract.get_unclaimed_reward(accounts(3), farm_id.clone());
        assert_eq!(unclaimed, U128(10000));
        let farm_info = contract.get_farm(farm_id.clone()).expect("Error");
        assert_eq!(farm_info.cur_round, 10);
        assert_eq!(farm_info.last_round, 8);
        assert_eq!(farm_info.farm_status, String::from("Ended"));

        // claim reward 
        println!("----> accounts(0) and accounts(3) claim reward");
        testing_env!(context
            .predecessor_account_id(accounts(0))
            .block_timestamp(to_nano(710))
            .is_view(true)
            .build());
        let unclaimed = contract.get_unclaimed_reward(accounts(0), farm_id.clone());
        assert_eq!(unclaimed, U128(5000));
        let unclaimed = contract.get_unclaimed_reward(accounts(3), farm_id.clone());
        assert_eq!(unclaimed, U128(10000));
        claim_reward(&mut context, &mut contract, accounts(0), 720);
        claim_reward(&mut context, &mut contract, accounts(3), 730);
        testing_env!(context.predecessor_account_id(accounts(0))
            .block_timestamp(to_nano(740)).is_view(true).build());
        let unclaimed = contract.get_unclaimed_reward(accounts(0), farm_id.clone());
        assert_eq!(unclaimed, U128(0));
        let unclaimed = contract.get_unclaimed_reward(accounts(3), farm_id.clone());
        assert_eq!(unclaimed, U128(0));
        let rewarded = contract.get_reward(accounts(0), accounts(2));
        assert_eq!(rewarded, U128(20000));
        let rewarded = contract.get_reward(accounts(3), accounts(2));
        assert_eq!(rewarded, U128(15000));
        let farm_info = contract.get_farm(farm_id.clone()).expect("Error");
        assert_eq!(farm_info.cur_round, 10);
        assert_eq!(farm_info.last_round, 10);

        // clean farm
        println!("----> clean farm");
        remove_farm(&mut context, &mut contract, 750);
        assert!(contract.get_farm(farm_id.clone()).is_none());

        // remove user rps
        println!("----> remove user rps");
        testing_env!(context.predecessor_account_id(accounts(0))
            .block_timestamp(to_nano(760)).is_view(true).build());
        let prev_locked = contract.storage_balance_of(accounts(0)).expect("Error").total.0;
        let ret = remove_user_rps(&mut context, &mut contract, accounts(0).into(), String::from("bob#0"), 770);
        assert!(ret);
        testing_env!(context.predecessor_account_id(accounts(0))
            .block_timestamp(to_nano(780)).is_view(true).build());
        let post_locked = contract.storage_balance_of(accounts(0)).expect("Error").total.0;
        assert_eq!(prev_locked - post_locked, 165*10_u128.pow(19));

        // withdraw seed
        println!("----> accounts(0) and accounts(3) withdraw seed");
        withdraw_seed(&mut context, &mut contract, accounts(0), 800, 10);
        withdraw_seed(&mut context, &mut contract, accounts(3), 810, 10);
        testing_env!(context.predecessor_account_id(accounts(0))
            .block_timestamp(to_nano(820)).is_view(true).build());
        let rewarded = contract.get_reward(accounts(0), accounts(2));
        assert_eq!(rewarded, U128(20000));
        let rewarded = contract.get_reward(accounts(3), accounts(2));
        assert_eq!(rewarded, U128(15000));
        
    }

}