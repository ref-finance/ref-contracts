//! View functions for the contract.

use std::collections::HashMap;

use near_sdk::json_types::{ValidAccountId, U128};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{near_bindgen, AccountId};

use crate::utils::SwapVolume;
use crate::*;

#[derive(Serialize)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(Deserialize, Debug))]
pub struct ContractMetadata {
    pub version: String,
    pub owner: AccountId,
    pub guardians: Vec<AccountId>,
    pub pool_count: u64,
    pub state: RunningState,
    pub exchange_fee: u32,
    pub referral_fee: u32,
}

#[derive(Serialize, Deserialize, PartialEq)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug))]
pub struct RefStorageState {
    pub deposit: U128,
    pub usage: U128,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug, PartialEq))]
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
    pub amp: u64,
}

impl From<Pool> for PoolInfo {
    fn from(pool: Pool) -> Self {
        let pool_kind = pool.kind();
        match pool {
            Pool::SimplePool(pool) => Self {
                pool_kind,
                amp: 0,
                token_account_ids: pool.token_account_ids,
                amounts: pool.amounts.into_iter().map(|a| U128(a)).collect(),
                total_fee: pool.total_fee,
                shares_total_supply: U128(pool.shares_total_supply),
            },
            Pool::StableSwapPool(pool) => Self {
                pool_kind,
                amp: pool.get_amp(),
                amounts: pool.get_amounts().into_iter().map(|a| U128(a)).collect(),
                token_account_ids: pool.token_account_ids,
                total_fee: pool.total_fee,
                shares_total_supply: U128(pool.shares_total_supply),
            },
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug, PartialEq))]
pub struct StablePoolInfo {
    /// List of tokens in the pool.
    pub token_account_ids: Vec<AccountId>,
    pub decimals: Vec<u8>,
    /// backend tokens.
    pub amounts: Vec<U128>,
    /// backend tokens in comparable precision
    pub c_amounts: Vec<U128>,
    /// Fee charged for swap.
    pub total_fee: u32,
    /// Total number of shares.
    pub shares_total_supply: U128,
    pub amp: u64,
}

impl From<Pool> for StablePoolInfo {
    fn from(pool: Pool) -> Self {
        match pool {
            Pool::SimplePool(_) => unimplemented!(),
            Pool::StableSwapPool(pool) => Self {
                amp: pool.get_amp(),
                amounts: pool.get_amounts().into_iter().map(|a| U128(a)).collect(),
                decimals: pool.token_decimals,
                c_amounts: pool.c_amounts.into_iter().map(|a| U128(a)).collect(),
                token_account_ids: pool.token_account_ids,
                total_fee: pool.total_fee,
                shares_total_supply: U128(pool.shares_total_supply),
            },
        }
    }
}

#[near_bindgen]
impl Contract {

    /// Return contract basic info
    pub fn metadata(&self) -> ContractMetadata {
        ContractMetadata {
            version: env!("CARGO_PKG_VERSION").to_string(),
            owner: self.owner_id.clone(),
            guardians: self.guardians.to_vec(),
            pool_count: self.pools.len(),
            state: self.state.clone(),
            exchange_fee: self.exchange_fee,
            referral_fee: self.referral_fee,
        }
    }

    /// Only get guardians info
    pub fn get_guardians(&self) -> Vec<AccountId> {
        self.guardians.to_vec()
    }
    
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

    /// Returns stable pool information about specified pool.
    pub fn get_stable_pool(&self, pool_id: u64) -> StablePoolInfo {
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

    pub fn get_pool_share_price(&self, pool_id: u64) -> U128 {
        self.pools.get(pool_id).expect("ERR_NO_POOL").get_share_price().into()
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
        let wrapped_account = self.internal_get_account(account_id.as_ref());
        if let Some(account) = wrapped_account {
            account.get_tokens()
                .iter()
                .map(|token| (token.clone(), U128(account.get_balance(token).unwrap())))
                .collect()
        } else {
            HashMap::new()
        }
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
        pool.get_return(token_in.as_ref(), amount_in.into(), token_out.as_ref(), &AdminFees::new(self.exchange_fee))
            .into()
    }

    /// Get contract level whitelisted tokens.
    pub fn get_whitelisted_tokens(&self) -> Vec<AccountId> {
        self.whitelisted_tokens.to_vec()
    }

    /// Get specific user whitelisted tokens.
    pub fn get_user_whitelisted_tokens(&self, account_id: ValidAccountId) -> Vec<AccountId> {
        self.internal_get_account(account_id.as_ref())
            .map(|x| x.get_tokens())
            .unwrap_or_default()
    }

    /// Get user's storage deposit and needed in the account of current version
    pub fn get_user_storage_state(&self, account_id: ValidAccountId) -> Option<RefStorageState> {
        let acc = self.internal_get_account(account_id.as_ref());
        if let Some(account) = acc {
            Some(
                RefStorageState {
                    deposit: U128(account.near_amount),
                    usage: U128(account.storage_usage()),
                }
            )           
        } else {
            None
        }
    }

    pub fn predict_add_stable_liquidity(
        &self,
        pool_id: u64,
        amounts: &Vec<U128>,
    ) -> U128 {
        let pool = self.pools.get(pool_id).expect("ERR_NO_POOL");
        pool.predict_add_stable_liquidity(&amounts.into_iter().map(|x| x.0).collect(), &AdminFees::new(self.exchange_fee))
            .into()
    }

    pub fn predict_remove_liquidity(
        &self,
        pool_id: u64,
        shares: U128,
    ) -> Vec<U128> {
        let pool = self.pools.get(pool_id).expect("ERR_NO_POOL");
        pool.predict_remove_liquidity(shares.into()).into_iter().map(|x| U128(x)).collect()
    }

    pub fn predict_remove_liquidity_by_tokens(
        &self,
        pool_id: u64,
        amounts: &Vec<U128>,
    ) -> U128 {
        let pool = self.pools.get(pool_id).expect("ERR_NO_POOL");
        pool.predict_remove_liquidity_by_tokens(&amounts.into_iter().map(|x| x.0).collect(), &AdminFees::new(self.exchange_fee))
            .into()
    }
}
