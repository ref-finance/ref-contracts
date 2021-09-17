//! This modules captures all the code needed to migrate from previous version.
use near_sdk::collections::{Vector, LookupMap, UnorderedSet};

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{AccountId, near_bindgen, PanicOnDefault};

use crate::account_deposit::{VAccount};
use crate::pool::Pool;

#[near_bindgen]
#[derive(BorshSerialize, BorshDeserialize, PanicOnDefault)]
pub struct ContractV1 {
    /// Account of the owner.
    pub owner_id: AccountId,
    /// Exchange fee, that goes to exchange itself (managed by governance).
    pub exchange_fee: u32,
    /// Referral fee, that goes to referrer in the call.
    pub referral_fee: u32,
    /// List of all the pools.
    pub pools: Vector<Pool>,
    /// Accounts registered, keeping track all the amounts deposited, storage and more.
    pub accounts: LookupMap<AccountId, VAccount>,
    /// Set of whitelisted tokens by "owner".
    pub whitelisted_tokens: UnorderedSet<AccountId>,
}
