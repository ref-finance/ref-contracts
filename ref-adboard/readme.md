# ref-adboard

### Background

This is a RUST version of adboard AssemblyScript contract by Daniel.


### Interface Structure

```rust
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
```rust
/// transfer ownership
pub fn set_owner(&mut self, owner_id: ValidAccountId);

/// 
pub fn add_token_to_whitelist(&mut self, token_id: ValidAccountId) -> bool;

///
pub fn remove_token_from_whitelist(&mut self, token_id: ValidAccountId) -> bool;
 
/// handle one failed payment at one call.
pub fn repay_failure_payment(&mut self);
```