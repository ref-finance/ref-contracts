
use near_sdk::{env, near_bindgen, Promise};
use near_sdk::json_types::{U128};
use simple_farm::{SimpleFarm, HRSimpleFarmTerms};
use crate::utils::{gen_farm_id, MIN_SEED_DEPOSIT};
use crate::errors::*;
use crate::*;


#[near_bindgen]
impl Contract {
    /// create farm and pay for its storage fee
    #[payable]
    pub fn create_simple_farm(&mut self, terms: HRSimpleFarmTerms, min_deposit: Option<U128>) -> FarmId {

        let prev_storage = env::storage_usage();

        let min_deposit: u128 = min_deposit.unwrap_or(U128(MIN_SEED_DEPOSIT)).0;

        let farm_id = self.internal_add_farm(&terms, min_deposit);

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
    fn internal_add_farm(&mut self, terms: &HRSimpleFarmTerms, min_deposit: Balance) -> FarmId {
        
        // let mut farm_seed = self.get_seed_default(&terms.seed_id, min_deposit);
        let mut farm_seed: VersionedFarmSeed;
        if let Some(fs) = self.get_seed_wrapped(&terms.seed_id) {
            farm_seed = fs;
            env::log(
                format!(
                    "New farm created In seed {}, with existed min_deposit {}",
                    terms.seed_id, farm_seed.get_ref().min_deposit
                )
                .as_bytes(),
            );
        } else {
            farm_seed = VersionedFarmSeed::new(&terms.seed_id, min_deposit);
            env::log(
                format!(
                    "The first farm created In seed {}, with min_deposit {}",
                    terms.seed_id, farm_seed.get_ref().min_deposit
                )
                .as_bytes(),
            );
        }

        let farm_id: FarmId = gen_farm_id(&terms.seed_id, farm_seed.get_ref().next_index as usize);

        let farm = Farm::SimpleFarm(SimpleFarm::new(
            farm_id.clone(),
            terms.into(),
        ));

        // farm_seed.get_ref_mut().farms.push(farm);
        farm_seed.get_ref_mut().farms.insert(farm_id.clone(), farm);
        farm_seed.get_ref_mut().next_index += 1;
        self.data_mut().seeds.insert(&terms.seed_id, &farm_seed);

        self.data_mut().farm_count += 1;
        farm_id
    }

    /// when farm's status is Ended, and unclaimed reward is 0, 
    /// the farm can be remove to reduce storage usage
    pub(crate) fn internal_remove_farm(&mut self, seed_id: &SeedId) {

        let mut farm_seed = self.get_seed(&seed_id);
        let mut removable_farms: Vec<String> = vec![];
        for farm in farm_seed.get_ref().farms.values() {
            if farm.can_be_removed() {
                removable_farms.push(farm.get_farm_id());
            }
        }
        for farm_id in &removable_farms {
            farm_seed.get_ref_mut().farms.remove(farm_id);
        }
        if removable_farms.len() > 0 {
            self.data_mut().seeds.insert(&seed_id, &farm_seed);
        }
        
    }
}
