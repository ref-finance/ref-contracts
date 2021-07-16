# ref-farming

## Interface Structure

```rust
/// metadata and the  whole statistics of the contract
pub struct Metadata {
    pub version: String,
    pub owner_id: AccountId,
    pub farmer_count: U64,
    pub farm_count: U64,
    pub seed_count: U64,
    pub reward_count: U64,
}

/// seed info
pub struct SeedInfo {
    pub seed_id: SeedId,
    pub seed_type: String, // FT, MFT
    pub farms: Vec<FarmId>,
    pub next_index: u32,
    pub amount: U128,
    pub min_deposit: U128,
}

/// used to create a farm
pub struct HRSimpleFarmTerms {
    pub seed_id: SeedId,
    pub reward_token: ValidAccountId,
    pub start_at: U64,
    pub reward_per_session: U128,
    pub session_interval: U64, 
}

/// Farm Status
pub struct FarmInfo {
    pub farm_id: FarmId,
    pub farm_kind: String,
    pub farm_status: String,  // Created, Running, Ended
    pub seed_id: SeedId,
    pub reward_token: AccountId,
    pub start_at: U64,
    pub reward_per_session: U128,
    pub session_interval: U64, 
    // total_reward = distributed + undistributed
    // distributed = claimed + unclaimed
    pub total_reward: U128,
    pub cur_round: U64,
    pub last_round: U64,
    pub claimed_reward: U128,
    pub unclaimed_reward: U128,
}

```

## Interface methods

***view functions***  
```rust

/// whole contract
pub fn get_metadata(&self) -> Metadata;

//***********************************
//************* about Farms *********
//***********************************

/// total number of farms.
pub fn get_number_of_farms(&self) -> u64;

/// batch get farm info by seed;
/// Cause farms are organized under Seed(ie. Farming-Token) in the contract
pub fn list_farms_by_seed(&self, seed_id: SeedId) -> Vec<FarmInfo>;

/// Get single farm's status
pub fn get_farm(&self, farm_id: FarmId) -> Option<FarmInfo>;

//***********************************
//*********** about Rewards *********
//***********************************

/// get all rewards and its supply
pub fn list_rewards_info(&self, from_index: u64, limit: u64) -> HashMap<AccountId, U128>;

/// claimed rewards of given user
pub fn list_rewards(&self, account_id: ValidAccountId) -> HashMap<AccountId, U128>;

/// claimed reward of given user and given reward token.
pub fn get_reward(&self, account_id: ValidAccountId, token_id: ValidAccountId) -> U128;

/// unclaimed reward of given user and given farm
pub fn get_unclaimed_reward(&self, account_id: ValidAccountId, farm_id: FarmId) -> U128;

//***********************************
//*********** about Seeds ***********
//***********************************

/// all staked seeds and its info
pub fn get_seed_info(&self, seed_id: SeedId) -> Option<SeedInfo>;

/// all staked seeds of given user
pub fn list_seeds_info(&self, from_index: u64, limit: u64) -> HashMap<SeedId, SeedInfo>;

```

***Storage functions***  
User of farming contract should register first and keep their storage fee valid.  
```rust

/// total can bigger than available, which means farmer owes storage fee, 
/// and before he storage_deposit more fee, all changeable method invoke 
/// would fail with ERR11_INSUFFICIENT_STORAGE
pub struct StorageBalance {
    pub total: U128, // here we redefine total to locked amount for storage fee.
    pub available: U128,  // here we redefine it to the user pre-deposited to cover the fee.
}

/// Only farmer need to register for storage, 
/// the attached should more than a suggested minimum storage fee, 
/// which can cover storage fee for 5 seeds, 5 rewards and 10 farms, 
/// registration_only true means to refund exceeded amount back to user. 
/// Farmer also use this method to add storage fee, with registration_only set to false.
#[payable]
fn storage_deposit(&mut self, account_id: 
    Option<ValidAccountId>, 
    registration_only: Option<bool>,
) -> StorageBalance;

/// Withdraw unlocked amount of storage fee
#[payable]
fn storage_withdraw(&mut self, amount: Option<U128>) -> StorageBalance;

/// to completely quit from this contract, 
/// should unstake all seeds and withdraw all rewards before call this one
fn storage_unregister(&mut self, force: Option<bool>) -> bool;

/// get current storage fee info
fn storage_balance_of(&self, account_id: ValidAccountId) -> Option<StorageBalance>;
```

