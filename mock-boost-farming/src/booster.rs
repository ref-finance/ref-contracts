use crate::*;

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct BoosterInfo {
    pub booster_decimal: u32,
    /// <affected_seed_id, log_base>
    pub affected_seeds: HashMap<SeedId, u32>,
}

impl BoosterInfo {
    pub fn assert_valid(&self, booster_id: &SeedId) {
        assert!(self.affected_seeds.contains_key(booster_id) == false, "{}", E202_FORBID_SELF_BOOST);
        assert!(self.affected_seeds.len() <= MAX_NUM_SEEDS_PER_BOOSTER, "{}", E204_EXCEED_SEED_NUM_IN_BOOSTER);
    }
}

#[near_bindgen]
impl Contract {
    #[payable]
    pub fn modify_booster(&mut self, booster_id: SeedId, booster_info: BoosterInfo) {
        assert_one_yocto();
        assert!(self.is_owner_or_operators(), "{}", E002_NOT_ALLOWED);
        assert!(self.data().state == RunningState::Running, "{}", E004_CONTRACT_PAUSED);
        assert!(self.internal_get_seed(&booster_id).is_some(), "{}", E301_SEED_NOT_EXIST);
        booster_info.assert_valid(&booster_id);

        let mut config =  self.data().config.get().unwrap();
        assert!(self.affected_farm_count(&booster_info) <= config.max_num_farms_per_booster, "{}", E203_EXCEED_FARM_NUM_IN_BOOST);
        
        config.booster_seeds.insert(booster_id.clone(), booster_info);
        self.data_mut().config.set(&config);
    }
}

impl Contract {

    fn affected_farm_count(&self, booster_info: &BoosterInfo) -> u32 {
        booster_info.affected_seeds
        .keys()
        .map(|seed_id| self.data().seeds.get(seed_id).expect(E301_SEED_NOT_EXIST))
        .map(|v| {
            let seed: Seed = v.into();
            seed.farms.len() as u32
        })
        .sum::<u32>()
    }

    pub fn assert_booster_affected_farm_num(&self) {
        let config = self.internal_config();
        for booster_info in config.booster_seeds.values() {
            assert!(self.affected_farm_count(booster_info) <= config.max_num_farms_per_booster, "{}", E203_EXCEED_FARM_NUM_IN_BOOST);
        }
    }

    /// generate booster ratios map for a given seed
    /// booster-ratio = ((booster_balance as f64) / (booster_base as f64)).log(log_base as f64)
    /// where log_base if from Config.global_booster_seeds.get(seed_id).unwrap().get(self.seed_id).unwrap()
    pub fn gen_booster_ratios(&self, seed_id: &SeedId, farmer: &Farmer) -> HashMap<SeedId, f64> {
        let mut ratios = HashMap::new();
        let log_bases = self.internal_config().get_boosters_from_seed(seed_id);
        for (booster, booster_decimal, log_base) in &log_bases {
            let booster_balance = farmer
                .seeds
                .get(booster)
                .map(|v| v.get_basic_seed_power())
                .unwrap_or(0_u128);
            if booster_balance > 0 && log_base > &0 {
                let booster_base = 10u128.pow(*booster_decimal);
                let booster_amount = booster_balance as f64 / booster_base as f64;
                let ratio = if booster_amount > 1f64 {
                    booster_amount.log(*log_base as f64)
                } else {
                    0f64
                };
                ratios.insert(booster.clone(), ratio);
            }
        }
        ratios
    }

    /// if seed_id is a booster, then update all impacted seed
    pub fn update_impacted_seeds(&mut self, farmer: &mut Farmer, booster_id: &SeedId) {
        if let Some(booster_info) = self.internal_config().get_affected_seeds_from_booster(booster_id) {
            for seed_id in booster_info.affected_seeds.keys() {
                // here we got each affected seed_id, then if the farmer has those seeds, should be updated on by one
                if farmer.seeds.get(seed_id).is_some() {
                    // first claim that farmer's current reward and update boost_ratios for the seed
                    let mut seed = self.internal_unwrap_seed(seed_id);
                    self.internal_do_farmer_claim(farmer, &mut seed);
                    self.internal_set_seed(seed_id, seed);
                }
            }
        }
    }

}