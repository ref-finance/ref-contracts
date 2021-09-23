//! This modules captures all the code needed to migrate from previous version.
use std::collections::HashMap;
use near_sdk::collections::{UnorderedMap, Vector, LookupMap, UnorderedSet};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{AccountId, Balance, StorageUsage, near_bindgen, PanicOnDefault};
use crate::account_deposit::{Account, VAccount};
use crate::StorageKey;
use crate::pool::Pool;

/// Account deposits information and storage cost.
#[derive(BorshSerialize, BorshDeserialize, Default, Clone)]
pub struct AccountV1 {
    /// Native NEAR amount sent to the exchange.
    /// Used for storage right now, but in future can be used for trading as well.
    pub near_amount: Balance,
    /// Amounts of various tokens deposited to this account.
    pub tokens: HashMap<AccountId, Balance>,
    pub storage_used: StorageUsage,
}

impl AccountV1 {
    pub fn into_current(&self, account_id: &AccountId) -> Account {
        Account {
            near_amount: self.near_amount,
            legacy_tokens: self.tokens.clone(),
            tokens: UnorderedMap::new(StorageKey::AccountTokens {
                account_id: account_id.clone(),
            }),
            storage_used: self.storage_used,
        }
    }
}

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
