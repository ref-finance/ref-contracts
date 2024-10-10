use near_contract_standards::fungible_token::metadata::FungibleTokenMetadata;
use near_sdk::json_types::{ValidAccountId, U128};
use near_sdk::{ext_contract, near_bindgen, Balance, PromiseOrValue};

use crate::utils::{GAS_FOR_FT_TRANSFER_CALL, GAS_FOR_RESOLVE_TRANSFER, NO_DEPOSIT};
use crate::*;

#[ext_contract(ext_self)]
trait MFTTokenResolver {
    fn mft_resolve_transfer(
        &mut self,
        token_id: String,
        sender_id: AccountId,
        receiver_id: AccountId,
        amount: U128,
    ) -> U128;
}

#[ext_contract(ext_share_token_receiver)]
pub trait MFTTokenReceiver {
    fn mft_on_transfer(
        &mut self,
        token_id: String,
        sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128>;
}

enum TokenOrPool {
    Token(AccountId),
    Pool(u64),
}

/// [AUDIT_06]
/// This is used to parse token_id fields in mft protocol used in ref,
/// So, if we choose #nn as a partern, should announce it in mft protocol.
/// cause : is not allowed in a normal account id, it can be a partern leading char
fn try_identify_pool_id(token_id: &String) -> Result<u64, &'static str> {
    if token_id.starts_with(":") {
        if let Ok(pool_id) = str::parse::<u64>(&token_id[1..token_id.len()]) {
            Ok(pool_id)
        } else {
            Err(ERR87_ILLEGAL_POOL_ID)
        }
    } else {
        Err(ERR87_ILLEGAL_POOL_ID)
    }
}

fn parse_token_id(token_id: String) -> TokenOrPool {
    if let Ok(pool_id) = try_identify_pool_id(&token_id) {
        TokenOrPool::Pool(pool_id)
    } else {
        TokenOrPool::Token(token_id)
    }
}

impl Contract {
    pub fn internal_mft_transfer(
        &mut self,
        token_id: String,
        sender_id: &AccountId,
        receiver_id: &AccountId,
        amount: Option<u128>,
        memo: Option<String>,
    ) -> Balance {
        // [AUDIT_07]
        assert_ne!(sender_id, receiver_id, "{}", ERR33_TRANSFER_TO_SELF);
        let transfer_amount = match parse_token_id(token_id) {
            TokenOrPool::Pool(pool_id) => {
                let mut pool = self.pools.get(pool_id).expect(ERR85_NO_POOL);

                let total_shares = pool.share_balances(sender_id);
                let available_shares = if let Some(sender_account) = self.internal_get_account(sender_id) {
                    if let Some(record) = sender_account.get_shadow_record(pool_id) {
                        record.free_shares(total_shares)
                    } else {
                        total_shares
                    }
                } else {
                    total_shares
                };
                let amount = amount.unwrap_or(available_shares);
                assert!(amount > 0, "transfer_amount must be greater than zero");
                assert!(amount <= available_shares, "Not enough free shares");
                
                pool.share_transfer(sender_id, receiver_id, amount);
                self.pools.replace(pool_id, &pool);
                log!(
                    "Transfer shares {} pool: {} from {} to {}",
                    pool_id,
                    amount,
                    sender_id,
                    receiver_id
                );
                amount
            }
            TokenOrPool::Token(token_id) => {
                // TokenOrPool::Token unsupport transfer all
                let amount = amount.unwrap_or_else(||{unimplemented!()});

                let mut sender_account: Account = self.internal_unwrap_account(&sender_id);
                let mut receiver_account: Account = self.internal_unwrap_account(&receiver_id);

                sender_account.withdraw(&token_id, amount);
                receiver_account.deposit(&token_id, amount);
                self.internal_save_account(&sender_id, sender_account);
                self.internal_save_account(&receiver_id, receiver_account);
                log!(
                    "Transfer {}: {} from {} to {}",
                    token_id,
                    amount,
                    sender_id,
                    receiver_id
                );
                amount
            }
        };
        if let Some(memo) = memo {
            log!("Memo: {}", memo);
        }
        transfer_amount
    }

