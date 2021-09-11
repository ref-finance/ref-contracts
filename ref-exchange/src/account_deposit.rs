//! Account deposit is information per user about their balances in the exchange.
use near_contract_standards::fungible_token::core_impl::ext_fungible_token;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::UnorderedMap;
use near_sdk::json_types::{ValidAccountId, U128};
use near_sdk::{
    assert_one_yocto, env, near_bindgen, AccountId, Balance, PromiseResult, StorageUsage,
};

use crate::legacy::AccountV1;
use crate::utils::{ext_self, GAS_FOR_FT_TRANSFER, GAS_FOR_RESOLVE_TRANSFER};
use crate::*;

// [AUDIT_01]
// const MAX_ACCOUNT_LENGTH: u128 = 64;
// const MAX_ACCOUNT_BYTES: u128 = MAX_ACCOUNT_LENGTH + 4;
// const MIN_ACCOUNT_DEPOSIT_LENGTH: u128 = 1 + MAX_ACCOUNT_BYTES + 16 + 4;

const U128_STORAGE: StorageUsage = 16;
const U64_STORAGE: StorageUsage = 8;
const U32_STORAGE: StorageUsage = 4;
/// max length of account id is 64 bytes. We charge per byte.
const ACC_ID_STORAGE: StorageUsage = 64;
/// As a key, 4 bytes length would be added to the head
const ACC_ID_AS_KEY_STORAGE: StorageUsage = ACC_ID_STORAGE + 4;
const KEY_PREFIX_ACC: StorageUsage = 64;
/// As a near_sdk::collection key, 1 byte for prefiex
const ACC_ID_AS_CLT_KEY_STORAGE: StorageUsage = ACC_ID_AS_KEY_STORAGE + 1;

// ACC_ID: the Contract accounts map key length
// + VAccount enum: 1 byte
// + U128_STORAGE: near_amount storage
// + U32_STORAGE: tokens UnorderedMap length
// + U64_STORAGE: storage_used
pub const INIT_ACCOUNT_STORAGE: StorageUsage =
    ACC_ID_AS_CLT_KEY_STORAGE + 1 + U128_STORAGE + U32_STORAGE + U64_STORAGE;

#[derive(BorshDeserialize, BorshSerialize)]
pub enum VAccount {
    Current(Account),
    V1(AccountV1),
}

impl VAccount {
    /// Upgrades from other versions to the currently used version.
    pub fn into_current(self, account_id: &AccountId) -> Account {
        match self {
            VAccount::Current(account) => account,
            VAccount::V1(account) => account.into_current(account_id),
        }
    }
}

impl From<Account> for VAccount {
    fn from(account: Account) -> Self {
        VAccount::Current(account)
    }
}

/// Account deposits information and storage cost.
#[derive(BorshSerialize, BorshDeserialize)]
pub struct Account {
    /// Native NEAR amount sent to the exchange.
    /// Used for storage right now, but in future can be used for trading as well.
    pub near_amount: Balance,
    /// Amounts of various tokens deposited to this account.
    pub tokens: UnorderedMap<AccountId, Balance>,
    pub storage_used: StorageUsage,
}

impl Account {
    pub fn new(account_id: &AccountId) -> Self {
        Account {
            near_amount: 0,
            tokens: UnorderedMap::new(StorageKey::AccountTokens {
                account_id: account_id.clone(),
            }),
            storage_used: 0,
        }
    }

    /// Deposit amount to the balance of given token.
    pub(crate) fn deposit(&mut self, token: &AccountId, amount: Balance) {
        env::log(
            format!(
                "[in Account::deposit], before deposit, tokens {:?}",
                self.tokens.to_vec()
            )
            .as_bytes(),
        );
        let balance = self.tokens.get(&token).unwrap_or(0);
        let new_balance = balance
            .checked_add(amount)
            .expect("errors::BALANCE_OVERFLOW");
        self.tokens.insert(token, &new_balance);
        env::log(format!("[in Account::deposit], len of tokens {}", self.tokens.len()).as_bytes());
        env::log(
            format!(
                "[in Account::deposit], balance of dai001 {}",
                self.tokens.get(&String::from("dai001")).unwrap_or(0)
            )
            .as_bytes(),
        );
        env::log(
            format!(
                "[in Account::deposit], balance of eth002 {}",
                self.tokens.get(&String::from("eth002")).unwrap_or(0)
            )
            .as_bytes(),
        );

        env::log(
            format!(
                "[in Account::deposit], after deposited {} to token {}, tokens {:?}",
                amount,
                token,
                self.tokens.to_vec()
            )
            .as_bytes(),
        );
    }

    /// Withdraw amount of `token` from the internal balance.
    /// Panics if `amount` is bigger than the current balance.
    pub(crate) fn withdraw(&mut self, token: &AccountId, amount: Balance) {
        if let Some(x) = self.tokens.get(token) {
            env::log(
                format!(
                    "[in Account::withdraw], before withdraw, tokens {:?}",
                    self.tokens.to_vec()
                )
                .as_bytes(),
            );
            assert!(x >= amount, "{}", ERR22_NOT_ENOUGH_TOKENS);
            self.tokens.insert(token, &(x - amount));

            env::log(
                format!(
                    "[in Account::withdraw], after withdrawn {} to token {}, tokens {:?}",
                    amount,
                    token,
                    self.tokens.to_vec()
                )
                .as_bytes(),
            );
        } else {
            env::panic(ERR21_TOKEN_NOT_REG.as_bytes());
        }
    }

