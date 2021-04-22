# ref-farming

### Terminology

|word|meaning|notes|
|-|-|-|
|Seed|Farming-Token|User stakes seed to this contract for various rewards token back|


### Logic

**Farmer** deposit/stake **Seed** token to farming on all **farms** that accept that seed, 
and gains **reward** token back. different farm can grows different reward token.

```rust
pub struct Contract {
    /// owner of this contract
    owner_id: AccountId,
    
    /// seed entry, farms using same seed 
    /// are managed under this structure
    seeds: UnorderedMap::<SeedId, FarmSeed>,

    /// farmer entry, farmer's seed and rewards 
    /// are managed under this structure
    farmers: LookupMap<AccountId, Farmer>,
}

pub struct Farmer {
    /// Native NEAR amount sent to this contract.
    /// Used for storage.
    pub amount: Balance,
    /// Amounts of various reward tokens the farmer get.
    pub rewards: HashMap<AccountId, Balance>,
    /// Amounts of various seed tokens the farmer staked.
    pub seeds: HashMap<SeedId, Balance>,
    /// record user_last_rps of farms
    pub farm_rps: HashMap<FarmId, Balance>,
}

pub struct FarmSeed {
    /// The seed this FarmSeed represented for
    pub seed_id: SeedId,
    /// seed token is FT or MFT standard
    pub seed_type: SeedType,
    /// all farms that accepted this seed
    pub xfarms: Vec<Farm>,
    /// total balance of this seed as total stake amount for each farm in xfarms
    pub amount: Balance,
}
```

***reward distribution logic***  
```rust

pub struct SimpleFarmTerms {
    pub seed_id: SeedId,
    pub reward_token: AccountId,
    pub start_at: BlockHeight,
    pub reward_per_session: Balance,
    pub session_interval: BlockHeight,
}

pub struct SimpleFarm {
    pub farm_id: FarmId,
    pub terms: SimpleFarmTerms,

    /// total reward send into this farm by far, 
    /// every time reward deposited in, sum up to this field
    pub amount_of_reward: Balance,
    /// reward token has been claimed by farmer by far
    pub amount_of_claimed: Balance,
        
    //*******************************************
    //*** RECORDS OF PREV REWARD DISTRIBUTION ***
    //*******************************************
    /// unreleased reward 
    pub amount_of_undistributed_reward: Balance,
    /// the total rewards distributed but not yet claimed by farmers.
    pub unclaimed_reward: Balance,
    /// RPS = Reward Per Seed, at each distribution
    /// rps += distributing_reward / total_seed_staked
    pub last_rps: Balance,
    /// reward_round = (cur_block_height - start_at) / session_interval
    pub last_reward_round: u64,
}
``` 
As designed that way, we can caculate farmers unclaimed reward like this:  

```rust
// 1. get current reward round CRR
let crr = (env::block_index() - self.terms.start_at) / self.terms.session_interval;
// 2. get reward to distribute this time
let reward_added = (crr - self.last_reward_round) as u128 * self.terms.reward_per_session;
// 3. get current RPS
let crps = self.last_rps + reward_added / total_seeds;
// 4. get user unclaimed by multiple user_staked_seed with rps diff.
let unclaimed_reward = user_staked_seed * (crps - user_last_rps);
```

# Things need to discuss

* Stake seed  
When farm stake seed, the fee attached by FT/MFT standard through ft_on_transfer/mft_on_transfer may insufficient for staking action.  
Do we need to split stake into two actions [deposit_seed, do_stake] for security?
* Storage fee management    
For complicity of storage usage, now we pre-charge storage fee, and forbidden farmers action when storage fee is beyond user deposited.  
* Farm management  
Now, we can only add Farm, and farm are mangaged under seed, which cause a little difficult to get all farms in one shot.  
My defense is that the requirement is to list farms under given seeds, not list all farms.  




### Interface Structure

```rust
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

### Interface

***view functions***  
```rust
/// number of farms.
pub fn get_number_of_farms(&self) -> u64;
/// farm status
pub fn list_farms(&self, from_index: u64, limit: u64) -> Vec<FarmInfo>;
pub fn list_farms_by_seed(&self, seed_id: SeedId) -> Vec<FarmInfo>;
pub fn get_farm(&self, farm_id: FarmId) -> Option<FarmInfo>;

/// claimed rewards of given user
pub fn list_rewards(&self, account_id: ValidAccountId) -> HashMap<AccountId, U128>;
/// claimed reward of given user and given reward token.
pub fn get_reward(&self, account_id: ValidAccountId, token_id: ValidAccountId) -> U128;
/// unclaimed reward of given user and given farm
pub fn get_unclaimed_reward(&self, account_id: ValidAccountId, farm_id: FarmId) -> U128;

/// all staked seeds
pub fn list_seeds(&self, from_index: u64, limit: u64) -> HashMap<SeedId, U128>;
/// all staked seeds of given user
pub fn list_user_seeds(&self, account_id: ValidAccountId) -> HashMap<SeedId, U128>;
```

***Storage functions***  
```rust

/// total can bigger than available, which means farmer owes storage fee, 
/// and before he storage_deposit more fee, all changeable method invoke 
/// would fail with ERR11_INSUFFICIENT_STORAGE
pub struct StorageBalance {
    pub total: U128, // here we redefine total to locked amount for storage fee.
    pub available: U128,  // here we redefine it to the user deposited.
}

/// Only farmer need to register for storage, 
/// the attatched should more than a suggested minimum storage fee, 
/// which contains 5 seeds, 5 rewards and 10 farms, 
/// registration_only true means to refund exceed amount back to user. 
/// Farmer also use this method to add storage fee, with registration_only set to false.
#[payable]
fn storage_deposit(&mut self, account_id: 
    Option<ValidAccountId>, 
    registration_only: Option<bool>,
) -> StorageBalance;

/// can withdraw unlocked amount of storage fee
#[payable]
fn storage_withdraw(&mut self, amount: Option<U128>) -> StorageBalance;

/// to completely quit from this contract, should remove all seeds and rewards first
fn storage_unregister(&mut self, force: Option<bool>) -> bool;

/// get current storage fee info
fn storage_balance_of(&self, account_id: ValidAccountId) -> Option<StorageBalance>;
```

***Manage farms***  
```rust
/// FarmId is like this:
let farm_id: FarmId = format!("{}#{}", seed_id, index);
/// creat farm and pay for its storage fee
#[payable]
pub fn create_simple_farm(&mut self, terms: HRSimpleFarmTerms) -> FarmId;
```

***Manage seeds***  
```rust
/// SeedId is like this:
/// receiver_id@pool_id for MFT
/// receiver_id for FT

/// stake is through MFT's mft_on_transfer or FT's ft_on_transfer, 
/// with msg field left to empty string.

/// unstake
#[payable]
pub fn withdraw_seed(&mut self, seed_id: SeedId, amount: U128);
```

***Manage rewards***  
```rust
/// standard claim reward from given farm
#[payable]
pub fn claim_reward_by_farm(&mut self, farm_id: FarmId);

/// batch claim from farms with same seeds
#[payable]
pub fn claim_reward_by_seed(&mut self, seed_id: SeedId);

/// Withdraws given reward token of given user.
#[payable]
pub fn withdraw_reward(&mut self, token_id: ValidAccountId, amount: Option<U128>);
```