***Manage farms***  
```rust
/// FarmId is like this:
let farm_id: FarmId = format!("{}#{}", seed_id, index);

/// create farm and pay for its storage fee
/// terms defines farm rules in type of HRSimpleFarmTerms,
/// min_deposit will set the minimum stake balance of seed token 
/// if this farm is the first farm in that seed, and 
/// if None is given, the default MIN_SEED_DEPOSIT will be used, 
/// that is 10**24.
#[payable]
pub fn create_simple_farm(&mut self, terms: HRSimpleFarmTerms, min_deposit: Option<U128>) -> FarmId;
```

***Manage seeds***  
```rust
/// SeedId is like this:
/// receiver_id@pool_id for MFT
/// receiver_id for FT

/// stake action is invoked outside this contract, 
/// actually by MFT's mft_on_transfer or FT's ft_on_transfer, 
/// with msg field left to empty string.

/// unstake, with amount is 0, means to unstake all.
#[payable]
pub fn withdraw_seed(&mut self, seed_id: SeedId, amount: U128);
```

***Manage rewards***  
```rust
/// claim reward from single farm
#[payable]
pub fn claim_reward_by_farm(&mut self, farm_id: FarmId);

/// batch claim from farms with same seeds
#[payable]
pub fn claim_reward_by_seed(&mut self, seed_id: SeedId);

/// All claimed rewards goes to farmer's inner account in this contract,
/// So, farmer can withdraw given reward token back to his own account.
#[payable]
pub fn withdraw_reward(&mut self, token_id: ValidAccountId, amount: Option<U128>);
```

***Owner methods***  
```rust
pub fn set_owner(&mut self, owner_id: ValidAccountId);

/// those farm with Ended status and zero unclaimed reward, 
/// can be cleaned to save storage.
pub fn clean_farm_by_seed(&mut self, seed_id: String);

/// owner can modify min_deposit of given seed.
pub fn modify_seed_min_deposit(&mut self, seed_id: String, min_deposit: Balance);

/// upgrade the contract
pub fn upgrade(
        &self,
        #[serializer(borsh)] code: Vec<u8>,
        #[serializer(borsh)] migrate: bool,
    ) -> Promise;
```

## contract core structure

```rust
pub struct Contract {

    // owner of this contract
    owner_id: AccountId,
    
    // record seeds and the farms under it.
    seeds: UnorderedMap::<SeedId, FarmSeed>,

    // each farmer has a structure to describe
    farmers: LookupMap<AccountId, Farmer>,

    // for statistic
    farmer_count: u64,
    farm_count: u64,
    reward_info: UnorderedMap::<AccountId, Balance>,
}

/// used to store U256 in contract storage
pub type RPS = [u8; 32];

pub struct Farmer {
    /// Native NEAR amount sent to this contract.
    /// Used for storage.
    pub amount: Balance,
    /// Amounts of various reward tokens the farmer claimed.
    pub rewards: HashMap<AccountId, Balance>,
    /// Amounts of various seed tokens the farmer staked.
    pub seeds: HashMap<SeedId, Balance>,
    /// record user_last_rps of farms
    pub farm_rps: HashMap<FarmId, RPS>,
}

pub struct FarmSeed {
    /// The Farming Token this FarmSeed represented for
    pub seed_id: SeedId,
    /// The seed is a FT or MFT
    pub seed_type: SeedType,
    /// all farms that accepted this seed
    /// may change to HashMap<GlobalIndex, Farm> 
    /// to enable whole life-circle (especially for removing of farm). 
    pub farms: Vec<Farm>,
    /// total (staked) balance of this seed (Farming Token)
    pub amount: Balance,
}
```
### Reward distribution implementation
Each simple farm has a terms `SimpleFarmTerms` to define how to distribute reward,  
And a Status `SimpleFarmStatus` to mark the life-circle,  
And the key last-distribution record - `SimpleFarmRewardDistribution`.  
```rust
pub struct SimpleFarmTerms {
    pub seed_id: SeedId,
    pub reward_token: AccountId,
    pub start_at: BlockHeight,
    pub reward_per_session: Balance,
    pub session_interval: BlockHeight,
}

pub enum SimpleFarmStatus {
    Created, Running, Ended, Cleared
}

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
```
Then, the whole farm is built as
```rust
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

}
``` 

