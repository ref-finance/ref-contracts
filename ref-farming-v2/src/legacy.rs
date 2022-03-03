use crate::*;

#[derive(BorshDeserialize, BorshSerialize)]
pub struct PrevContractData {

    // owner of this contract
    pub owner_id: AccountId,
    
    // record seeds and the farms under it.
    // seeds: UnorderedMap<SeedId, FarmSeed>,
    pub seeds: UnorderedMap<SeedId, VersionedFarmSeed>,

    // all slashed seed would recorded in here
    pub seeds_slashed: UnorderedMap<SeedId, Balance>,

    // if unstake seed encounter error, the seed would go to here
    pub seeds_lostfound: UnorderedMap<SeedId, Balance>,

    // each farmer has a structure to describe
    // farmers: LookupMap<AccountId, Farmer>,
    pub farmers: LookupMap<AccountId, VersionedFarmer>,

    pub farms: UnorderedMap<FarmId, Farm>,
    pub outdated_farms: UnorderedMap<FarmId, Farm>,

    // for statistic
    pub farmer_count: u64,
    pub reward_info: UnorderedMap<AccountId, Balance>,

    // strategy for farmer CDAccount
    pub cd_strategy: CDStrategy,
}

/// Versioned contract data. Allows to easily upgrade contracts.
#[derive(BorshSerialize, BorshDeserialize)]
pub enum PrevVersionedContractData {
    Current(PrevContractData),
}

#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct PrevContract {

    pub data: PrevVersionedContractData,
}