    fn internal_mft_balance(&self, token_id: String, account_id: &AccountId) -> Balance {
        match parse_token_id(token_id) {
            TokenOrPool::Pool(pool_id) => {
                let pool = self.pools.get(pool_id).expect(ERR85_NO_POOL);
                pool.share_balances(account_id)
            }
            TokenOrPool::Token(token_id) => self.internal_get_deposit(account_id, &token_id),
        }
    }
}

#[near_bindgen]
impl Contract {

    /// Returns the balance of the given account. If the account doesn't exist will return `"0"`.
    pub fn mft_balance_of(&self, token_id: String, account_id: ValidAccountId) -> U128 {
        self.internal_mft_balance(token_id, account_id.as_ref())
            .into()
    }

    /// Returns the total supply of the given token, if the token is one of the pools.
    /// If token references external token - fails with unimplemented.
    pub fn mft_total_supply(&self, token_id: String) -> U128 {
        match parse_token_id(token_id) {
            TokenOrPool::Pool(pool_id) => {
                let pool = self.pools.get(pool_id).expect(ERR85_NO_POOL);
                U128(pool.share_total_balance())
            }
            TokenOrPool::Token(_token_id) => unimplemented!(),
        }
    }

    pub fn mft_has_registered(&self, token_id: String, account_id: ValidAccountId) -> bool {
        match parse_token_id(token_id) {
            TokenOrPool::Token(_) => false,
            TokenOrPool::Pool(pool_id) => {
                if let Some(pool) = self.pools.get(pool_id) {
                    pool.share_has_registered(account_id.as_ref())
                } else {
                    false
                }
            }
        }
    }

    /// Register LP token of given pool for given account.
    /// Fails if token_id is not a pool.
    #[payable]
    pub fn mft_register(&mut self, token_id: String, account_id: ValidAccountId) {
        self.assert_contract_running();
        let prev_storage = env::storage_usage();
        match parse_token_id(token_id) {
            TokenOrPool::Token(_) => env::panic(ERR110_INVALID_REGISTER.as_bytes()),
            TokenOrPool::Pool(pool_id) => {
                let mut pool = self.pools.get(pool_id).expect(ERR85_NO_POOL);
                pool.share_register(account_id.as_ref());
                self.pools.replace(pool_id, &pool);
                self.internal_check_storage(prev_storage);
            }
        }
    }

    /// Unregister LP token of given pool for given account.
    #[payable]
    pub fn mft_unregister(&mut self, token_id: String) {
        assert_one_yocto();
        self.assert_contract_running();
        let account_id = env::predecessor_account_id();
        let prev_storage = env::storage_usage();
        match parse_token_id(token_id) {
            TokenOrPool::Token(_) => env::panic(ERR111_INVALID_UNREGISTER.as_bytes()),
            TokenOrPool::Pool(pool_id) => {
                let mut pool = self.pools.get(pool_id).expect(ERR85_NO_POOL);
                pool.share_unregister(&account_id);
                self.pools.replace(pool_id, &pool);
                if prev_storage > env::storage_usage() {
                    let refund = (prev_storage - env::storage_usage()) as Balance * env::storage_byte_cost();
                    Promise::new(account_id).transfer(refund);
                }
            }
        }
    }

    /// Transfer one of internal tokens: LP or balances.
    /// `token_id` can either by account of the token or pool number.
    #[payable]
    pub fn mft_transfer(
        &mut self,
        token_id: String,
        receiver_id: ValidAccountId,
        amount: U128,
        memo: Option<String>,
    ) {
        assert_one_yocto();
        self.assert_contract_running();
        self.internal_mft_transfer(
            token_id,
            &env::predecessor_account_id(),
            receiver_id.as_ref(),
            Some(amount.0),
            memo,
        );
    }

