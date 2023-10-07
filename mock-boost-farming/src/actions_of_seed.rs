use crate::*;

#[near_bindgen]
impl Contract {
    /// create seed
    #[payable]
    pub fn create_seed(&mut self, seed_id: SeedId, seed_decimal: u32, min_deposit: Option<U128>, min_locking_duration_sec: Option<u32>) {
        assert_one_yocto();
        assert!(self.is_owner_or_operators(), "{}", E002_NOT_ALLOWED);
        assert!(self.data().state == RunningState::Running, "{}", E004_CONTRACT_PAUSED);

        let default_slash_rate = self.internal_config().seed_slash_rate;
        let min_deposit = min_deposit.unwrap_or(U128(MIN_SEED_DEPOSIT));
        let min_locking_duration_sec = min_locking_duration_sec.unwrap_or(DEFAULT_SEED_MIN_LOCKING_DURATION_SEC);

        assert!(
            self.internal_get_seed(&seed_id).is_none(),
            "{}", E302_SEED_ALREADY_EXIST
        );

        self.internal_set_seed(&seed_id, Seed::new(&seed_id, seed_decimal, min_deposit.into(), default_slash_rate, min_locking_duration_sec));

        Event::SeedCreate {
            caller_id: &env::predecessor_account_id(),
            seed_id: &seed_id,
            min_deposit: &min_deposit,
            slash_rate: default_slash_rate,
            min_locking_duration: min_locking_duration_sec,
        }
        .emit();
    }

    /// create farm
    #[payable]
    pub fn create_farm(&mut self, seed_id: SeedId, terms: FarmTerms) -> FarmId {
        assert_one_yocto();
        assert!(self.is_owner_or_operators(), "{}", E002_NOT_ALLOWED);
        assert!(self.data().state == RunningState::Running, "{}", E004_CONTRACT_PAUSED);

        let farm_id = self.internal_add_farm(&seed_id, &terms);

        self.assert_booster_affected_farm_num();

        Event::FarmCreate {
            caller_id: &env::predecessor_account_id(),
            reward_token: &terms.reward_token,
            farm_id: &farm_id,
            daily_reward: &U128(terms.daily_reward),
            start_at: terms.start_at,
        }
        .emit();

        farm_id
    }

    /// cancel a farm before any reward deposited
    #[payable]
    pub fn cancel_farm(&mut self, farm_id: String) {
        assert_one_yocto();
        assert!(self.is_owner_or_operators(), "{}", E002_NOT_ALLOWED);
        assert!(self.data().state == RunningState::Running, "{}", E004_CONTRACT_PAUSED);

        self.internal_cancel_farm(&farm_id);

        Event::FarmCancel {
            caller_id: &env::predecessor_account_id(),
            farm_id: &farm_id,
        }
        .emit();
    }

    /// outdate a farm to make it offline from both farmer and reward provider
    #[payable]
    pub fn remove_farm_from_seed(&mut self, farm_id: String) {
        assert_one_yocto();
        self.assert_owner();
        assert!(self.data().state == RunningState::Running, "{}", E004_CONTRACT_PAUSED);

        let (seed_id, _) = parse_farm_id(&farm_id);
        let mut seed = self.internal_unwrap_seed(&seed_id);

        let VSeedFarm::Current(mut outdated_farm) = seed.farms.remove(&farm_id).expect(E401_FARM_NOT_EXIST);
        outdated_farm.finalize();

        self.data_mut().outdated_farms.insert(&farm_id, &outdated_farm.into());
        self.internal_set_seed(&seed_id, seed);
        self.data_mut().farm_count -= 1;
    }
}

impl Contract {

    fn internal_add_farm(&mut self, seed_id: &SeedId, terms: &FarmTerms) -> FarmId {
        if let Some(mut seed) = self.internal_get_seed(seed_id) {
            assert!(
                seed.farms.len() < self.internal_config().max_num_farms_per_seed as usize,
                "{}", E303_EXCEED_FARM_NUM_IN_SEED
            );

            let farm_id: FarmId = gen_farm_id(seed_id, seed.next_index as usize);
            seed.next_index += 1;
            let farm = SeedFarm::new(farm_id.clone(), &terms);
            seed
                .farms
                .insert(farm_id.clone(), VSeedFarm::Current(farm));

            self.internal_set_seed(seed_id, seed);
            self.data_mut().farm_count += 1;

            farm_id
        } else {
            panic!("{}", E301_SEED_NOT_EXIST);
        }
    }

    fn internal_cancel_farm(&mut self, farm_id: &FarmId) {

        let (seed_id, _) = parse_farm_id(farm_id);
        let mut seed = self.internal_get_seed(&seed_id).expect(E301_SEED_NOT_EXIST);                 
        let vfarm = seed.farms.remove(farm_id).expect(E401_FARM_NOT_EXIST);
        let farm: SeedFarm = vfarm.into();
        assert!(farm.total_reward == 0, "{}", E403_FARM_ALREADY_DEPOSIT_REWARD);
        self.internal_set_seed(&seed_id, seed);
        self.data_mut().farm_count -= 1;
        
    }

    pub fn internal_deposit_reward(&mut self, farm_id: &FarmId, reward_token: &AccountId, amount: Balance) -> (Balance, u32) {

        let (seed_id, _) = parse_farm_id(farm_id);
        let mut seed = self.internal_get_seed(&seed_id).expect(E301_SEED_NOT_EXIST);    

        let VSeedFarm::Current(farm) = seed.farms.get_mut(farm_id).expect(E401_FARM_NOT_EXIST);
        let ret = farm.add_reward(reward_token, amount);

        self.internal_set_seed(&seed_id, seed);

        ret
    }
}
