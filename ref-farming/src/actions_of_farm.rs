
use near_sdk::{env, near_bindgen, Promise};

use simple_farm::{SimpleFarm, HRSimpleFarmTerms};
use crate::utils::{gen_farm_id};
use crate::errors::*;
use crate::*;


#[near_bindgen]
impl Contract {
    /// create farm and pay for its storage fee
    #[payable]
    pub fn create_simple_farm(&mut self, terms: HRSimpleFarmTerms) -> FarmId {

        let prev_storage = env::storage_usage();

        let farm_id = self.internal_add_farm(&terms);

        // Check how much storage cost and refund the left over back.
        let storage_needed = env::storage_usage() - prev_storage;
        let storage_cost = storage_needed as u128 * env::storage_byte_cost();
        assert!(
            storage_cost <= env::attached_deposit(),
            "{}: {}", ERR11_INSUFFICIENT_STORAGE, storage_needed
        );

        let refund = env::attached_deposit() - storage_cost;
        if refund > 0 {
            Promise::new(env::predecessor_account_id()).transfer(refund);
        }

        farm_id
    }
}

impl Contract {
    /// Adds given farm to the vec and returns it's id.
    /// If there is not enough attached balance to cover storage, fails.
    /// If too much attached - refunds it back.
    fn internal_add_farm(&mut self, terms: &HRSimpleFarmTerms) -> FarmId {
        
        let mut farm_seed = self.get_seed_default(&terms.seed_id);

        let farm_id: FarmId = gen_farm_id(&terms.seed_id, farm_seed.get_ref().farms.len());

        let farm = Farm::SimpleFarm(SimpleFarm::new(
            farm_id.clone(),
            terms.into(),
        ));

        farm_seed.get_ref_mut().farms.push(farm);
        self.data_mut().seeds.insert(&terms.seed_id, &farm_seed);

        self.data_mut().farm_count += 1;
        farm_id
    }
}
