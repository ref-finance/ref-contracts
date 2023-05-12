use crate::*;
use near_sdk::json_types::U64;

#[derive(Serialize)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(feature = "test", derive(Deserialize, Clone))]
pub struct Metadata {
    pub version: String,
    pub owner_id: AccountId,
    pub state: RunningState,
    pub operators: Vec<AccountId>,
    pub farmer_count: U64,
    pub farm_count: U64,
    pub outdated_farm_count: U64,
    pub seed_count: U64,
}

#[derive(Serialize)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(feature = "test", derive(Deserialize, Clone))]
pub struct StorageReport {
    pub storage: U64,
    pub locking_near: U128,
}

#[near_bindgen]
impl Contract {
    //******** Contract Concern */
    pub fn get_metadata(&self) -> Metadata {
        Metadata {
            version: env!("CARGO_PKG_VERSION").to_string(),
            owner_id: self.data().owner_id.clone(),
            state: self.data().state.clone(),
            operators: self.data().operators.to_vec(),
            farmer_count: self.data().farmer_count.into(),
            farm_count: self.data().farm_count.into(),
            outdated_farm_count: self.data().outdated_farms.len().into(),
            seed_count: self.data().seeds.len().into(),
        }
    }

    pub fn get_config(&self) -> Config {
        self.internal_config()
    }

    pub fn get_contract_storage_report(&self) -> StorageReport {
        let su = env::storage_usage();
        StorageReport {
            storage: U64(su),
            locking_near: U128(su as Balance * env::storage_byte_cost()),
        }
    }

    pub fn list_outdated_farms(
        &self,
        from_index: Option<u64>,
        limit: Option<u64>,
    ) -> Vec<SeedFarm> {
        let values = self.data().outdated_farms.values_as_vector();
        let from_index = from_index.unwrap_or(0);
        let limit = limit.unwrap_or(values.len());
        (from_index..std::cmp::min(values.len(), from_index + limit))
            .map(|index| values.get(index).unwrap().into())
            .collect()
    }

    pub fn get_outdated_farm(&self, farm_id: FarmId) -> Option<SeedFarm> {
        self.data().outdated_farms.get(&farm_id).map(|vf| {
            let VSeedFarm::Current(farm) = vf;
            farm
        })
    }

    /// return slashed seed and its amount in this contract in a hashmap
    pub fn list_slashed(&self, from_index: Option<u64>, limit: Option<u64>) -> HashMap<SeedId, U128> {

        let keys = self.data().seeds_slashed.keys_as_vector();

        let from_index = from_index.unwrap_or(0);
        let limit = limit.unwrap_or(keys.len());

        (from_index..std::cmp::min(from_index + limit, keys.len()))
            .map(|index| {
                (
                    keys.get(index).unwrap(),
                    self.data().seeds_slashed.get(&keys.get(index).unwrap()).unwrap().into()
                )
            })
            .collect()
    }

    /// return lostfound seed and its amount in this contract in a hashmap
    pub fn list_lostfound(&self, from_index: Option<u64>, limit: Option<u64>) -> HashMap<SeedId, U128> {
        let keys = self.data().seeds_lostfound.keys_as_vector();

        let from_index = from_index.unwrap_or(0);
        let limit = limit.unwrap_or(keys.len());

        (from_index..std::cmp::min(from_index + limit, keys.len()))
            .map(|index| {
                (
                    keys.get(index).unwrap(),
                    self.data().seeds_lostfound.get(&keys.get(index).unwrap()).unwrap().into()
                )
            })
            .collect()
    }

    //******** Seed Concern */
    pub fn list_seeds_info(&self, from_index: Option<u64>, limit: Option<u64>) -> Vec<Seed> {
        let values = self.data().seeds.values_as_vector();
        let from_index = from_index.unwrap_or(0);
        let limit = limit.unwrap_or(values.len());
        (from_index..std::cmp::min(values.len(), from_index + limit))
            .map(|index| values.get(index).unwrap().into())
            .collect()
    }

    pub fn get_seed(&self, seed_id: SeedId) -> Option<Seed> {
        self.data().seeds.get(&seed_id).map(|vs| {
            let VSeed::Current(seed) = vs;
            seed
        })
    }

    pub fn list_seed_farms(&self, seed_id: SeedId) -> Vec<SeedFarm> {
        let seed = self.internal_unwrap_seed(&seed_id);
        seed.farms
            .values()
            .map(|vf| {
                let VSeedFarm::Current(farm) = vf;
                farm.clone()
            })
            .collect()
    }

    pub fn get_farm(&self, farm_id: FarmId) -> Option<SeedFarm> {
        let (seed_id, _) = parse_farm_id(&farm_id);
        let seed = self.internal_unwrap_seed(&seed_id);
        seed.farms.get(&farm_id).map(|vf| {
            let VSeedFarm::Current(farm) = vf;
            farm.clone()
        })
    }

    //******** Farmer Concern */
    pub fn get_unclaimed_rewards(
        &self,
        farmer_id: AccountId,
        seed_id: SeedId,
    ) -> HashMap<AccountId, U128> {
        let farmer = self.internal_unwrap_farmer(&farmer_id);
        let seed = self.internal_unwrap_seed(&seed_id);
        let (_, rewards, _) = self.internal_calc_farmer_claim(&farmer, &seed);
        rewards
            .into_iter()
            .map(|(key, val)| (key, val.into()))
            .collect()
    }

    pub fn list_farmer_seeds(
        &self,
        farmer_id: AccountId,
        from_index: Option<u64>,
        limit: Option<u64>,
    ) -> HashMap<SeedId, FarmerSeed> {
        if let Some(farmer) = self.internal_get_farmer(&farmer_id) {
            let keys = farmer.seeds.keys_as_vector();
            let from_index = from_index.unwrap_or(0);
            let limit = limit.unwrap_or(keys.len() as u64);
            (from_index..std::cmp::min(keys.len() as u64, from_index + limit))
                .map(|idx| {
                    let key = keys.get(idx).unwrap();
                    (key.clone(), farmer.seeds.get(&key).unwrap())
                })
                .collect()
        } else {
            HashMap::new()
        }
    }

    pub fn get_farmer_seed(&self, farmer_id: AccountId, seed_id: SeedId) -> Option<FarmerSeed> {
        if let Some(farmer) = self.internal_get_farmer(&farmer_id) {
            farmer.seeds.get(&seed_id)
        } else {
            None
        }
    }

    /// Returns reward token claimed for given user outside of any farms.
    /// Returns empty list if no rewards claimed.
    pub fn list_farmer_rewards(&self, farmer_id: AccountId) -> HashMap<AccountId, U128> {
        if let Some(farmer) = self.internal_get_farmer(&farmer_id) {
            farmer.rewards.into_iter()
                .map(|(token_id, amount)| (token_id, U128(amount)))
                .collect()
        } else {
            HashMap::new()
        }
    }

    /// Returns balance of amount of given reward token that ready to withdraw.
    pub fn get_farmer_reward(&self, farmer_id: AccountId, token_id: AccountId) -> U128 {
        if let Some(farmer) = self.internal_get_farmer(&farmer_id) {
            farmer.rewards.get(&token_id)
                .map(|v| v.clone().into())
                .unwrap_or(U128(0))
        } else {
            U128(0)
        }
    }

    pub fn get_farmer_sponsor(&self, farmer_id: AccountId) -> Option<AccountId> {
        if let Some(farmer) = self.internal_get_farmer(&farmer_id) {
            Some(farmer.sponsor_id)
        } else {
            None
        }
    }
}
