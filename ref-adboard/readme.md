# ref-adboard

### Background

This is a rust version of adboard as contract of Daniel.


### Interface Structure

```rust
/// metadata of one frame
pub struct HumanReadableFrameMetadata {
    pub token_price: U128,
    pub token_id: AccountId,
    pub owner: AccountId,
    pub protected_ts: U64,
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

```