    // [AUDIT_01]
    /// Returns amount of $NEAR necessary to cover storage used by this data structure.
    pub fn storage_usage(&self) -> Balance {
        (INIT_ACCOUNT_STORAGE
            + self.tokens.len() as u64 * (KEY_PREFIX_ACC + ACC_ID_AS_KEY_STORAGE + U128_STORAGE))
            as u128
            * env::storage_byte_cost()
    }

    /// Returns how much NEAR is available for storage.
    pub fn storage_available(&self) -> Balance {
        // [AUDIT_01] avoid math overflow
        let locked = self.storage_usage();
        if self.near_amount > locked {
            self.near_amount - locked
        } else {
            0
        }
    }

    /// Asserts there is sufficient amount of $NEAR to cover storage usage.
    pub fn assert_storage_usage(&self) {
        assert!(
            self.storage_usage() <= self.near_amount,
            "{}",
            ERR11_INSUFFICIENT_STORAGE
        );
    }

    /// like assert_storage_usage but return true or false instead panic directly
    pub fn is_storage_covered(&self) -> bool {
        self.storage_usage() <= self.near_amount
    }

    /// Returns minimal account deposit storage usage possible.
    pub fn min_storage_usage() -> Balance {
        INIT_ACCOUNT_STORAGE as Balance * env::storage_byte_cost()
    }

    /// Registers given token and set balance to 0.
    /// Fails if not enough amount to cover new storage usage.
    pub(crate) fn register(&mut self, token_ids: &Vec<ValidAccountId>) {
        for token_id in token_ids {
            let t = token_id.as_ref();
            if self.tokens.get(t).is_none() {
                self.tokens.insert(t, &0);
            }
        }
    }

    /// Unregisters `token_id` from this account balance.
    /// Panics if the `token_id` balance is not 0.
    pub(crate) fn unregister(&mut self, token_id: &AccountId) {
        let amount = self.tokens.remove(token_id).unwrap_or(0);
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
        let mut deposits = self.internal_unwrap_account(&sender_id);
        deposits.register(&token_ids);
        self.internal_save_account(&sender_id, deposits);
    }

    /// Unregister given token from user's account deposit.
    /// Panics if the balance of any given token is non 0.
    #[payable]
    pub fn unregister_tokens(&mut self, token_ids: Vec<ValidAccountId>) {
        assert_one_yocto();
        let sender_id = env::predecessor_account_id();
        let mut deposits = self.internal_unwrap_account(&sender_id);
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
        let mut deposits = self.internal_unwrap_account(&sender_id);
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
                // This reverts the changes from withdraw function.
                // If account doesn't exit, deposits to the owner's account as lostfound.
                let mut failed = false;
                if let Some(mut account) = self.internal_get_account(&sender_id) {
                    account.deposit(&token_id, amount.0);
                    if account.is_storage_covered() {
                        self.internal_save_account(&sender_id, account);
                    } else {
                        // The deposit added this token to the state.
                        account.tokens.remove(&token_id);
                        env::log(
                            format!(
                                "Account {} has not enough storage. Depositing to owner.",
                                sender_id
                            )
                            .as_bytes(),
                        );
                        failed = true;
                    }
                } else {
                    env::log(
                        format!(
                            "Account {} is not registered. Depositing to owner.",
                            sender_id
                        )
                        .as_bytes(),
                    );
                    failed = true;
                }
                if failed {
                    self.internal_lostfound(&token_id, amount.0);
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
        self.accounts.insert(&account_id, &account.into());
    }

    /// save token to owner account as lostfound
    pub(crate) fn internal_lostfound(&mut self, token_id: &AccountId, amount: u128) {
        let mut lostfound = self.internal_unwrap_or_default_account(&self.owner_id);
        lostfound.deposit(token_id, amount);
        self.accounts.insert(&self.owner_id, &lostfound.into());
    }

    /// Registers account in deposited amounts with given amount of $NEAR.
    /// If account already exists, adds amount to it.
    /// This should be used when it's known that storage is prepaid.
    pub(crate) fn internal_register_account(&mut self, account_id: &AccountId, amount: Balance) {
        let mut account = self.internal_unwrap_or_default_account(&account_id);
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
        let mut account = self.internal_unwrap_account(sender_id);
        assert!(
            self.whitelisted_tokens.contains(token_id) || account.tokens.get(token_id).is_some(),
            "{}",
            ERR12_TOKEN_NOT_WHITELISTED
        );
        account.deposit(token_id, amount);
        self.internal_save_account(&sender_id, account);
    }

    pub fn internal_get_account(&self, account_id: &AccountId) -> Option<Account> {
        self.accounts
            .get(account_id)
            .map(|va| va.into_current(account_id))
    }

    pub fn internal_unwrap_account(&self, account_id: &AccountId) -> Account {
        self.internal_get_account(account_id)
            .expect(errors::ERR10_ACC_NOT_REGISTERED)
    }

    pub fn internal_unwrap_or_default_account(&self, account_id: &AccountId) -> Account {
        self.internal_get_account(account_id)
            .unwrap_or_else(|| Account::new(account_id))
    }

    /// Returns current balance of given token for given user. If there is nothing recorded, returns 0.
    pub(crate) fn internal_get_deposit(
        &self,
        sender_id: &AccountId,
        token_id: &AccountId,
    ) -> Balance {
        self.internal_get_account(sender_id)
            .and_then(|a| a.tokens.get(token_id))
            .unwrap_or(0)
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
            GAS_FOR_RESOLVE_TRANSFER,
        ))
    }
}
