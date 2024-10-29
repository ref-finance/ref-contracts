//! View functions for the contract.

use std::collections::HashMap;

use near_sdk::json_types::{ValidAccountId, U128, U64};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{near_bindgen, AccountId};
use crate::utils::{SwapVolume, TokenCache};
use crate::rated_swap::rate::Rate;
use crate::*;

#[derive(Serialize)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(Deserialize, Debug))]
pub struct ContractMetadata {
    pub version: String,
    pub owner: AccountId,
    pub boost_farm_id: AccountId,
    pub burrowland_id: AccountId,
    pub guardians: Vec<AccountId>,
    pub pool_count: u64,
    pub state: RunningState,
    pub admin_fee_bps: u32,
    pub cumulative_info_record_interval_sec: u32,
    pub wnear_id: Option<AccountId>,
    pub auto_whitelisted_postfix: HashSet<String>
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
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug))]
pub struct RatedTokenInfo {
    pub rate_type: String,
    pub rate_price: U128,
    pub last_update_ts: U64,
    pub is_valid: bool,
    pub extra_info: Option<String>
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug))]
pub struct DegenTokenInfo {
    pub degen_type: String,
    pub degen_price: U128,
    pub last_update_ts: U64,
    pub is_price_valid: bool,
    pub price_identifier: Option<pyth_oracle::PriceIdentifier>,
    pub decimals: Option<u8>
}

