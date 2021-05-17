//! View functions for the contract.

use std::collections::HashMap;

use near_sdk::json_types::{ValidAccountId, U128};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{near_bindgen, AccountId};

use crate::utils::SwapVolume;
use crate::*;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(crate = "near_sdk::serde")]
pub struct PoolInfo {
    /// Pool kind.
    pub pool_kind: String,
    /// List of tokens in the pool.
    pub token_account_ids: Vec<AccountId>,
    /// How much NEAR this contract has.
    pub amounts: Vec<U128>,
    /// Fee charged for swap.
    pub total_fee: u32,
    /// Total number of shares.
    pub shares_total_supply: U128,
}

impl From<Pool> for PoolInfo {
    fn from(pool: Pool) -> Self {
        let pool_kind = pool.kind();
        match pool {
            Pool::SimplePool(pool) => Self {
                pool_kind,
                token_account_ids: pool.token_account_ids,
                amounts: pool.amounts.into_iter().map(|a| U128(a)).collect(),
                total_fee: pool.total_fee,
                shares_total_supply: U128(pool.shares_total_supply),
            },
        }
    }
}

#[near_bindgen]
impl Contract {
    /// Returns semver of this contract.
    pub fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    /// Returns number of pools.
    pub fn get_number_of_pools(&self) -> u64 {
        self.pools.len()
    }

    /// Returns list of pools of given length from given start index.
    pub fn get_pools(&self, from_index: u64, limit: u64) -> Vec<PoolInfo> {
        (from_index..std::cmp::min(from_index + limit, self.pools.len()))
            .map(|index| self.get_pool(index))
            .collect()
    }

    /// Returns information about specified pool.
    pub fn get_pool(&self, pool_id: u64) -> PoolInfo {
        self.pools.get(pool_id).expect("ERR_NO_POOL").into()
    }

    /// Return total fee of the given pool.
    pub fn get_pool_fee(&self, pool_id: u64) -> u32 {
        self.pools.get(pool_id).expect("ERR_NO_POOL").get_fee()
    }

    /// Return volumes of the given pool.
    pub fn get_pool_volumes(&self, pool_id: u64) -> Vec<SwapVolume> {
        self.pools.get(pool_id).expect("ERR_NO_POOL").get_volumes()
    }

    /// Returns number of shares given account has in given pool.
    pub fn get_pool_shares(&self, pool_id: u64, account_id: ValidAccountId) -> U128 {
        self.pools
            .get(pool_id)
            .expect("ERR_NO_POOL")
            .share_balances(account_id.as_ref())
            .into()
    }

    /// Returns total number of shares in the given pool.
    pub fn get_pool_total_shares(&self, pool_id: u64) -> U128 {
        self.pools
            .get(pool_id)
            .expect("ERR_NO_POOL")
            .share_total_balance()
            .into()
    }

    /// Returns balances of the deposits for given user outside of any pools.
    /// Returns empty list if no tokens deposited.
    pub fn get_deposits(&self, account_id: ValidAccountId) -> HashMap<AccountId, U128> {
        self.deposited_amounts
            .get(account_id.as_ref())
            .map(|d| {
                d.tokens
                    .into_iter()
                    .map(|(acc, bal)| (acc, U128(bal)))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Returns balance of the deposit for given user outside of any pools.
    pub fn get_deposit(&self, account_id: ValidAccountId, token_id: ValidAccountId) -> U128 {
        self.internal_get_deposit(account_id.as_ref(), token_id.as_ref())
            .into()
    }

    /// Given specific pool, returns amount of token_out recevied swapping amount_in of token_in.
    pub fn get_return(
        &self,
        pool_id: u64,
        token_in: ValidAccountId,
        amount_in: U128,
        token_out: ValidAccountId,
    ) -> U128 {
        let pool = self.pools.get(pool_id).expect("ERR_NO_POOL");
        pool.get_return(token_in.as_ref(), amount_in.into(), token_out.as_ref())
            .into()
    }

    /// Get contract level whitelisted tokens.
    pub fn get_whitelisted_tokens(&self) -> Vec<AccountId> {
        self.whitelisted_tokens.to_vec()
    }

    /// Get specific user whitelisted tokens.
    pub fn get_user_whitelisted_tokens(&self, account_id: &AccountId) -> Vec<AccountId> {
        self.deposited_amounts
            .get(&account_id)
            .map(|d| d.tokens.keys().cloned().collect())
            .unwrap_or_default()
    }
}
