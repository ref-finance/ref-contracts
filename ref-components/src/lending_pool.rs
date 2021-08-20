//! General lending pool that can work with different types of collateral and borrowing tokens.
//! Can be used for implementing lending applications or synthetic asset protocols.

use std::collections::HashMap;

use crate::account::AccountStorage;
use crate::storage_consts::{MAX_ACCOUNT_ID_BYTES, U128_BYTES, U32_BYTES, U64_BYTES};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::LookupMap;
use near_sdk::json_types::U128;
use near_sdk::{
    env, near_bindgen, AccountId, Balance, BorshStorageKey, PanicOnDefault, Promise,
    PromiseOrValue, StorageUsage,
};

#[derive(BorshSerialize, BorshDeserialize)]
struct LendingToken {
    /// Can this token be borrowed.
    paused: bool,
    /// If rate is 0, it can not be a collateral.
    collateral_rate: Balance,
    /// Latest price from the oracle in global accounting unit.
    oracle_price: Balance,
}

#[derive(BorshSerialize, BorshDeserialize)]
struct LendingAccount {
    /// Used for storage accounting.
    near_amount: Balance,
    /// Extra storage usage.
    storage_used: StorageUsage,
    /// All the deposited collaterals.
    collaterals: HashMap<AccountId, Balance>,
    /// All the debt.
    debts: HashMap<AccountId, Balance>,
}

impl AccountStorage for LendingAccount {
    fn new(near_amount: Balance) -> Self {
        LendingAccount {
            near_amount,
            storage_used: 0,
            collaterals: HashMap::new(),
            debts: HashMap::new(),
        }
    }

    fn storage_total(&self) -> Balance {
        self.near_amount
    }

    fn storage_used(&self) -> Balance {
        (U128_BYTES
            + U64_BYTES
            + U32_BYTES
            + (self.collaterals.len() as u64) * (MAX_ACCOUNT_ID_BYTES + U128_BYTES)
            + U32_BYTES
            + (self.debts.len() as u64) * (MAX_ACCOUNT_ID_BYTES + U128_BYTES)) as u128
            * env::storage_byte_cost()
    }

    fn min_storage_usage() -> Balance {
        todo!()
    }

    fn add_storage(&mut self, bytes: StorageUsage) {
        todo!()
    }

    fn remove_storage(&mut self, bytes: StorageUsage) {
        todo!()
    }

    fn remove(&self, force: bool) {
        todo!()
    }
}

impl LendingAccount {
    /// Record deposit into the account.
    /// Check that there is enough storage.
    pub fn deposit(&mut self, token_id: &AccountId, amount: Balance) {
        let prev_amount = self.collaterals.get(token_id).unwrap_or(&0);
        self.collaterals
            .insert(token_id.clone(), (prev_amount + amount));
    }

    /// Record withdraw from the account.
    /// Fails if not enough amount available.
    pub fn withdraw(&mut self, token_id: &AccountId, amount: Balance) {
        let prev_amount = self.collaterals.get(token_id).unwrap_or(&0);
        // assert!(prevA)
        self.collaterals
            .insert(token_id.clone(), (prev_amount - amount));
    }
}

/// Lender maintains all the collateral/lending information.
#[derive(BorshSerialize, BorshDeserliaze)]
struct LendingPool {
    tokens: LookupMap<AccountId, LendingToken>,
    accounts: LookupMap<AccountId, LendingAccount>,
}

impl LendingPool {
    pub fn new() -> Self {
        Self {
            tokens: LookupMap::new(b"t".to_vec()),
            accounts: LookupMap::new(b"a".to_vec()),
        }
    }

    pub fn add_token(&mut self, token_id: &AccountId, info: LendingToken) {
        self.tokens.insert(&token_id, &info);
    }

    pub fn update_token(&mut self, token_id: &AccountId, info: LendingToken) {
        self.tokens.insert(&token_id, &info);
    }

    pub fn pause_token(&mut self, token_id: &AccountId) {
        // let info = self.tokens.get(token_id).unwrap();
        // info.pause = true;
        // self.update_token(token_id, info);
    }

    pub fn unpause_token(&mut self, token_id: &AccountId) {
        // let info = self.tokens.get(token_id).unwrap();
        // info.pause = false;
        // self.update_token(token_id, info);
    }

    /// Called by oracle to set the price for given token in the global accounting currency.
    pub fn set_price(&mut self, token_id: &AccountId, price: Balance) {
        // let info = self.tokens.get(token_id).unwrap();
        // info.latest_price = price;
        // self.update_token(token_id, info);
    }

    /// Deposit given amount of token.
    pub fn deposit(&mut self, sender_id: &AccountId, token_id: &AccountId, amount: Balance) {
        // let info = self.tokens.get(token_id).unwrap();
        // assert!(!info.paused);
        // info.total_amount += amount;
        // self.update_token(token_id, info);
        // let mut acc_info = self.accounts.get(sender_id);
        // acc_info.deposit(token_id, amount);
        // self.accounts.insert(sender_id, &acc_info);
    }

    /// Borrow given amount of token.
    pub fn borrow(&mut self, sender_id: &AccountId, token_id: &AccountId, amount: Balance) {}

    /// Repay the borrowed amount.
    pub fn repay(&mut self, sender_id: &AccountId, token_id: &AccountId, amount: Balance) {}

    /// Withdraw given amount of token.
    pub fn withdraw(&mut self, sender_id: &AccountId, token_id: &AccountId, amount: Balance) {}

    /// Liquidate outstanding loan that is under collaterized.
    pub fn liquidate(
        &mut self,
        sender_id: &AccountId,
        account_id: &AccountId,
        collateral_token_id: &AccountId,
        borrow_token_id: &AccountId,
        amount: Balance,
    ) {

    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lending() {
        // let mut lending_pool = LendingPool::new();
        // lending_pool.add_token(
        //     "test1",
        //     LendingToken {
        //         paused: false,
        //         collateral_rate: 110,
        //         oracle_price: 1,
        //     },
        // );
        // lending_pool.add_token(
        //     "test2",
        //     LendingToken {
        //         paused: false,
        //         collateral_rate: 0,
        //         oracle_price: 1,
        //     },
        // );
        // lending_pool.deposit("user1", "test1", 100_000);
        // lending_pool.deposit("user2", "test2", 100_000);
        // lending_pool.borrow("user1", "test2", 50_000);
    }
}
