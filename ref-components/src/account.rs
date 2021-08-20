use std::convert::TryInto;

use near_contract_standards::storage_management::{StorageBalance, StorageBalanceBounds};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::LookupMap;
use near_sdk::json_types::{ValidAccountId, U128};
use near_sdk::{assert_one_yocto, env, log, AccountId, Balance, Promise, StorageUsage};

/// Trait for an account to manage it's internal storage.
pub trait AccountStorage: BorshSerialize + BorshDeserialize {
    /// Create new account.
    fn new(near_amount: Balance) -> Self;

    /// Total storage in this account paid via deposit in $NEAR.
    fn storage_total(&self) -> Balance;

    /// Total storage used in $NEAR.
    fn storage_used(&self) -> Balance;

    /// Storage available for this account in $NEAR.
    fn storage_available(&self) -> Balance {
        self.storage_total()
            .checked_sub(self.storage_used())
            .unwrap_or(0)
    }

    /// Minimum amount of $NEAR required to store empty account.
    fn min_storage_usage() -> Balance {
        Self::new(0).storage_used()
    }

    /// Add extra storage that's not stored under the account.
    /// Should fail if there is not enough deposit to cover this extra storage.
    fn add_storage(&mut self, bytes: StorageUsage);

    /// Remove storage that's not stored under the account.
    fn remove_storage(&mut self, bytes: StorageUsage);

    /// Should handle removing account.
    /// If not `force` can fail if account is not ready to be removed.
    /// If `force` should re-assign any resources to owner or alternative and remove the account.
    fn remove(&self, force: bool);

    fn assert_storage(&self) {
        assert!(
            self.storage_used() <= self.storage_total(),
            "ERR_NO_STORAGE"
        );
    }
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
