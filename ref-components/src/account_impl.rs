//! Default implementation of AccountStorage to store token amounts that given account owns.

use std::collections::HashMap;

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::env;
use near_sdk::json_types::U128;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{AccountId, Balance, StorageUsage};

use crate::account::AccountStorage;
use crate::storage_consts::*;

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct Account {
    /// Amount of NEAR for storage only.
    pub near_amount: U128,
    /// Amount of storage outside the account under this account.
    pub storage_used: StorageUsage,
    /// Amounts for different tokens.
    pub amounts: HashMap<AccountId, U128>,
}

impl Account {
    pub fn add_storage(&mut self, bytes: StorageUsage) {
        self.storage_used += bytes;
        self.assert_storage();
    }

    pub fn remove_storage(&mut self, bytes: StorageUsage) {
        assert!(self.storage_used >= bytes, "ERR_INTERNAL");
        self.storage_used -= bytes;
        self.assert_storage();
    }

    pub fn deposit(&mut self, token_id: &AccountId, amount: Balance) {
        (*self.amounts.entry(token_id.clone()).or_insert(U128(0))).0 += amount;
        self.assert_storage();
    }

    pub fn withdraw(&mut self, token_id: &AccountId, amount: Balance) {
        let current_amount = (*self.amounts.get(token_id).expect("ERR_NOT_ENOUGH_FUNDS")).0;
        assert!(current_amount > amount, "ERR_NOT_ENOUGH_FUNDS");
        if current_amount == amount {
            self.amounts.remove(token_id);
        } else {
            self.amounts
                .insert(token_id.clone(), U128(current_amount - amount));
        }
    }
}

impl AccountStorage for Account {
    fn new(near_amount: Balance) -> Self {
        Self {
            near_amount: U128(near_amount),
            storage_used: 0,
            amounts: HashMap::new(),
        }
    }

    fn storage_total(&self) -> Balance {
        self.near_amount.0
    }

    fn storage_used(&self) -> Balance {
        ((self.amounts.len() as u64) * (MAX_ACCOUNT_ID_BYTES + U128_BYTES)
            + U128_BYTES
            + U64_BYTES
            + self.storage_used) as u128
            * env::storage_byte_cost()
    }

    fn add_storage(&mut self, bytes: StorageUsage) {
        self.storage_used += bytes;
        self.assert_storage();
    }

    fn remove_storage(&mut self, bytes: StorageUsage) {
        assert!(self.storage_used > bytes, "ERR_INTERNAL");
        self.storage_used -= bytes;
    }

    fn remove(&self, _force: bool) {
        // TODO: figure out what to do given this is generic implementation.
    }
}
