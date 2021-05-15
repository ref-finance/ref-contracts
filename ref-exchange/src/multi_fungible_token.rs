use near_contract_standards::fungible_token::metadata::{FungibleTokenMetadata, FT_METADATA_SPEC};
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

fn parse_token_id(token_id: String) -> TokenOrPool {
    if let Ok(pool_id) = str::parse::<u64>(&token_id) {
        TokenOrPool::Pool(pool_id)
    } else {
        TokenOrPool::Token(token_id)
    }
}

#[near_bindgen]
impl Contract {
    fn internal_mft_transfer(
        &mut self,
        token_id: String,
        sender_id: &AccountId,
        receiver_id: &AccountId,
        amount: u128,
        memo: Option<String>,
    ) {
        match parse_token_id(token_id) {
            TokenOrPool::Pool(pool_id) => {
                let mut pool = self.pools.get(pool_id).expect("ERR_NO_POOL");
                pool.share_transfer(sender_id, receiver_id, amount);
                self.pools.replace(pool_id, &pool);
                log!(
                    "Transfer shares {} pool: {} from {} to {}",
                    pool_id,
                    amount,
                    sender_id,
                    receiver_id
                );
            }
            TokenOrPool::Token(token_id) => {
                let mut sender_account = self
                    .accounts
                    .get(&sender_id)
                    .expect(ERR10_ACC_NOT_REGISTERED);
                let mut receiver_account = self
                    .accounts
                    .get(receiver_id)
                    .expect(ERR10_ACC_NOT_REGISTERED);
                sender_account.withdraw(&token_id, amount);
                receiver_account.deposit(&token_id, amount);
                self.accounts.insert(&sender_id, &sender_account);
                self.accounts.insert(&receiver_id, &receiver_account);
                log!(
                    "Transfer {}: {} from {} to {}",
                    token_id,
                    amount,
                    sender_id,
                    receiver_id
                );
            }
        }
        if let Some(memo) = memo {
            log!("Memo: {}", memo);
        }
    }

    fn internal_mft_balance(&self, token_id: String, account_id: &AccountId) -> Balance {
        match parse_token_id(token_id) {
            TokenOrPool::Pool(pool_id) => {
                let pool = self.pools.get(pool_id).expect("ERR_NO_POOL");
                pool.share_balances(account_id)
            }
            TokenOrPool::Token(token_id) => self.internal_get_deposit(account_id, &token_id),
        }
    }

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
                let pool = self.pools.get(pool_id).expect("ERR_NO_POOL");
                U128(pool.share_total_balance())
            }
            TokenOrPool::Token(_token_id) => unimplemented!(),
        }
    }

    /// Register LP token of given pool for given account.
    /// Fails if token_id is not a pool.
    #[payable]
    pub fn mft_register(&mut self, token_id: String, account_id: ValidAccountId) {
        let prev_storage = env::storage_usage();
        match parse_token_id(token_id) {
            TokenOrPool::Token(_) => env::panic(b"ERR_INVALID_REGISTER"),
            TokenOrPool::Pool(pool_id) => {
                let mut pool = self.pools.get(pool_id).expect("ERR_NO_POOL");
                pool.share_register(account_id.as_ref());
                self.pools.replace(pool_id, &pool);
                self.internal_check_storage(prev_storage);
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
        self.internal_mft_transfer(
            token_id,
            &env::predecessor_account_id(),
            receiver_id.as_ref(),
            amount.0,
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
        let sender_id = env::predecessor_account_id();
        self.internal_mft_transfer(
            token_id.clone(),
            &sender_id,
            receiver_id.as_ref(),
            amount.0,
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

    /// Returns how much was refunded back to the sender.
    /// If sender removed account in the meantime, the tokens are sent to the owner account.
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
                // If sender's account was deleted, we assume that they have also withdrew all the liquidity from pools.
                // Funds are sent to the owner account.
                let refund_to = if self.accounts.get(&sender_id).is_some() {
                    sender_id
                } else {
                    self.owner_id.clone()
                };
                self.internal_mft_transfer(token_id, &receiver_id, &refund_to, refund_amount, None);
            }
        }
        U128(unused_amount)
    }

    pub fn mft_metadata(&self, token_id: String) -> FungibleTokenMetadata {
        match parse_token_id(token_id) {
            TokenOrPool::Pool(pool_id) => FungibleTokenMetadata {
                spec: FT_METADATA_SPEC.to_string(),
                name: format!("ref-pool-{}", pool_id),
                symbol: format!("REF-POOL-{}", pool_id),
                icon: None,
                reference: None,
                reference_hash: None,
                decimals: 24,
            },
            TokenOrPool::Token(_token_id) => unimplemented!(),
        }
    }
}
