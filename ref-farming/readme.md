# ref-farming

### Terminology

|word|meaning|notes|
|-|-|-|
|Seed|Farming-Token|User stakes seed to this contract for various rewards token back|
|SeedId|String|Token contract_id for ft token, token contract_id + "@" + inner_id for mft token|
|FarmId|String|SeedId + "#" + farm_index in that seed|


### Interface Structure

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
    pub farm_status: String,
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

/// whole contract
pub fn get_metadata(&self) -> Metadata;

//***********************************
//************* about Farms *********
//***********************************

/// total number of farms.
pub fn get_number_of_farms(&self) -> u64;

/// get all farms info in a vector. 
/// Note that the from_index and limit are useless for this version,
/// they are just reserved for future work.
pub fn list_farms(&self, from_index: u64, limit: u64) -> Vec<FarmInfo>;

/// The suggested way to batch get farm info;
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

/// all staked seeds and its total amount
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
#[payable]
pub fn create_simple_farm(&mut self, terms: HRSimpleFarmTerms) -> FarmId;
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
/// upgrade the contract
pub fn upgrade(
        &self,
        #[serializer(borsh)] code: Vec<u8>,
        #[serializer(borsh)] migrate: bool,
    ) -> Promise;
```