impl From<&Degen> for DegenTokenInfo {
    fn from(v: &Degen) -> Self {
        DegenTokenInfo {
            degen_type: v.get_type(),
            degen_price: v.get_price_info().stored_degen.into(),
            last_update_ts: v.get_price_info().degen_updated_at.into(),
            is_price_valid: v.is_price_valid(),
            price_identifier: if let Degen::PythOracle(d) = v {
                Some(d.price_identifier.clone())
            } else {
                None
            },
            decimals: if let Degen::PriceOracle(d) = v {
                Some(d.decimals)
            } else {
                None
            },
        }
    }
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

#[derive(Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug))]
pub struct AddLiquidityPrediction {
    pub need_amounts: Vec<U128>,
    pub mint_shares: U128,
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
            Pool::RatedSwapPool(pool) => Self {
                pool_kind,
                amp: pool.get_amp(),
                amounts: pool.get_amounts().into_iter().map(|a| U128(a)).collect(),
                token_account_ids: pool.token_account_ids,
                total_fee: pool.total_fee,
                shares_total_supply: U128(pool.shares_total_supply),
            },
            Pool::DegenSwapPool(pool) => Self {
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
pub enum PoolDetailInfo {
    SimplePoolInfo(SimplePoolInfo),
    StablePoolInfo(StablePoolInfo),
    RatedPoolInfo(RatedPoolInfo),
    DegenPoolInfo(DegenPoolInfo),
}

impl From<SimplePoolInfo> for PoolDetailInfo {
    fn from(pool: SimplePoolInfo) -> Self {
        PoolDetailInfo::SimplePoolInfo(pool)
    }
}

impl From<StablePoolInfo> for PoolDetailInfo {
    fn from(pool: StablePoolInfo) -> Self {
        PoolDetailInfo::StablePoolInfo(pool)
    }
}

impl From<RatedPoolInfo> for PoolDetailInfo {
    fn from(pool: RatedPoolInfo) -> Self {
        PoolDetailInfo::RatedPoolInfo(pool)
    }
}

impl From<DegenPoolInfo> for PoolDetailInfo {
    fn from(pool: DegenPoolInfo) -> Self {
        PoolDetailInfo::DegenPoolInfo(pool)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug, PartialEq))]
pub struct SimplePoolInfo {
    /// List of tokens in the pool.
    pub token_account_ids: Vec<AccountId>,
    /// How much NEAR this contract has.
    pub amounts: Vec<U128>,
    /// Fee charged for swap.
    pub total_fee: u32,
    /// Total number of shares.
    pub shares_total_supply: U128,
}

impl From<Pool> for SimplePoolInfo {
    fn from(pool: Pool) -> Self {
        match pool {
            Pool::SimplePool(pool) => Self {
                token_account_ids: pool.token_account_ids,
                amounts: pool.amounts.into_iter().map(|a| U128(a)).collect(),
                total_fee: pool.total_fee,
                shares_total_supply: U128(pool.shares_total_supply),
            },
            Pool::StableSwapPool(_) => unimplemented!(),
            Pool::RatedSwapPool(_) => unimplemented!(),
            Pool::DegenSwapPool(_) => unimplemented!(),
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
            Pool::RatedSwapPool(_) => unimplemented!(),
            Pool::DegenSwapPool(_) => unimplemented!(),
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug, PartialEq))]
pub struct RatedPoolInfo {
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
    pub rates: Vec<U128>,
}

impl From<Pool> for RatedPoolInfo {
    fn from(pool: Pool) -> Self {
        match pool {
            Pool::SimplePool(_) => unimplemented!(),
            Pool::StableSwapPool(_) => unimplemented!(),
            Pool::RatedSwapPool(pool) => Self {
                rates: pool.get_rates().into_iter().map(|a| U128(a)).collect(),
                amp: pool.get_amp(),
                amounts: pool.get_amounts().into_iter().map(|a| U128(a)).collect(),
                decimals: pool.token_decimals,
                c_amounts: pool.c_amounts.into_iter().map(|a| U128(a)).collect(),
                token_account_ids: pool.token_account_ids,
                total_fee: pool.total_fee,
                shares_total_supply: U128(pool.shares_total_supply),
                
            },
            Pool::DegenSwapPool(_) => unimplemented!(),
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug, PartialEq))]
pub struct DegenPoolInfo {
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
    pub degens: Vec<U128>,
}

impl From<Pool> for DegenPoolInfo {
    fn from(pool: Pool) -> Self {
        match pool {
            Pool::SimplePool(_) => unimplemented!(),
            Pool::StableSwapPool(_) => unimplemented!(),
            Pool::RatedSwapPool(_) => unimplemented!(),
            Pool::DegenSwapPool(pool) => Self {
                degens: pool.get_degens().into_iter().map(|a| U128(a)).collect(),
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

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug))]
pub struct ShadowRecordInfo {
    pub shadow_in_farm: U128,
    pub shadow_in_burrow: U128
}

impl From<ShadowRecord> for ShadowRecordInfo {
    fn from(v: ShadowRecord) -> Self {
        Self { 
            shadow_in_farm: U128(v.shadow_in_farm), 
            shadow_in_burrow: U128(v.shadow_in_burrow) 
        }
    }
}

impl From<VShadowRecord> for ShadowRecordInfo {
    fn from(v_shadow_record: VShadowRecord) -> Self {
        match v_shadow_record {
            VShadowRecord::Current(v) => v.into(),
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]

pub struct AccountBaseInfo {
    pub near_amount: U128,
    pub storage_used: U64,
}

#[near_bindgen]
impl Contract {

    /// Return contract basic info
    pub fn metadata(&self) -> ContractMetadata {
        ContractMetadata {
            version: env!("CARGO_PKG_VERSION").to_string(),
            owner: self.owner_id.clone(),
            boost_farm_id: self.boost_farm_id.clone(),
            burrowland_id: self.burrowland_id.clone(),
            guardians: self.guardians.to_vec(),
            pool_count: self.pools.len(),
            state: self.state.clone(),
            admin_fee_bps: self.admin_fee_bps,
            cumulative_info_record_interval_sec: self.cumulative_info_record_interval_sec,
            wnear_id: self.wnear_id.clone(),
            auto_whitelisted_postfix: self.auto_whitelisted_postfix.clone()
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
        self.pools.get(pool_id).expect(ERR85_NO_POOL).into()
    }

    /// Returns list of pools of given pool ids.
    pub fn get_pool_by_ids(&self, pool_ids: Vec<u64>) -> Vec<PoolInfo> {
        pool_ids.iter()
            .map(|index| self.get_pool(*index))
            .collect()
    }

    /// Returns list of pool detail infos of given length from given start index.
    pub fn get_pool_detail_infos(&self, from_index: u64, limit: u64) -> Vec<PoolDetailInfo> {
        (from_index..std::cmp::min(from_index + limit, self.pools.len()))
            .map(|index| self.get_pool_detail_info(index))
            .collect()
    }

    /// Returns pool detail info about specified pool.
    pub fn get_pool_detail_info(&self, pool_id: u64) -> PoolDetailInfo {
        let pool = self.pools.get(pool_id).expect(ERR85_NO_POOL);
        match &pool {
            Pool::SimplePool(_) => <Pool as Into<SimplePoolInfo>>::into(pool).into(),
            Pool::StableSwapPool(_) => <Pool as Into<StablePoolInfo>>::into(pool).into(),
            Pool::RatedSwapPool(_) => <Pool as Into<RatedPoolInfo>>::into(pool).into(),
            Pool::DegenSwapPool(_) => <Pool as Into<DegenPoolInfo>>::into(pool).into(),
        }
    }

    /// Returns list of pool detail infos of given pool ids.
    pub fn get_pool_detail_info_by_ids(&self, pool_ids: Vec<u64>) -> Vec<PoolDetailInfo> {
        pool_ids.into_iter()
            .map(|index| self.get_pool_detail_info(index))
            .collect()
    }

    /// Returns stable pool information about specified pool.
    pub fn get_stable_pool(&self, pool_id: u64) -> StablePoolInfo {
        self.pools.get(pool_id).expect(ERR85_NO_POOL).into()
    }

    /// Returns rated pool information about specified pool.
    pub fn get_rated_pool(&self, pool_id: u64) -> RatedPoolInfo {
        self.pools.get(pool_id).expect(ERR85_NO_POOL).into()
    }

    /// Returns degen pool information about specified pool.
    pub fn get_degen_pool(&self, pool_id: u64) -> DegenPoolInfo {
        self.pools.get(pool_id).expect(ERR85_NO_POOL).into()
    }

    /// Return total fee of the given pool.
    pub fn get_pool_fee(&self, pool_id: u64) -> u32 {
        self.pools.get(pool_id).expect(ERR85_NO_POOL).get_fee()
    }

    /// Return volumes of the given pool.
    pub fn get_pool_volumes(&self, pool_id: u64) -> Vec<SwapVolume> {
        self.pools.get(pool_id).expect(ERR85_NO_POOL).get_volumes()
    }

    pub fn get_pool_volumes_by_ids(&self, pool_ids: Vec<u64>) -> Vec<Vec<SwapVolume>> {
        pool_ids.iter()
            .map(|index| self.pools.get(*index).expect(ERR85_NO_POOL).get_volumes())
            .collect()
    }

    pub fn list_pool_volumes(&self, from_index: u64, limit: u64) -> Vec<Vec<SwapVolume>> {
        (from_index..std::cmp::min(from_index + limit, self.pools.len()))
            .map(|index| self.pools.get(index).expect(ERR85_NO_POOL).get_volumes())
            .collect()
    }

    pub fn get_pool_share_price(&self, pool_id: u64) -> U128 {
        self.pools.get(pool_id).expect(ERR85_NO_POOL).get_share_price().into()
    }

    /// Returns number of shares given account has in given pool.
    pub fn get_pool_shares(&self, pool_id: u64, account_id: ValidAccountId) -> U128 {
        self.pools
            .get(pool_id)
            .expect(ERR85_NO_POOL)
            .share_balances(account_id.as_ref())
            .into()
    }

    /// Returns total number of shares in the given pool.
    pub fn get_pool_total_shares(&self, pool_id: u64) -> U128 {
        self.pools
            .get(pool_id)
            .expect(ERR85_NO_POOL)
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

    pub fn get_tokens_paged(&self, account_id: ValidAccountId, from_index: Option<u64>, limit: Option<u64>) -> HashMap<AccountId, U128> {
        if let Some(account) = self.internal_get_account(account_id.as_ref()) {
            let keys = account.tokens.keys_as_vector();
            let from_index = from_index.unwrap_or(0);
            let limit = limit.unwrap_or(keys.len() as u64);
            (from_index..std::cmp::min(keys.len() as u64, from_index + limit))
                .map(|idx| {
                    let key = keys.get(idx).unwrap();
                    (key.clone(), account.tokens.get(&key).unwrap().into())
                })
                .collect()
        } else {
            Default::default()
        }
    }

    pub fn get_account_basic_info(&self, account_id: AccountId) -> Option<AccountBaseInfo> {
        let wrapped_account = self.internal_get_account(&account_id);
        if let Some(account) = wrapped_account {
            Some(AccountBaseInfo{
                near_amount: U128(account.near_amount),
                storage_used: U64(account.storage_used),
            })
        } else {
            None
        }
    }

    pub fn get_shadow_records(&self, account_id: ValidAccountId) -> HashMap<u64, ShadowRecordInfo> {
        let wrapped_account = self.internal_get_account(account_id.as_ref());
        if let Some(account) = wrapped_account {
            account.shadow_records
                .iter()
                .map(|(pool_id, vshadow_record)| (pool_id, vshadow_record.into()))
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
        let mut pool = self.pools.get(pool_id).expect(ERR85_NO_POOL);
        pool.swap(token_in.as_ref(), amount_in.into(), token_out.as_ref(), 0, AdminFees::new(self.admin_fee_bps), true).into()
    }

    /// Given a specific pool, returns the amount of token_in required to receive amount_out of token_out.
    pub fn get_return_by_output(
        &self,
        pool_id: u64,
        token_in: ValidAccountId,
        amount_out: U128,
        token_out: ValidAccountId,
    ) -> U128 {
        let mut pool = self.pools.get(pool_id).expect(ERR85_NO_POOL);
        pool.swap_by_output(token_in.as_ref(), amount_out.into(), token_out.as_ref(), None, AdminFees::new(self.admin_fee_bps), true).into()
    }

    /// List referrals
    pub fn list_referrals(&self, from_index: Option<u64>, limit: Option<u64>) -> HashMap<AccountId, u32> {
        let keys = self.referrals.keys_as_vector();
        let from_index = from_index.unwrap_or(0);
        let limit = limit.unwrap_or(keys.len());

        (from_index..std::cmp::min(from_index + limit, keys.len()))
            .map(|index| {
                (
                    keys.get(index).unwrap(),
                    self.referrals.get(&keys.get(index).unwrap()).unwrap()
                )
            })
            .collect()
    }

    /// Get frozenlist tokens.
    pub fn get_frozenlist_tokens(&self) -> Vec<AccountId> {
        self.frozen_tokens.to_vec()
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

    ///
    pub fn predict_add_simple_liquidity(
        &self,
        pool_id: u64,
        amounts: &Vec<U128>,
    ) -> AddLiquidityPrediction {
        let mut pool = self.pools.get(pool_id).expect(ERR85_NO_POOL);
        let mut amounts = amounts.iter().map(|v| v.0).collect();
        let mint_shares = pool.add_liquidity(&String::from("@view"), &mut amounts, true);
        AddLiquidityPrediction {
            need_amounts: amounts.iter().map(|v| U128(*v)).collect(),
            mint_shares: U128(mint_shares)
        }
    }

    pub fn predict_add_stable_liquidity(
        &self,
        pool_id: u64,
        amounts: &Vec<U128>,
    ) -> U128 {
        let mut pool = self.pools.get(pool_id).expect(ERR85_NO_POOL);
        let amounts = amounts.iter().map(|v| v.0).collect();
        pool.add_stable_liquidity(&String::from("@view"), &amounts, 0, AdminFees::new(self.admin_fee_bps), true).into()
    }

    pub fn predict_remove_liquidity(
        &self,
        pool_id: u64,
        shares: U128,
    ) -> Vec<U128> {
        let mut pool = self.pools.get(pool_id).expect(ERR85_NO_POOL);
        pool.remove_liquidity(&String::from("@view"), shares.into(), vec![0; pool.tokens().len()], true).into_iter().map(|x| U128(x)).collect()
    }

    pub fn predict_remove_liquidity_by_tokens(
        &self,
        pool_id: u64,
        amounts: &Vec<U128>,
    ) -> U128 {
        let mut pool = self.pools.get(pool_id).expect(ERR85_NO_POOL);
        let amounts = amounts.iter().map(|v| v.0).collect();
        pool.remove_liquidity_by_tokens(&String::from("@view"), amounts, u128::MAX, AdminFees::new(self.admin_fee_bps), true).into()
    }

    pub fn list_rated_tokens(&self) -> HashMap<String, RatedTokenInfo> {
        // read from storage
        let rates: HashMap<String, Rate> = if let Some(content) = env::storage_read(RATE_STORAGE_KEY.as_bytes()) {
            HashMap::try_from_slice(&content).expect("deserialize failed.")
        } else {
            HashMap::new()
        };
        rates
        .iter()
        .map(|(k, v)| {
            match v {
                Rate::Sfrax(r) => (k.clone(), 
                    RatedTokenInfo {
                        rate_type: v.get_type(),
                        rate_price: v.get().into(),
                        last_update_ts: v.last_update_ts().into(),
                        is_valid: v.are_actual(),
                        extra_info: Some(near_sdk::serde_json::to_string(&r.extra_info).unwrap())
                    }),
                _ => (k.clone(), 
                    RatedTokenInfo {
                        rate_type: v.get_type(),
                        rate_price: v.get().into(),
                        last_update_ts: v.last_update_ts().into(),
                        is_valid: v.are_actual(),
                        extra_info: None
                    })
            }
            
        })
        .collect()
    }

    /// Batch retrieve DegenTokenInfo based on the specified token ID list.
    ///
    /// # Arguments
    ///
    /// * `token_ids` - List of token IDs.
    pub fn batch_get_degen_tokens(&self, token_ids: Vec<ValidAccountId>) -> HashMap<String, Option<DegenTokenInfo>> {
        let degens = read_degens_from_storage();
        token_ids.into_iter().map(|t| {
            let token_id: AccountId = t.into();
            (token_id.clone(), degens.get(&token_id).map(|v| v.into()))
        }).collect()
    }

    pub fn list_degen_tokens(&self) -> HashMap<String, DegenTokenInfo> {
        let degens = read_degens_from_storage();
        degens
        .iter()
        .map(|(k, v)| (
            k.clone(),  
            v.into()
        ))
        .collect()
    }

    pub fn list_degen_oracle_configs(&self) -> HashMap<String, DegenOracleConfig> {
        read_degen_oracle_configs_from_storage()
    }

    /// get predicted result of add_liquidity for a given rated token price
    pub fn predict_add_rated_liquidity(
        &self,
        pool_id: u64,
        amounts: &Vec<U128>,
        rates: &Option<Vec<U128>>,
    ) -> U128 {
        let pool = self.pools.get(pool_id).expect(ERR85_NO_POOL);
        let rates = match rates {
            Some(rates) => Some(rates.into_iter().map(|x| x.0).collect()),
            _ => None
        };
        pool.predict_add_rated_liquidity(
            &amounts.into_iter().map(|x| x.0).collect(),
            &rates,
            &AdminFees::new(self.admin_fee_bps)
        ).into()
    }

    /// get predicted result of add_liquidity for a given degen token price
    pub fn predict_add_degen_liquidity(
        &self,
        pool_id: u64,
        amounts: &Vec<U128>,
        degens: &Option<Vec<U128>>,
    ) -> U128 {
        let pool = self.pools.get(pool_id).expect(ERR85_NO_POOL);
        let degens = match degens {
            Some(degens) => Some(degens.into_iter().map(|x| x.0).collect()),
            _ => None
        };
        pool.predict_add_degen_liquidity(
            &amounts.into_iter().map(|x| x.0).collect(),
            &degens,
            &AdminFees::new(self.admin_fee_bps)
        ).into()
    }

    /// get predicted result of remove_liquidity_by_tokens for a given rated token price 
    pub fn predict_remove_rated_liquidity_by_tokens(
        &self,
        pool_id: u64,
        amounts: &Vec<U128>,
        rates: &Option<Vec<U128>>,
    ) -> U128 {
        let pool = self.pools.get(pool_id).expect(ERR85_NO_POOL);
        let rates = match rates {
            Some(rates) => Some(rates.into_iter().map(|x| x.0).collect()),
            _ => None
        };
        pool.predict_remove_rated_liquidity_by_tokens(&amounts.into_iter().map(|x| x.0).collect(), &rates, &AdminFees::new(self.admin_fee_bps))
            .into()
    }

    /// get predicted result of remove_liquidity_by_tokens for a given degen token price 
    pub fn predict_remove_degen_liquidity_by_tokens(
        &self,
        pool_id: u64,
        amounts: &Vec<U128>,
        degens: &Option<Vec<U128>>,
    ) -> U128 {
        let pool = self.pools.get(pool_id).expect(ERR85_NO_POOL);
        let degens = match degens {
            Some(degens) => Some(degens.into_iter().map(|x| x.0).collect()),
            _ => None
        };
        pool.predict_remove_degen_liquidity_by_tokens(&amounts.into_iter().map(|x| x.0).collect(), &degens, &AdminFees::new(self.admin_fee_bps))
            .into()
    }

    /// get predicted swap result of a rated stable swap pool for given rated token price 
    pub fn get_rated_return(
        &self,
        pool_id: u64,
        token_in: ValidAccountId,
        amount_in: U128,
        token_out: ValidAccountId,
        rates: &Option<Vec<U128>>,
    ) -> U128 {
        let pool = self.pools.get(pool_id).expect(ERR85_NO_POOL);
        let rates = match rates {
            Some(rates) => Some(rates.into_iter().map(|x| x.0).collect()),
            _ => None
        };
        pool.get_rated_return(token_in.as_ref(), amount_in.into(), token_out.as_ref(), &rates, &AdminFees::new(self.admin_fee_bps))
            .into()
    }

    /// get predicted swap result of a rated stable swap pool for given degen token price 
    pub fn get_degen_return(
        &self,
        pool_id: u64,
        token_in: ValidAccountId,
        amount_in: U128,
        token_out: ValidAccountId,
        degens: &Option<Vec<U128>>,
    ) -> U128 {
        let pool = self.pools.get(pool_id).expect(ERR85_NO_POOL);
        let degens = match degens {
            Some(degens) => Some(degens.into_iter().map(|x| x.0).collect()),
            _ => None
        };
        pool.get_degen_return(token_in.as_ref(), amount_in.into(), token_out.as_ref(), &degens, &AdminFees::new(self.admin_fee_bps))
            .into()
    }

    pub fn predict_swap_actions(
        &self, 
        token_deposit: HashMap<AccountId, U128>,
        actions: Vec<Action>,
    ) -> HashMap<AccountId, U128> {
        let mut pool_cache = HashMap::new();
        let mut token_cache = TokenCache(token_deposit.into_iter().map(|(k, v)| (k, v.into())).collect());

        self.internal_execute_actions_by_cache(
            &mut pool_cache,
            &mut token_cache,
            &None,
            &actions,
            ActionResult::None,
        );
        token_cache.0.into_iter().map(|(k, v)| (k, v.into())).collect()
    }

    pub fn batch_predict_swap_actions(
        &self, 
        batch_token_deposit: Vec<HashMap<AccountId, U128>>,
        batch_actions: Vec<Vec<Action>>,
    ) -> Vec<HashMap<AccountId, U128>> {
        let mut results = vec![];
        for index in 0..batch_actions.len() {
            results.push(self.predict_swap_actions(batch_token_deposit[index].clone(), batch_actions[index].clone()));
        }
        results
    }

    pub fn predict_hot_zap(
        &self, 
        referral_id: Option<ValidAccountId>,
        account_id: Option<ValidAccountId>,
        token_in: ValidAccountId,
        amount_in: U128,
        hot_zap_actions: Vec<Action>,
        add_liquidity_infos: Vec<token_receiver::AddLiquidityInfo>
    ) -> Option<(Vec<AddLiquidityPrediction>, HashMap<AccountId, U128>)> {
        if hot_zap_actions.is_empty() || add_liquidity_infos.is_empty() {
            return None
        }
        let all_tokens = self.get_hot_zap_tokens(&hot_zap_actions, &add_liquidity_infos);
        if let Some(account_id) = account_id {
            let account = self.internal_unwrap_account(&account_id.into());      
            for token_id in all_tokens.iter() {
                assert!(
                    self.is_whitelisted_token(token_id) 
                        || account.get_balance(token_id).is_some(),
                    "{}",
                    ERR12_TOKEN_NOT_WHITELISTED
                );
            }
        } else {
            for token_id in all_tokens.iter() {
                assert!(
                    self.is_whitelisted_token(token_id),
                    "{}",
                    ERR12_TOKEN_NOT_WHITELISTED
                );
            }
        }
        self.assert_no_frozen_tokens(&all_tokens);
        let mut pool_cache = HashMap::new();
        let mut add_liquidity_predictions = vec![];
        let mut token_cache = TokenCache::new();
        token_cache.add(&token_in.into(), amount_in.0);
        let view_account_id: AccountId = "@view".to_string();

        let referral_id = referral_id.map(|x| x.to_string());
        let referral_info :Option<(AccountId, u32)> = referral_id
            .as_ref().and_then(|rid| self.referrals.get(&rid))
            .map(|fee| (referral_id.unwrap().into(), fee));

        self.internal_execute_actions_by_cache(
            &mut pool_cache,
            &mut token_cache,
            &referral_info,
            &hot_zap_actions,
            match hot_zap_actions[0] { 
                Action::Swap(_) => ActionResult::Amount(amount_in),
                Action::SwapByOutput(_) => ActionResult::None,
            },
        );

        for add_liquidity_info in add_liquidity_infos {
            let mut pool = pool_cache.remove(&add_liquidity_info.pool_id).unwrap_or(self.pools.get(add_liquidity_info.pool_id).expect(ERR85_NO_POOL));
            
            let tokens_in_pool = match &pool {
                Pool::SimplePool(p) => p.token_account_ids.clone(),
                Pool::RatedSwapPool(p) => p.token_account_ids.clone(),
                Pool::StableSwapPool(p) => p.token_account_ids.clone(),
                Pool::DegenSwapPool(p) => p.token_account_ids.clone(),
            };
            
            let mut add_liquidity_amounts = add_liquidity_info.amounts.iter().map(|v| v.0).collect();
            
            let shares = match pool {
                Pool::SimplePool(_) => {
                    let shares = pool.add_liquidity(
                        &view_account_id,
                        &mut add_liquidity_amounts,
                        true
                    );
                    shares
                },
                Pool::StableSwapPool(_) | Pool::RatedSwapPool(_) | Pool::DegenSwapPool(_) => {
                    let shares = pool.add_stable_liquidity(
                        &view_account_id,
                        &add_liquidity_amounts,
                        0,
                        AdminFees::new(self.admin_fee_bps),
                        true
                    );
                    shares
                }
            };
            pool.assert_tvl_not_exceed_limit(add_liquidity_info.pool_id);
            
            add_liquidity_predictions.push(
                AddLiquidityPrediction {
                    need_amounts: add_liquidity_amounts.iter().map(|v| U128(*v)).collect(),
                    mint_shares: U128(shares)
                }
            );

            for (cost_token_id, cost_amount) in tokens_in_pool.iter().zip(add_liquidity_amounts.into_iter()) {
                token_cache.sub(cost_token_id, cost_amount);
            }

            pool_cache.insert(add_liquidity_info.pool_id, pool);
        }
        
        Some((add_liquidity_predictions, token_cache.into()))
    }

    pub fn get_degen_pool_tvl(&self, pool_id: u64) -> U128 {
        self.pools.get(pool_id).expect(ERR85_NO_POOL).get_tvl().into()
    }

    pub fn get_pool_limit_by_pool_id(&self, pool_id: u64) -> Option<VPoolLimitInfo> {
        read_pool_limit_from_storage().get(&pool_id)
    }

    pub fn get_pool_limit_paged(&self, from_index: Option<u64>, limit: Option<u64>) -> HashMap<u64, VPoolLimitInfo> {
        let pool_limit = read_pool_limit_from_storage();
        let keys = pool_limit.keys_as_vector();
        let from_index = from_index.unwrap_or(0);
        let limit = limit.unwrap_or(keys.len() as u64);
        (from_index..std::cmp::min(keys.len() as u64, from_index + limit))
            .map(|idx| {
                let key = keys.get(idx).unwrap();
                (key.clone(), pool_limit.get(&key).unwrap())
            })
            .collect()
    }
}
