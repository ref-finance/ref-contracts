//! Account deposit is information per user about their balances in the exchange.

use std::collections::HashMap;

use near_contract_standards::fungible_token::core_impl::ext_fungible_token;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::{ValidAccountId, U128};
use near_sdk::{assert_one_yocto, env, near_bindgen, AccountId, Balance, PromiseResult};

use crate::utils::{ext_self, GAS_FOR_FT_TRANSFER};
use crate::*;

const MAX_ACCOUNT_LENGTH: u128 = 64;
const MIN_ACCOUNT_DEPOSIT_LENGTH: u128 = MAX_ACCOUNT_LENGTH + 16 + 4;

/// Account deposits information and storage cost.
#[derive(BorshSerialize, BorshDeserialize, Default, Clone)]
pub struct Account {
    /// Native NEAR amount sent to the exchange.
    /// Used for storage right now, but in future can be used for trading as well.
    pub near_amount: Balance,
    /// Amounts of various tokens deposited to this account.
    pub tokens: HashMap<AccountId, Balance>,
}

impl Account {
    /// Deposit amount to the balance of given token.
    pub(crate) fn deposit(&mut self, token: &AccountId, amount: Balance) {
        if let Some(x) = self.tokens.get_mut(token) {
            *x = *x + amount;
        } else {
            self.tokens.insert(token.clone(), amount);
        }
    }

    /// Withdraw amount of `token` from the internal balance.
    /// Panics if `amount` is bigger than the current balance.
    pub(crate) fn withdraw(&mut self, token: &AccountId, amount: Balance) {
        let value = *self.tokens.get(token).expect(ERR21_TOKEN_NOT_REG);
        assert!(value >= amount, "{}", ERR22_NOT_ENOUGH_TOKENS);
        self.tokens.insert(token.clone(), value - amount);
    }

    /// Returns amount of $NEAR necessary to cover storage used by this data structure.
    pub fn storage_usage(&self) -> Balance {
        (MIN_ACCOUNT_DEPOSIT_LENGTH + self.tokens.len() as u128 * (MAX_ACCOUNT_LENGTH + 16))
            * env::storage_byte_cost()
    }

    /// Returns how much NEAR is available for storage.
    pub fn storage_available(&self) -> Balance {
        self.near_amount - self.storage_usage()
    }

    /// Asserts there is sufficient amount of $NEAR to cover storage usage.
    pub fn assert_storage_usage(&self) {
        assert!(
            self.storage_usage() <= self.near_amount,
            "{}",
            ERR11_INSUFFICIENT_STORAGE
        );
    }

    /// Returns minimal account deposit storage usage possible.
    pub fn min_storage_usage() -> Balance {
        MIN_ACCOUNT_DEPOSIT_LENGTH * env::storage_byte_cost()
    }

    /// Registers given token and set balance to 0.
    /// Fails if not enough amount to cover new storage usage.
    pub(crate) fn register(&mut self, token_ids: &Vec<ValidAccountId>) {
        for token_id in token_ids {
            let t = token_id.as_ref();
            if !self.tokens.contains_key(t) {
                self.tokens.insert(t.clone(), 0);
            }
        }
    }

    /// Unregisters `token_id` from this account balance.
    /// Panics if the `token_id` balance is not 0.
    pub(crate) fn unregister(&mut self, token_id: &AccountId) {
        let amount = self.tokens.remove(token_id).unwrap_or_default();
        assert_eq!(amount, 0, "{}", ERR24_NON_ZERO_TOKEN_BALANCE);
    }
}

#[near_bindgen]
impl Contract {
    /// Registers given token in the user's account deposit.
    /// Fails if not enough balance on this account to cover storage.
    #[payable]
    pub fn register_tokens(&mut self, token_ids: Vec<ValidAccountId>) {
        assert_one_yocto();
        let sender_id = env::predecessor_account_id();
        let mut deposits = self.get_account_deposits(&sender_id);
        deposits.register(&token_ids);
        self.internal_save_account(&sender_id, deposits);
    }

    /// Unregister given token from user's account deposit.
    /// Panics if the balance of any given token is non 0.
    #[payable]
    pub fn unregister_tokens(&mut self, token_ids: Vec<ValidAccountId>) {
        assert_one_yocto();
        let sender_id = env::predecessor_account_id();
        let mut deposits = self.get_account_deposits(&sender_id);
        for token_id in token_ids {
            deposits.unregister(token_id.as_ref());
        }
        self.internal_save_account(&sender_id, deposits);
    }

