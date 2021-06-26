//! This modules captures all the code needed to migrate from previous version.

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LookupMap, UnorderedSet, Vector};
use near_sdk::{near_bindgen, AccountId,};

use crate::account_deposit::Account;
use crate::pool::Pool;




#[near_bindgen]
#[derive(BorshSerialize, BorshDeserialize)]
pub struct OldContract {
    /// Account of the owner.
    pub owner_id: AccountId,
    /// Exchange fee, that goes to exchange itself (managed by governance).
    exchange_fee: u32,
    /// Referral fee, that goes to referrer in the call.
    referral_fee: u32,
    /// List of all the pools.
    pub pools: Vector<Pool>,
    /// Accounts registered, keeping track all the amounts deposited, storage and more.
    pub accounts: LookupMap<AccountId, Account>,
    /// Set of whitelisted tokens by "owner".
    pub whitelisted_tokens: UnorderedSet<AccountId>,
}
