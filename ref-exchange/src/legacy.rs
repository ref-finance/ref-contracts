//! This modules captures all the code needed to migrate from previous version.
use std::collections::HashMap;
use near_sdk::collections::UnorderedMap;

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{AccountId, Balance, StorageUsage};

use crate::account_deposit::{Account};
use crate::StorageKey;

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