    #[payable]
    pub fn mft_transfer_call(
        &mut self,
        token_id: String,
        receiver_id: ValidAccountId,
        amount: U128,
        memo: Option<String>,
        msg: String,
    ) -> PromiseOrValue<U128> {
        assert_one_yocto();
        self.assert_contract_running();
        let sender_id = env::predecessor_account_id();
        self.internal_mft_transfer(
            token_id.clone(),
            &sender_id,
            receiver_id.as_ref(),
            Some(amount.0),
            memo,
        );
        ext_share_token_receiver::mft_on_transfer(
            token_id.clone(),
            sender_id.clone(),
            amount,
            msg,
            receiver_id.as_ref(),
            NO_DEPOSIT,
            env::prepaid_gas() - GAS_FOR_FT_TRANSFER_CALL,
        )
        .then(ext_self::mft_resolve_transfer(
            token_id,
            sender_id,
            receiver_id.into(),
            amount,
            &env::current_account_id(),
            NO_DEPOSIT,
            GAS_FOR_RESOLVE_TRANSFER,
        ))
        .into()
    }

    #[payable]
    pub fn mft_transfer_all_call(
        &mut self,
        token_id: String,
        receiver_id: ValidAccountId,
        memo: Option<String>,
        msg: String,
    ) -> PromiseOrValue<U128> {
        assert_one_yocto();
        self.assert_contract_running();
        let sender_id = env::predecessor_account_id();
        let transfer_amount = self.internal_mft_transfer(
            token_id.clone(),
            &sender_id,
            receiver_id.as_ref(),
            None,
            memo,
        );
        ext_share_token_receiver::mft_on_transfer(
            token_id.clone(),
            sender_id.clone(),
            U128(transfer_amount),
            msg,
            receiver_id.as_ref(),
            NO_DEPOSIT,
            env::prepaid_gas() - GAS_FOR_FT_TRANSFER_CALL,
        )
        .then(ext_self::mft_resolve_transfer(
            token_id,
            sender_id,
            receiver_id.into(),
            U128(transfer_amount),
            &env::current_account_id(),
            NO_DEPOSIT,
            GAS_FOR_RESOLVE_TRANSFER,
        ))
        .into()
    }

    /// Returns how much was refunded back to the sender.
    /// If sender removed account in the meantime, the tokens are sent to the contract account.
    /// Tokens are never burnt.
    #[private]
    pub fn mft_resolve_transfer(
        &mut self,
        token_id: String,
        sender_id: AccountId,
        receiver_id: &AccountId,
        amount: U128,
    ) -> U128 {
        let unused_amount = match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Successful(value) => {
                if let Ok(unused_amount) = near_sdk::serde_json::from_slice::<U128>(&value) {
                    std::cmp::min(amount.0, unused_amount.0)
                } else {
                    amount.0
                }
            }
            PromiseResult::Failed => amount.0,
        };
        if unused_amount > 0 {
            let receiver_balance = self.internal_mft_balance(token_id.clone(), &receiver_id);
            if receiver_balance > 0 {
                let refund_amount = std::cmp::min(receiver_balance, unused_amount);
                
                let refund_to = if self.accounts.get(&sender_id).is_some() {
                    sender_id
                } else {
                    // If sender's account was deleted, we assume that they have also withdrew all the liquidity from pools.
                    // Funds are sent to the contract account.
                    env::current_account_id()
                };
                self.internal_mft_transfer(token_id, &receiver_id, &refund_to, Some(refund_amount), None);
            }
        }
        U128(unused_amount)
    }

    pub fn mft_metadata(&self, token_id: String) -> FungibleTokenMetadata {
        match parse_token_id(token_id) {
            TokenOrPool::Pool(pool_id) => {
                let pool = self.pools.get(pool_id).expect(ERR85_NO_POOL);
                let decimals = pool.get_share_decimal();
                FungibleTokenMetadata {
                    // [AUDIT_08]
                    spec: "mft-1.0.0".to_string(),
                    name: format!("ref-pool-{}", pool_id),
                    symbol: format!("REF-POOL-{}", pool_id),
                    icon: None,
                    reference: None,
                    reference_hash: None,
                    decimals,
                }
            },
            TokenOrPool::Token(_token_id) => unimplemented!(),
        }
    }
}