As designed that way, we can calculate farmers unclaimed reward like this:  

```rust
// 1. get current reward round CRR
let crr = (env::block_index() - self.terms.start_at) / self.terms.session_interval;
// 2. get reward to distribute this time
let reward_added = (crr - self.last_distribution.rr) as u128 * self.terms.reward_per_session;
// 3. get current RPS
let crps = self.last_distribution.rps + reward_added / total_seeds;
// 4. get user unclaimed by multiple user_staked_seed with rps diff.
let unclaimed_reward = user_staked_seed * (crps - user_last_rps);
```
This logic is sealed in 
```rust
pub(crate) fn view_farmer_unclaimed_reward(
        &self,
        user_rps: &RPS,
        user_seeds: &Balance,
        total_seeds: &Balance,
    ) -> Balance
```
which, based on 
```rust
pub(crate) fn try_distribute(&self, total_seeds: &Balance) -> Option<SimpleFarmRewardDistribution>
```
to calculate cur RPS and RR of the farm without modifying the storage (means not really update the farm)

And when farmer actually claims his reward, the whole logic is sealed in 
```rust
pub(crate) fn claim_user_reward(
        &mut self, 
        user_rps: &RPS,
        user_seeds: &Balance, 
        total_seeds: &Balance
    ) -> Option<(Balance, Balance)>
```
which, based on 
```rust
pub(crate) fn distribute(&mut self, total_seeds: &Balance)
```
to calculate and update the farm.



# Things need to explain
## Storage fee in this contract
As each farmer would have a place to record his rps in each farm he involved, the storage belongs to a farmer may increase out of his notice.  

For example, when a new farm established and running, which accepts the farmer's seed that has been staked in the contract, then at the following action such as claim_reward, or deposit/withdraw seeds invoked by the farmer, his storage would expand to record the new rps related to that farm.  

Consider that, and also to improve farmer's user-experience, we have a `suggested_min_storage_usage()` which covers 5 seed, 5 reward and 10 farms as one shot. When farmer register for the first time, we will force him to deposit more or equal to that amount, which is about 1,688 bytes, 0.0134 near. 
```rust
const MAX_ACCOUNT_LENGTH: u128 = 64;
const MIN_FARMER_LENGTH: u128 = MAX_ACCOUNT_LENGTH + 16 + 4 * 3;
/// Returns minimal storage usage possible.
/// 5 reward tokens, 5 seed tokens, 10 farms as assumption.
pub(crate) fn suggested_min_storage_usage() -> Balance {
    (
        MIN_FARMER_LENGTH 
        + 2_u128 * 5_u128 * (MAX_ACCOUNT_LENGTH + 16)
        + 10_u128 * (MAX_ACCOUNT_LENGTH + 32)
    ) * env::storage_byte_cost()
}
```
And when a farmer owes storage fee, then before he storage_deposit more fee,  
all changeable method would fail with ERR11_INSUFFICIENT_STORAGE.

