# ref-farming

## Terminology

|word|meaning|notes|
|-|-|-|
|Seed|Farming-Token|User stakes seed to this contract for various rewards token back|
|SeedId|String|Token contract_id for ft token, token contract_id + "@" + inner_id for mft token|
|FarmId|String|SeedId + "#" + farm_index in that seed|
|RPS|Reward-Per-Seed|The key concept to distribute rewards between farmers in a farm|
|RR|Reward Round in block num|the reward are released by round|

## Logic

### Core concept

**Farmer** deposit/stake **Seed** token to farming on all **farms** that accept that seed, 
and gains **reward** token back. different farm can (not must) grows different reward token.

### contract structure

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
### Reward distribution in simple farm
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

### When to update the farm

It's worth to noticed, 
each time a farmer deposit seed, withdraw seed, claim reward,  
All relevant farm woulds be invoke with distribute to update themselves, and  
furthermore, in deposit and withdraw seed actions, the farmer's claim_reward action would be automatically invoked to keep this RPS logic correctly.

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

