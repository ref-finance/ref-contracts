# ref-adboard

### Background

This is a RUST version of adboard AssemblyScript contract by Daniel.

### Init
When initialize the adboard, we should set following params:  

* owner_id: the owner of this contract, has the right to call owner methods,
* amm_id: the ref main contract, play a role of contract of all whitelisted tokens,
* default_token_id and defalut_sell_balance: the initial status of frames, usually wnear's contract, which is wrap.near on mainnet and wrap.testnet on testnet. And the sell_balance is usually set to 1 wnear, which is 10**24. 
* protected_period: the seconds that a frame can not be traded after previous trading.
* frame_count: the total frame counts in this contract.
* trading_fee: in bps, which means a 10,000 denominator.

```rust
pub fn new(
        owner_id: ValidAccountId, 
        amm_id: ValidAccountId, 
        default_token_id: ValidAccountId,
        default_sell_balance: U128,
        protected_period: u16,
        frame_count: u16, 
        trading_fee: u16
    ) -> Self;
```

### Interface Structure

```rust
/// metadata of the contract
pub struct ContractMetadata {
    pub version: String,
    pub owner_id: AccountId,
    pub amm_id: AccountId,
    pub default_token_id: AccountId,
    pub default_sell_balance: U128,
    pub protected_period: u16,
    pub frame_count: u16,
    pub trading_fee: u16,
}

/// metadata of one frame
pub struct HumanReadableFrameMetadata {
    pub token_price: U128,
    pub token_id: AccountId,
    pub owner: AccountId,
    pub protected_ts: U64,
}

/// Payment that failed auto-execution
pub struct HumanReadablePaymentItem {
    pub amount: U128,
    pub token_id: AccountId,
    pub receiver_id: AccountId,
}
```

### Interface

***view functions***  
```rust

pub fn get_metadata(&self) -> ContractMetadata;

/// tokens that permitted in this contract
pub fn get_whitelist(&self) -> Vec<String>;

/// get single frame's metadata
pub fn get_frame_metadata(&self, index: FrameId) -> Option<HumanReadableFrameMetadata>;

/// batch get frame's metadata
pub fn list_frame_metadata(&self, from_index: u64, limit: u64) -> Vec<HumanReadableFrameMetadata>;

/// get single frame's data
pub fn get_frame_data(&self, index: FrameId) -> Option<String>;

/// batch get frame's data
pub fn list_frame_data(&self, from_index: u64, limit: u64) -> Vec<String>;

/// batch get failed payments
pub fn list_failed_payments(&self, from_index: u64, limit: u64) -> Vec<HumanReadablePaymentItem>;

```

***user functions***  
```rust
/// buy frame, call from ref-finance contract
/// with msg is "frame_id||sell_token_id||sell_balance||pool_id"
/// receiver_id is ref-adboard contract id,
/// token_id is the frame's current token,
/// amount is the frame's sell price at that token.
pub fn mft_transfer_call(
        &mut self,
        token_id: String,
        receiver_id: ValidAccountId,
        amount: U128,
        memo: Option<String>,
        msg: String,
    ) -> PromiseOrValue<U128>;
 
/// edit frame
pub fn edit_frame(&mut self, frame_id: FrameId, frame_data: String);
```

***owner functions***  
The owner of this contract is suggested to be some DAO. It can adjust parameters, such as trading fee, protected period, token whitelist, and etc.  
A regular operation for the owner is to repay failure payments. Those payments are generated through frame trading process. It's the last step of the process that the contract would pay prev-sell_balance in prev-frame_token of the trading frame to the prev-owner. But in some rare conditions, this payment would fail. Then it will be recorded in a special vector called failed_payments.  
The ```repay_failure_payment``` interface gives owner the right to handle those payments.

```rust
/// transfer ownership
pub fn set_owner(&mut self, owner_id: ValidAccountId);

/// 
pub fn add_token_to_whitelist(&mut self, token_id: ValidAccountId) -> bool;

///
pub fn remove_token_from_whitelist(&mut self, token_id: ValidAccountId) -> bool;
 
/// handle one failed payment at one call.
pub fn repay_failure_payment(&mut self);

/// owner can change amm account id.
pub fn set_amm(&mut self, amm_id: ValidAccountId);

/// owner can change protected period from being sold after a frame complete trading.
pub fn set_protected_period(&mut self, protected_period: u16);

/// owner can adjust trading fee, the unit is bps, that means a 10,000 denominator. 
pub fn set_trading_fee(&mut self, trading_fee: u16);

/// owner can expand total frames, the new generate frame would have default values.
pub fn expand_frames(&mut self, expend_count: u16);

/// owner can change the default values a frame initially use.
pub fn set_default_token(&mut self, token_id: ValidAccountId, sell_balance: U128);
```