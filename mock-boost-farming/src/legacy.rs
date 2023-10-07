use crate::*;

#[derive(BorshDeserialize, BorshSerialize)]
pub struct ContractDataV0100 {
    pub owner_id: AccountId,
    pub operators: UnorderedSet<AccountId>,
    pub config: LazyOption<Config>,
    pub seeds: UnorderedMap<SeedId, VSeed>,
    pub farmers: LookupMap<AccountId, VFarmer>,
    pub outdated_farms: UnorderedMap<FarmId, VSeedFarm>,
    // all slashed seed would recorded in here
    pub seeds_slashed: UnorderedMap<SeedId, Balance>,
    // if unstake seed encounter error, the seed would go to here
    pub seeds_lostfound: UnorderedMap<SeedId, Balance>,

    // for statistic
    farmer_count: u64,
    farm_count: u64,
}

impl From<ContractDataV0100> for ContractData {
    fn from(a: ContractDataV0100) -> Self {
        let ContractDataV0100 {
            owner_id,
            operators,
            config,
            seeds,
            farmers,
            outdated_farms,
            seeds_slashed,
            seeds_lostfound,
            farmer_count,
            farm_count,
        } = a;
        Self {
            owner_id,
            operators,
            config,
            seeds,
            farmers,
            outdated_farms,
            seeds_slashed,
            seeds_lostfound,
            farmer_count,
            farm_count,
            state: RunningState::Running,
        }
    }
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct FarmerV0 {
    /// A copy of an farmer ID. Saves one storage_read when iterating on farmers.
    pub farmer_id: AccountId,
    /// Amounts of various reward tokens the farmer claimed.
    pub rewards: HashMap<AccountId, Balance>,
    /// Various seed tokens the farmer staked.
    pub seeds: UnorderedMap<SeedId, FarmerSeed>,
}

impl From<FarmerV0> for Farmer {
    fn from(a: FarmerV0) -> Self {
        let FarmerV0 {
            farmer_id,
            rewards,
            seeds,
        } = a;
        Self {
            farmer_id: farmer_id.clone(),
            sponsor_id: farmer_id.clone(),
            rewards,
            seeds,
        }
    }
}