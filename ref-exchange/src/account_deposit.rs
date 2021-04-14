//! Account deposit is information per user about their balances in the exchange.

use std::collections::HashMap;
use std::convert::TryInto;

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::{ValidAccountId, U128};
use near_sdk::{
    assert_one_yocto, env, near_bindgen, AccountId, Balance, PromiseResult, StorageUsage,
};

use crate::utils::{ext_fungible_token, ext_self, GAS_FOR_FT_TRANSFER};
use crate::*;

const U128_STORAGE: StorageUsage = 16;
/// bytes length of u64 values
const U64_STORAGE: StorageUsage = 8;
/// bytes length of u32 values. Used in length operations
const U32_STORAGE: StorageUsage = 4;
/// max length of account id * 8 (1 byte)
const ACC_ID_STORAGE: StorageUsage = 64 * 8;
// 64 = max account name length

// ACC_ID: the Contract accounts map
// + U128_STORAGE: near_amount storage
// + U32_STORAGE: tokens HashMap length
// + U64_STORAGE: storage_used
// + 2: version
pub const INIT_ACCOUNT_STORAGE: StorageUsage =
    ACC_ID_STORAGE + U128_STORAGE + U32_STORAGE + 2 * U64_STORAGE + 2;

// NEAR native token. This is not a valid token ID. HACK: NEAR is a native token, we use the
// empty string we use it to reference not existing near account.
// pub const NEAR: AccountId = "".to_string();

#[derive(BorshDeserialize, BorshSerialize)]
pub enum AccountDeposit {
    V2(AccountDepositV2),
}

impl From<AccountDeposit> for AccountDepositV2 {
    fn from(account: AccountDeposit) -> Self {
        match account {
            AccountDeposit::V2(a) => {
                if a.storage_used > 0 {
                    return a;
                }
                // migrate from V1
                a.storage_used = U64_STORAGE;
                a
            }
        }
    }
}

/// Account deposits information and storage cost.
/// Legacy version
#[derive(BorshSerialize, BorshDeserialize, Default)]
#[cfg_attr(feature = "test", derive(Clone))]
pub struct AccountDepositV1 {
    /// NEAR sent to the exchange.
    /// Used for storage and trading.
    pub near_amount: Balance,
    /// Amounts of various tokens in this account.
    pub tokens: HashMap<AccountId, Balance>,
}

/// Account deposits information and storage cost.
#[derive(BorshSerialize, BorshDeserialize, Default)]
#[cfg_attr(feature = "test", derive(Clone))]
pub struct AccountDepositV2 {
    /// NEAR sent to the exchange.
    /// Used for storage and trading.
    pub near_amount: Balance,
    /// Amounts of various tokens in this account.
    pub tokens: HashMap<AccountId, Balance>,
    pub storage_used: StorageUsage,
}

impl From<AccountDepositV2> for AccountDeposit {
    fn from(a: AccountDepositV2) -> Self {
        AccountDeposit::V2(a)
    }
}

impl AccountDepositV2 {
    pub fn new(account_id: &AccountId, near_amount: Balance) -> Self {
        Self {
            near_amount,
            tokens: HashMap::default(),
            // Here we manually compute the initial storage size of account deposit.
            storage_used: U64_STORAGE,
        }
    }

    /// Adds amount to the balance of given token while checking that storage is covered.
    pub(crate) fn add(&mut self, token: &AccountId, amount: Balance) {
        if *token == "" {
            // We use empty string to represent NEAR
            self.near_amount += amount;
        } else if let Some(x) = self.tokens.get_mut(token) {
            *x = *x + amount;
        } else {
            self.tokens.insert(token.clone(), amount);
            self.assert_storage_usage();
        }
    }

    /// Subtract from `token` balance.
    /// Panics if `amount` is bigger than the current balance.
    pub(crate) fn sub(&mut self, token: &AccountId, amount: Balance) {
        if *token == "" {
            // We use empty string to represent NEAR
            self.near_amount -= amount;
            self.assert_storage_usage();
            return;
        }
        let value = *self.tokens.get(token).expect(ERR21_TOKEN_NOT_REG);
        assert!(value >= amount, "{}", ERR22_NOT_ENOUGH_TOKENS);
        self.tokens.insert(token.clone(), value - amount);
    }

    /// Returns amount of $NEAR necessary to cover storage used by account referenced to this structure.
    pub fn storage_usage(&self) -> Balance {
        let s = self.storage_used
            + INIT_ACCOUNT_STORAGE  // empty account storage
            + (ACC_ID_STORAGE + U64_STORAGE) * self.tokens.len() as u64; // self.tokens storage
        return s as Balance * env::storage_byte_cost();
    }

    /// Returns how much NEAR is available for storage and swaps.
    #[inline]
    pub(crate) fn storage_available(&self) -> Balance {
        self.near_amount - self.storage_usage()
    }

    /// Asserts there is sufficient amount of $NEAR to cover storage usage.
    #[inline]
    pub fn assert_storage_usage(&self) {
        assert!(
            self.storage_usage() <= self.near_amount,
            "{}",
            ERR11_INSUFFICIENT_STORAGE
        );
    }

    /// Updates the account storage usage.
    /// Panics if there is not enought $NEAR to cover storage usage.
    pub(crate) fn update_storage(&mut self, tx_start_storage: StorageUsage) {
        self.storage_used += env::storage_usage() - tx_start_storage;
        self.assert_storage_usage();
    }

