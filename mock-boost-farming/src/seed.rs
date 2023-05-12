use crate::*;

static SEEDS: Lazy<Mutex<HashMap<SeedId, Option<Seed>>>> = Lazy::new(|| Mutex::new(HashMap::new()));

#[derive(BorshSerialize, BorshDeserialize, Clone, Serialize, Deserialize, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct Seed {
    /// The Farming Token this FarmSeed represented for
    pub seed_id: SeedId,
    pub seed_decimal: u32,
    /// FarmId = {seed_id}#{next_index}
    #[serde(skip_serializing)]
    pub farms: HashMap<FarmId, VSeedFarm>,
    pub next_index: u32,
    /// total (staked) balance of this seed (Farming Token)
    #[serde(with = "u128_dec_format")]
    pub total_seed_amount: Balance,
    #[serde(with = "u128_dec_format")]
    pub total_seed_power: Balance,
    #[serde(with = "u128_dec_format")]
    pub min_deposit: Balance,
    /// the CD Account slash rate for this seed
    pub slash_rate: u32,
    /// if min_lock_duration == 0, means forbid locking
    pub min_locking_duration_sec: DurationSec,
}

#[derive(BorshSerialize, BorshDeserialize)]
pub enum VSeed {
    Current(Seed),
}

impl From<VSeed> for Seed {
    fn from(v: VSeed) -> Self {
        match v {
            VSeed::Current(c) => c,
        }
    }
}

impl From<Seed> for VSeed {
    fn from(c: Seed) -> Self {
        VSeed::Current(c)
    }
}

impl Seed {
    #[allow(unreachable_patterns)]
    pub fn update(&mut self) {
        for (_, vfarm) in self.farms.iter_mut() {
            match vfarm {
                VSeedFarm::Current(farm) => {
                    farm.update(self.total_seed_power);
                }
                _ => {}
            }
        }
    }

    pub fn update_claimed(&mut self, claimed: &HashMap<FarmId, Balance>) {
        for (farm_id, amount) in claimed {
            let VSeedFarm::Current(seed_farm) = self.farms.get_mut(farm_id).unwrap();
            seed_farm.claimed_reward += amount;
        }
    }

    pub fn new(
        seed_id: &SeedId,
        seed_decimal: u32,
        min_deposit: Balance,
        default_slash_rate: u32,
        min_locking_duration_sec: DurationSec,
    ) -> Self {
        Self {
            seed_id: seed_id.clone(),
            seed_decimal,
            farms: HashMap::new(),
            next_index: 0,
            total_seed_amount: 0,
            total_seed_power: 0,
            min_deposit,
            slash_rate: default_slash_rate,
            min_locking_duration_sec,
        }
    }
}

impl Contract {

    pub fn internal_unwrap_seed(&self, seed_id: &SeedId) -> Seed {
        self.internal_get_seed(seed_id).expect(E301_SEED_NOT_EXIST)
    }

    pub fn internal_get_seed(&self, seed_id: &SeedId) -> Option<Seed> {
        let mut cache = SEEDS.lock().unwrap();
        cache.get(seed_id).cloned().unwrap_or_else(|| {
            let seed = self.data().seeds.get(seed_id).map(|v| {
                let mut seed: Seed = v.into();
                seed.update();
                seed
            });
            cache.insert(seed_id.clone(), seed.clone());
            seed
        })
    }

    pub fn internal_set_seed(&mut self, seed_id: &SeedId, seed: Seed) {
        SEEDS
            .lock()
            .unwrap()
            .insert(seed_id.clone(), Some(seed.clone()));
        self.data_mut().seeds.insert(seed_id, &seed.into());
    }
}
