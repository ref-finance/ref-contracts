use std::collections::HashMap;

use near_contract_standards::storage_management::{StorageBalance, StorageBalanceBounds};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::LookupMap;
use near_sdk::json_types::{ValidAccountId, U128};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{assert_one_yocto, env, log, AccountId, Balance, Promise, StorageUsage};
use std::convert::TryInto;

/// Max account length is 64 + 4 bytes for serialization.
const MAX_ACCOUNT_LENGTH: u64 = 68;

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct Account {
    /// Amount of NEAR for storage only.
    pub near_amount: U128,
    /// Number of active offers.
    pub num_offers: u32,
    /// Amounts for different tokens.
    pub amounts: HashMap<AccountId, U128>,
}

impl Account {
    pub fn add_offer(&mut self) {
        self.num_offers += 1;
        self.assert_storage();
    }

    pub fn remove_offer(&mut self) {
        assert!(self.num_offers > 0, "ERR_INTERNAL");
        self.num_offers -= 1;
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

    fn storage_used(&self) -> StorageUsage {
        // Single Offer is up to 320 bytes.
        (self.amounts.len() as u64) * (MAX_ACCOUNT_LENGTH + 16)
            + 16
            + 4
            + (self.num_offers as u64) * 320
    }

    pub fn assert_storage(&self) {
        assert!(
            (self.storage_used() as u128) * env::storage_byte_cost() < self.near_amount.0,
            "ERR_NO_STORAGE"
        );
    }
}

impl AccountStorage for Account {
    fn new(near_amount: Balance) -> Self {
        Self {
            near_amount: U128(near_amount),
            num_offers: 0,
            amounts: HashMap::new(),
        }
    }

    fn storage_total(&self) -> Balance {
        self.near_amount.0
    }

    fn storage_available(&self) -> Balance {
        self.near_amount.0 - self.storage_used() as u128 * env::storage_byte_cost()
    }

    fn min_storage_usage() -> Balance {
        16 + 4
    }

    fn remove(&self, _force: bool) {
        // TODO: currently doesn't reassign.
    }
}

/// Trait for account to manage it's internal storage.
pub trait AccountStorage: BorshSerialize + BorshDeserialize {
    fn new(near_amount: Balance) -> Self;
    fn storage_total(&self) -> Balance;
    fn storage_available(&self) -> Balance;
    fn min_storage_usage() -> Balance;

    /// Should handle removing account.
    /// If not `force` can fail if account is not ready to be removed.
    /// If `force` should re-assign any resources to owner or alternative and remove the account.
    fn remove(&self, force: bool);
}

/// Manages user accounts in the contract.
#[derive(BorshSerialize, BorshDeserialize)]
pub struct AccountManager<Account>
where
    Account: AccountStorage,
{
    accounts: LookupMap<AccountId, Account>,
}

/// Generic account manager that handles storage and updates of accounts.
impl<Account> AccountManager<Account>
where
    Account: AccountStorage,
{
    pub fn new() -> Self {
        Self {
            accounts: LookupMap::new(b"a".to_vec()),
        }
    }

    /// Get account from the storage.
    pub fn get_account(&self, account_id: &AccountId) -> Option<Account> {
        self.accounts.get(account_id)
    }

    /// Set account to the storage.
    pub fn set_account(&mut self, account_id: &AccountId, account: &Account) {
        self.accounts.insert(account_id, account);
    }

    /// Should handle removing account from storage.
    /// If not `force` can fail if account is not ready to be removed.
    /// If `force` should re-assign any resources to owner or alternative and remove the account.
    pub fn remove_account(&mut self, account_id: &AccountId, force: bool) {
        let account = self.get_account_or(account_id);
        account.remove(force);
        self.accounts.remove(account_id);
    }

    pub fn get_account_or(&self, account_id: &AccountId) -> Account {
        self.get_account(account_id).expect("ERR_MISSING_ACCOUNT")
    }

    pub fn update_account<F>(&mut self, account_id: &AccountId, f: F)
    where
        F: Fn(&mut Account),
    {
        let mut account = self.get_account_or(account_id);
        f(&mut account);
        self.set_account(&account_id, &account);
    }

    pub fn internal_register_account(&mut self, account_id: &AccountId, near_amount: Balance) {
        let account = Account::new(near_amount);
        self.set_account(account_id, &account);
    }

    pub fn internal_storage_deposit(
        &mut self,
        account_id: Option<ValidAccountId>,
        registration_only: Option<bool>,
    ) -> StorageBalance {
        let amount = env::attached_deposit();
        let account_id = account_id
            .map(|a| a.into())
            .unwrap_or_else(|| env::predecessor_account_id());
        let registration_only = registration_only.unwrap_or(false);
        let min_balance = self.internal_storage_balance_bounds().min.0;
        let already_registered = self.get_account(&account_id).is_some();
        if amount < min_balance && !already_registered {
            env::panic(b"ERR_DEPOSIT_LESS_THAN_MIN_STORAGE");
        }
        if registration_only {
            // Registration only setups the account but doesn't leave space for tokens.
            if already_registered {
                log!("ERR_ACC_REGISTERED");
                if amount > 0 {
                    Promise::new(env::predecessor_account_id()).transfer(amount);
                }
            } else {
                self.internal_register_account(&account_id, min_balance);
                let refund = amount - min_balance;
                if refund > 0 {
                    Promise::new(env::predecessor_account_id()).transfer(refund);
                }
            }
        } else {
            self.internal_register_account(&account_id, amount);
        }
        self.internal_storage_balance_of(account_id.try_into().unwrap())
            .unwrap()
    }

    pub fn internal_storage_withdraw(&mut self, amount: Option<U128>) -> StorageBalance {
        assert_one_yocto();
        let account_id = env::predecessor_account_id();
        let account = self.get_account_or(&account_id);
        let available = account.storage_available();
        let amount = amount.map(|a| a.0).unwrap_or(available);
        assert!(amount <= available, "ERR_STORAGE_WITHDRAW_TOO_MUCH");
        Promise::new(account_id.clone()).transfer(amount);
        self.internal_storage_balance_of(account_id.try_into().unwrap())
            .unwrap()
    }

    /// Unregisters the account.
    pub fn internal_storage_unregister(&mut self, force: Option<bool>) -> bool {
        assert_one_yocto();
        let account_id = env::predecessor_account_id();
        if let Some(account) = self.get_account(&account_id) {
            self.remove_account(&account_id, force.unwrap_or(false));
            Promise::new(account_id.clone()).transfer(account.storage_total());
            true
        } else {
            false
        }
    }

    pub fn internal_storage_balance_bounds(&self) -> StorageBalanceBounds {
        StorageBalanceBounds {
            min: Account::min_storage_usage().into(),
            max: None,
        }
    }

    pub fn internal_storage_balance_of(
        &self,
        account_id: ValidAccountId,
    ) -> Option<StorageBalance> {
        self.get_account(account_id.as_ref())
            .map(|account| StorageBalance {
                total: U128(account.storage_total()),
                available: U128(account.storage_available()),
            })
    }
}
