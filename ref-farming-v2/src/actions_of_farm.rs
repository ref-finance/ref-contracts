
use near_sdk::{env, near_bindgen};
use near_sdk::json_types::{U128};
use simple_farm::{SimpleFarm, HRSimpleFarmTerms};
use crate::utils::{gen_farm_id, MIN_SEED_DEPOSIT, MAX_FARM_NUM, parse_farm_id};
use crate::errors::*;
use crate::*;


#[near_bindgen]
impl Contract {
    /// create farm and pay for its storage fee
    #[payable]
    pub fn create_simple_farm(&mut self, terms: HRSimpleFarmTerms, min_deposit: Option<U128>) -> FarmId {

        self.assert_owner();
        
        let min_deposit: u128 = min_deposit.unwrap_or(U128(MIN_SEED_DEPOSIT)).0;

        let farm_id = self.internal_add_farm(&terms, min_deposit);

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

        assert!(farm_seed.get_ref().farms.len() < MAX_FARM_NUM, "{}", ERR36_FARMS_NUM_HAS_REACHED_LIMIT);

        let farm_id: FarmId = gen_farm_id(&terms.seed_id, farm_seed.get_ref().next_index as usize);

        let farm = Farm::SimpleFarm(SimpleFarm::new(
            farm_id.clone(),
            terms.into(),
        ));
        
        farm_seed.get_ref_mut().farms.insert(farm_id.clone());
        farm_seed.get_ref_mut().next_index += 1;
        self.data_mut().seeds.insert(&terms.seed_id, &farm_seed);
        self.data_mut().farms.insert(&farm_id.clone(), &farm);
        farm_id
    }

    pub(crate) fn internal_remove_farm_by_farm_id(&mut self, farm_id: &FarmId) -> bool {
        let (seed_id, _) = parse_farm_id(farm_id);
        let mut removable = false;
        if let Some(mut farm_seed) = self.get_seed_wrapped(&seed_id) {
            // let seed_amount = farm_seed.get_ref().total_seed_amount; // TODO power
            let seed_amount = farm_seed.get_ref().total_seed_power;
            if let Some(farm) = self.data().farms.get(farm_id) {
                if farm.can_be_removed(&seed_amount) {
                    removable = true;
                }
            }
            if removable {
                let mut farm = self.data_mut().farms.remove(farm_id).expect(ERR41_FARM_NOT_EXIST);
                farm.move_to_clear(&seed_amount);
                self.data_mut().outdated_farms.insert(farm_id, &farm);
                farm_seed.get_ref_mut().farms.remove(farm_id);
                self.data_mut().seeds.insert(&seed_id, &farm_seed);
                return true;
            }
        }
        false
    }
}