    /// Withdraws given token from the deposits of given user.
    /// Optional unregister will try to remove record of this token from AccountDeposit for given user.
    /// Unregister will fail if the left over balance is non 0.
    #[payable]
    pub fn withdraw(
        &mut self,
        token_id: ValidAccountId,
        amount: U128,
        unregister: Option<bool>,
    ) -> Promise {
        assert_one_yocto();
        let token_id: AccountId = token_id.into();
        let amount: u128 = amount.into();
        let sender_id = env::predecessor_account_id();
        let mut deposits = self.get_account_deposits(&sender_id);
        // Note: subtraction and deregistration will be reverted if the promise fails.
        deposits.withdraw(&token_id, amount);
        if unregister == Some(true) {
            deposits.unregister(&token_id);
        }
        self.internal_save_account(&sender_id, deposits);
        self.internal_send_tokens(&sender_id, &token_id, amount)
    }

    #[private]
    pub fn exchange_callback_post_withdraw(
        &mut self,
        token_id: AccountId,
        sender_id: AccountId,
        amount: U128,
    ) {
        assert_eq!(
            env::promise_results_count(),
            1,
            "{}",
            ERR25_CALLBACK_POST_WITHDRAW_INVALID
        );
        match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Successful(_) => {}
            PromiseResult::Failed => {
                // This reverts the changes from withdraw function. If account doesn't exit, deposits to the owner's account.
                if let Some(mut account) = self.accounts.get(&sender_id) {
                    account.deposit(&token_id, amount.0);
                    self.internal_save_account(&sender_id, account);
                } else {
                    env::log(
                        format!(
                            "Account {} is not registered or not enough storage. Depositing to owner.",
                            sender_id
                        )
                        .as_bytes(),
                    );
                    let mut owner_account = self.get_account_deposits(&self.owner_id);
                    owner_account.deposit(&token_id, amount.0);
                    self.internal_save_account(&self.owner_id.clone(), owner_account);
                }
            }
        };
    }
}

impl Contract {
    /// Checks that account has enough storage to be stored and saves it into collection.
    /// This should be only place to directly use `self.accounts`.
    pub(crate) fn internal_save_account(&mut self, account_id: &AccountId, account: Account) {
        account.assert_storage_usage();
        self.accounts.insert(&account_id, &account);
    }

    /// Registers account in deposited amounts with given amount of $NEAR.
    /// If account already exists, adds amount to it.
    /// This should be used when it's known that storage is prepaid.
    pub(crate) fn internal_register_account(&mut self, account_id: &AccountId, amount: Balance) {
        let mut account = self.accounts.get(&account_id).unwrap_or_default();
        account.near_amount += amount;
        self.internal_save_account(&account_id, account);
    }

    /// Record deposit of some number of tokens to this contract.
    /// Fails if account is not registered or if token isn't whitelisted.
    pub(crate) fn internal_deposit(
        &mut self,
        sender_id: &AccountId,
        token_id: &AccountId,
        amount: Balance,
    ) {
        let mut account = self.get_account_deposits(sender_id);
        assert!(
            self.whitelisted_tokens.contains(token_id) || account.tokens.contains_key(token_id),
            "{}",
            ERR12_TOKEN_NOT_WHITELISTED
        );
        account.deposit(token_id, amount);
        self.internal_save_account(&sender_id, account);
    }

    // Returns `from` AccountDeposit.
    #[inline]
    pub(crate) fn get_account_deposits(&self, from: &AccountId) -> Account {
        self.accounts.get(from).expect(ERR10_ACC_NOT_REGISTERED)
    }

    /// Returns current balance of given token for given user. If there is nothing recorded, returns 0.
    pub(crate) fn internal_get_deposit(
        &self,
        sender_id: &AccountId,
        token_id: &AccountId,
    ) -> Balance {
        self.accounts
            .get(sender_id)
            .and_then(|d| d.tokens.get(token_id).cloned())
            .unwrap_or_default()
    }

    /// Sends given amount to given user and if it fails, returns it back to user's balance.
    /// Tokens must already be subtracted from internal balance.
    pub(crate) fn internal_send_tokens(
        &self,
        sender_id: &AccountId,
        token_id: &AccountId,
        amount: Balance,
    ) -> Promise {
        ext_fungible_token::ft_transfer(
            sender_id.clone(),
            U128(amount),
            None,
            token_id,
            1,
            GAS_FOR_FT_TRANSFER,
        )
        .then(ext_self::exchange_callback_post_withdraw(
            token_id.clone(),
            sender_id.clone(),
            U128(amount),
            &env::current_account_id(),
            0,
            GAS_FOR_FT_TRANSFER,
        ))
    }
}