    /// Registers given `token_id` and set balance to 0.
    /// Panics if there is not enought $NEAR to cover storage usage.
    pub(crate) fn register(&mut self, token_ids: &Vec<ValidAccountId>) {
        for token_id in token_ids {
            let t = token_id.as_ref();
            if !self.tokens.contains_key(t) {
                self.tokens.insert(t.clone(), 0);
            }
        }
        self.assert_storage_usage();
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
    /// Deposits attached NEAR into predecessor account deposits. The deposited near will be used
    /// for trades and for storage. Predecessor account must be registered. Panics otherwise.
    /// NOTE: this is a simplified and more direct version of `storage_deposit` function.
    #[payable]
    pub fn deposit_near(&mut self) {
        let sender_id = env::predecessor_account_id();
        let mut acc = self.get_account(&sender_id);
        acc.near_amount += env::attached_deposit();
        self.accounts.insert(&sender_id, &acc.into());
    }

    /// Registers given token in the user's account deposit.
    /// Fails if not enough balance on this account to cover storage.
    pub fn register_tokens(&mut self, token_ids: Vec<ValidAccountId>) {
        let sender_id = env::predecessor_account_id();
        let mut acc = self.get_account(&sender_id);
        acc.register(&token_ids);
        self.accounts.insert(&sender_id, &acc.into());
    }

    /// Unregister given token from user's account deposit.
    /// Panics if the balance of any given token is non 0.
    pub fn unregister_tokens(&mut self, token_ids: Vec<ValidAccountId>) {
        let sender_id = env::predecessor_account_id();
        let mut deposits = self.get_account(&sender_id);
        for token_id in token_ids {
            deposits.unregister(token_id.as_ref());
        }
        self.accounts.insert(&sender_id, &deposits.into());
    }

    /// Withdraws given token from the deposits of given user.
    /// Optional unregister will try to remove record of this token from AccountDeposit for given user.
    /// Unregister will fail if the left over balance is non 0.
    #[payable]
    pub fn withdraw(&mut self, token_id: ValidAccountId, amount: U128, unregister: Option<bool>) {
        assert_one_yocto();
        let token_id: AccountId = token_id.into();
        let amount: u128 = amount.into();
        let sender_id = env::predecessor_account_id();
        let mut deposits = self.get_account(&sender_id);
        // Note: subtraction and deregistration will be reverted if the promise fails.
        deposits.sub(&token_id, amount);
        if unregister == Some(true) {
            deposits.unregister(&token_id);
        }
        self.accounts.insert(&sender_id, &deposits.into());
        ext_fungible_token::ft_transfer(
            sender_id.clone().try_into().unwrap(),
            amount.into(),
            None,
            &token_id,
            1,
            GAS_FOR_FT_TRANSFER,
        )
        .then(ext_self::exchange_callback_post_withdraw(
            token_id,
            sender_id,
            amount.into(),
            &env::current_account_id(),
            0,
            GAS_FOR_FT_TRANSFER,
        ));
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
                // This reverts the changes from withdraw function.
                let mut deposits = self.get_account(&sender_id);
                deposits.add(&token_id, amount.0);
                self.accounts.insert(&sender_id, &deposits.into());
            }
        };
    }
}

impl Contract {
    /// Registers account in deposited amounts with given amount of $NEAR.
    /// If account already exists, adds amount to it.
    /// This should be used when it's known that storage is prepaid.
    pub(crate) fn register_account(&mut self, account_id: &AccountId, amount: Balance) {
        let acc = if let Some(mut account_deposit) = self.accounts.get(&account_id) {
            account_deposit.near_amount += amount;
            account_deposit
        } else {
            AccountDepositV2::new(account_id, amount)
        };
        self.accounts.insert(&account_id, &acc);
    }

    /// Record deposit of some number of tokens to this contract.
    /// Fails if account is not registered or if token isn't whitelisted.
    pub(crate) fn internal_deposit(
        &mut self,
        sender_id: &AccountId,
        token_id: &AccountId,
        amount: Balance,
    ) {
        let mut acc = self.get_account(sender_id);
        assert!(
            self.whitelisted_tokens.contains(token_id) || acc.tokens.contains_key(token_id),
            "{}",
            ERR12_TOKEN_NOT_WHITELISTED
        );
        acc.add(token_id, amount);
        self.accounts.insert(sender_id, &acc.into());
    }

    // Returns `from` AccountDeposit.
    #[inline]
    pub(crate) fn get_account(&self, from: &AccountId) -> AccountDepositV2 {
        self.accounts
            .get(from)
            .expect(ERR10_ACC_NOT_REGISTERED)
            .into()
    }

    pub(crate) fn get_account_option(&self, from: &AccountId) -> Option<AccountDepositV2> {
        // let key = ("d".to_owned() + from).into_bytes();
        // let data = env::storage_read(&key);
        // if data == None {
        //     return None;
        // }
        // let Some(data) = data;
        // AccountDepositV1::Dese
        // borsh::de::

        self.accounts.get(from).and_then(|a| a.into())
    }

    /// Returns current balance of given token for given user. If token_id == "" then returns NEAR (native)
    /// balance. If there is nothing recorded, returns 0.
    pub(crate) fn get_deposit_balance(
        &self,
        sender_id: &AccountId,
        token_id: &AccountId,
    ) -> Balance {
        let acc = self.get_account(sender_id);
        if token_id == "" {
            return acc.near_amount;
        }
        *acc.tokens.get(token_id).unwrap_or(&0)
    }
}
