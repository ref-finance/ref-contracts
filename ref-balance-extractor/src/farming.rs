use crate::*;
use near_sdk::BlockHeight;

pub(crate) type SeedId = String;
pub(crate) type FarmId = String;
pub type RPS = [u8; 32];

#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub enum SeedType {
    FT,
    MFT,
}

#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub struct SimpleFarmTerms {
    pub seed_id: SeedId,
    pub reward_token: AccountId,
    pub start_at: BlockHeight,
    pub reward_per_session: Balance,
    pub session_interval: BlockHeight,
}

#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub enum SimpleFarmStatus {
    Created,
    Running,
    Ended,
    Cleared,
}

/// Reward Distribution Record
#[derive(BorshSerialize, BorshDeserialize, Clone, Default)]
pub struct SimpleFarmRewardDistribution {
    /// unreleased reward
    pub undistributed: Balance,
    /// the total rewards distributed but not yet claimed by farmers.
    pub unclaimed: Balance,
    /// Reward_Per_Seed
    /// rps(cur) = rps(prev) + distributing_reward / total_seed_staked
    pub rps: RPS,
    /// Reward_Round
    /// rr = (cur_block_height - start_at) / session_interval
    pub rr: u64,
}

#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub struct SimpleFarm {
    pub farm_id: FarmId,

    pub terms: SimpleFarmTerms,

    pub status: SimpleFarmStatus,

    pub last_distribution: SimpleFarmRewardDistribution,

    /// total reward send into this farm by far,
    /// every time reward deposited in, add to this field
    pub amount_of_reward: Balance,
    /// reward token has been claimed by farmer by far
    pub amount_of_claimed: Balance,
    /// when there is no seed token staked, reward goes to beneficiary
    pub amount_of_beneficiary: Balance,
}

#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub enum Farm {
    SimpleFarm(SimpleFarm),
}

/// Account deposits information and storage cost.
#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub struct Farmer {
    /// Native NEAR amount sent to this contract.
    /// Used for storage.
    pub amount: Balance,
    /// Amounts of various reward tokens the farmer claimed.
    pub rewards: HashMap<AccountId, Balance>,
    /// Amounts of various seed tokens the farmer staked.
    pub seeds: HashMap<SeedId, Balance>,
    /// record user_last_rps of farms
    pub user_rps: HashMap<FarmId, RPS>,
}

#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub struct FarmSeed {
    /// The Farming Token this FarmSeed represented for
    pub seed_id: SeedId,
    /// The seed is a FT or MFT, enum size is 2 bytes?
    pub seed_type: SeedType,
    /// all farms that accepted this seed
    /// FarmId = {seed_id}#{next_index}
    pub farms: HashMap<FarmId, Farm>,
    pub next_index: u32,
    /// total (staked) balance of this seed (Farming Token)
    pub amount: Balance,
    pub min_deposit: Balance,
}

#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub enum VersionedFarmSeed {
    V101(FarmSeed),
}

impl VersionedFarmSeed {}

#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub enum VersionedFarmer {
    V101(Farmer),
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct RefFarmingContractData {
    // owner of this contract
    pub owner_id: AccountId,

    // record seeds and the farms under it.
    // seeds: UnorderedMap<SeedId, FarmSeed>,
    pub seeds: UnorderedMap<SeedId, VersionedFarmSeed>,

    // each farmer has a structure to describe
    // farmers: LookupMap<AccountId, Farmer>,
    pub farmers: LookupMap<AccountId, VersionedFarmer>,

    // for statistic
    pub farmer_count: u64,
    pub farm_count: u64,
    pub reward_info: UnorderedMap<AccountId, Balance>,
}

/// Versioned contract data. Allows to easily upgrade contracts.
#[derive(BorshSerialize, BorshDeserialize)]
pub enum VersionedContractData {
    Current(RefFarmingContractData),
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct RefFarmingContract {
    pub data: VersionedContractData,
}

impl RefFarmingContract {
    pub fn parse(&mut self, state: &mut State) {
        match &mut self.data {
            VersionedContractData::Current(data) => {
                data.seeds.parse(state);
                data.farmers.parse(state);

                data.reward_info.parse(state);
            }
        }
    }
}

pub const MFT_TAG: &str = "@";

// return receiver_id, token_id
pub fn parse_seed_id(lpt_id: &str) -> (String, String) {
    let v: Vec<&str> = lpt_id.split(MFT_TAG).collect();
    if v.len() == 2 {
        // receiver_id@pool_id
        (v[0].to_string(), v[1].to_string())
    } else if v.len() == 1 {
        // receiver_id
        (v[0].to_string(), v[0].to_string())
    } else {
        unreachable!();
    }
}
