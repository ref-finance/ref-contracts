use std::convert::TryInto;
use std::fmt;
use std::collections::{HashMap, HashSet};

use degen_swap::degen::{global_get_degen, global_set_degen, DegenTrait};
use degen_swap::DegenSwapPool;
use near_contract_standards::storage_management::{
    StorageBalance, StorageBalanceBounds, StorageManagement,
};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LookupMap, UnorderedSet, Vector, UnorderedMap};
use near_sdk::json_types::{ValidAccountId, U128};
use near_sdk::{
    assert_one_yocto, env, log, near_bindgen, AccountId, Balance, PanicOnDefault, Promise,
    PromiseResult, StorageUsage, BorshStorageKey, PromiseOrValue, ext_contract, Gas
};
use utils::{NO_DEPOSIT, GAS_FOR_BASIC_OP};

use crate::account_deposit::*;
pub use crate::action::{SwapAction, SwapByOutputAction, Action, ActionResult, get_tokens_in_actions, assert_all_same_action_type};
use crate::errors::*;
use crate::admin_fee::AdminFees;
use crate::pool::Pool;
use crate::simple_pool::SimplePool;
use crate::stable_swap::StableSwapPool;
use crate::rated_swap::{RatedSwapPool, rate::{RateTrait, global_get_rate, global_set_rate}};
pub use crate::utils::{check_token_duplicates, pair_rated_price_to_vec_u8, TokenCache, SwapVolume};
pub use crate::custom_keys::*;
pub use crate::views::{PoolInfo, ShadowRecordInfo, RatedPoolInfo, StablePoolInfo, ContractMetadata, RatedTokenInfo, DegenTokenInfo, AddLiquidityPrediction, RefStorageState};
pub use crate::token_receiver::{AddLiquidityInfo, VIRTUAL_ACC};
pub use crate::shadow_actions::*;
pub use crate::unit_lpt_cumulative_infos::*;
pub use crate::oracle::*;
pub use crate::degen_swap::*;
pub use crate::pool_limit_info::*;
pub use crate::client_echo_limit::*;
pub use crate::swap_volume::*;

mod account_deposit;
mod account_lostfound;
mod action;
mod errors;
mod admin_fee;
mod legacy;
mod multi_fungible_token;
mod owner;
mod pool;
mod simple_pool;
mod stable_swap;
mod rated_swap;
mod degen_swap;
mod oracle;
mod storage_impl;
mod token_receiver;
mod utils;
mod views;
mod custom_keys;
mod shadow_actions;
mod unit_lpt_cumulative_infos;
mod pool_limit_info;
mod client_echo_limit;
mod donation;
mod event;
mod swap_volume;

near_sdk::setup_alloc!();

#[derive(BorshStorageKey, BorshSerialize)]
pub(crate) enum StorageKey {
    Pools,
    Accounts,
    Shares { pool_id: u32 },
    Whitelist,
    Guardian,
    AccountTokens {account_id: AccountId},
    Frozenlist,
    Referral,
    ShadowRecord {account_id: AccountId},
    UnitShareCumulativeInfo,
    PoolLimit,
    ClientEchoTokenIdWhitelistItem,
    ClientEchoSenderIdWhitelistItem,
    SecureSenderWhitelistItem,
    LostfoundAccounts,
    LostfoundAccountTokens {account_id: AccountId},
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Eq, PartialEq, Clone)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug))]
pub enum RunningState {
    Running, Paused
}

impl fmt::Display for RunningState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RunningState::Running => write!(f, "Running"),
            RunningState::Paused => write!(f, "Paused"),
        }
    }
}

#[ext_contract(ext_self)]
pub trait SelfCallbacks {
    fn update_token_rate_callback(&mut self, token_id: AccountId);
    fn update_degen_token_price_callback(&mut self, token_id: AccountId);
    fn batch_update_degen_token_by_price_oracle_callback(&mut self, token_id_decimals_map: HashMap<AccountId, u8>);
    fn batch_update_degen_token_by_pyth_oracle_callback(&mut self, price_id_token_id_map: HashMap<pyth_oracle::PriceIdentifier, AccountId>);
}

#[near_bindgen]
#[derive(BorshSerialize, BorshDeserialize, PanicOnDefault)]
pub struct Contract {
    /// Account of the owner.
    owner_id: AccountId,
    /// Account of the boost_farm contract.
    boost_farm_id: AccountId,
    /// Account of the burrowland contract.
    burrowland_id: AccountId,
    /// Admin fee rate in total fee.
    admin_fee_bps: u32,
    /// List of all the pools.
    pools: Vector<Pool>,
    /// Accounts registered, keeping track all the amounts deposited, storage and more.
    accounts: LookupMap<AccountId, VAccount>,
    /// Set of whitelisted tokens by "owner".
    whitelisted_tokens: UnorderedSet<AccountId>,
    /// Set of guardians.
    guardians: UnorderedSet<AccountId>,
    /// Running state
    state: RunningState,
    /// Set of frozenlist tokens
    frozen_tokens: UnorderedSet<AccountId>,
    /// Map of referrals
    referrals: UnorderedMap<AccountId, u32>,

    cumulative_info_record_interval_sec: u32,
    unit_share_cumulative_infos: UnorderedMap<u64, VUnitShareCumulativeInfo>,

    wnear_id: Option<AccountId>,
    auto_whitelisted_postfix: HashSet<String>
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new(owner_id: ValidAccountId, boost_farm_id: ValidAccountId, burrowland_id: ValidAccountId, exchange_fee: u32, referral_fee: u32) -> Self {
        Self {
            owner_id: owner_id.as_ref().clone(),
            boost_farm_id: boost_farm_id.as_ref().clone(),
            burrowland_id: burrowland_id.as_ref().clone(),
            admin_fee_bps: exchange_fee + referral_fee,
            pools: Vector::new(StorageKey::Pools),
            accounts: LookupMap::new(StorageKey::Accounts),
            whitelisted_tokens: UnorderedSet::new(StorageKey::Whitelist),
            guardians: UnorderedSet::new(StorageKey::Guardian),
            state: RunningState::Running,
            frozen_tokens: UnorderedSet::new(StorageKey::Frozenlist),
            referrals: UnorderedMap::new(StorageKey::Referral),
            cumulative_info_record_interval_sec: 12 * 60, // 12 min
            unit_share_cumulative_infos: UnorderedMap::new(StorageKey::UnitShareCumulativeInfo),
            wnear_id: None,
            auto_whitelisted_postfix: HashSet::new()
        }
    }

    /// Adds new "Simple Pool" with given tokens and given fee.
    /// Attached NEAR should be enough to cover the added storage.
    #[payable]
    pub fn add_simple_pool(&mut self, tokens: Vec<ValidAccountId>, fee: u32) -> u64 {
        self.assert_contract_running();
        check_token_duplicates(&tokens);
        self.internal_add_pool(Pool::SimplePool(SimplePool::new(
            self.pools.len() as u32,
            tokens,
            fee,
        )))
    }

    /// Adds new "Stable Pool" with given tokens, decimals, fee and amp.
    /// It is limited to owner or guardians, cause a complex and correct config is needed.
    /// tokens: pool tokens in this stable swap.
    /// decimals: each pool tokens decimal, needed to make them comparable.
    /// fee: total fee of the pool, admin fee is inclusive.
    /// amp_factor: algorithm parameter, decide how stable the pool will be.
    #[payable]
    pub fn add_stable_swap_pool(
        &mut self,
        tokens: Vec<ValidAccountId>,
        decimals: Vec<u8>,
        fee: u32,
        amp_factor: u64,
    ) -> u64 {
        assert!(self.is_owner_or_guardians(), "{}", ERR100_NOT_ALLOWED);
        assert!(tokens.len() == decimals.len(), "The number of tokens is inconsistent with the number of decimals.");
        check_token_duplicates(&tokens);
        self.internal_add_pool(Pool::StableSwapPool(StableSwapPool::new(
            self.pools.len() as u32,
            tokens,
            decimals,
            amp_factor as u128,
            fee,
        )))
    }

    ///
    #[payable]
    pub fn add_rated_swap_pool(
        &mut self,
        tokens: Vec<ValidAccountId>,
        decimals: Vec<u8>,
        fee: u32,
        amp_factor: u64,
    ) -> u64 {
        assert!(self.is_owner_or_guardians(), "{}", ERR100_NOT_ALLOWED);
        assert!(tokens.len() == decimals.len(), "The number of tokens is inconsistent with the number of decimals.");
        check_token_duplicates(&tokens);
        self.internal_add_pool(Pool::RatedSwapPool(RatedSwapPool::new(
            self.pools.len() as u32,
            tokens,
            decimals,
            amp_factor as u128,
            fee,
        )))
    }

    #[payable]
    pub fn add_degen_swap_pool(
        &mut self,
        tokens: Vec<ValidAccountId>,
        decimals: Vec<u8>,
        fee: u32,
        amp_factor: u64,
    ) -> u64 {
        assert!(self.is_owner_or_guardians(), "{}", ERR100_NOT_ALLOWED);
        assert!(tokens.len() == decimals.len(), "The number of tokens is inconsistent with the number of decimals.");
        check_token_duplicates(&tokens);
        self.internal_add_pool(Pool::DegenSwapPool(DegenSwapPool::new(
            self.pools.len() as u32,
            tokens,
            decimals,
            amp_factor as u128,
            fee,
        )))
    }

    #[payable]
    pub fn execute_actions_in_va(
        &mut self,
        use_tokens: HashMap<AccountId, U128>,
        actions: Vec<Action>,
        referral_id: Option<ValidAccountId>,
        skip_degen_price_sync: Option<bool>,
    ) -> HashMap<AccountId, U128> {
        self.assert_contract_running();
        assert_ne!(actions.len(), 0, "{}", ERR72_AT_LEAST_ONE_SWAP);
        let sender_id = env::predecessor_account_id();
        let mut account = self.internal_unwrap_account(&sender_id);
        // Validate that all tokens are whitelisted if no deposit (e.g. trade with access key).
        if env::attached_deposit() == 0 {
            for action in &actions {
                for token in action.tokens() {
                    assert!(
                        account.get_balance(&token).is_some() 
                            || self.is_whitelisted_token(&token),
                        "{}",
                        // [AUDIT_05]
                        ERR27_DEPOSIT_NEEDED
                    );
                }
            }
        }

        let mut virtual_account: Account = Account::new(&String::from(VIRTUAL_ACC));
        let referral_info :Option<(AccountId, u32)> = referral_id
            .as_ref().and_then(|rid| self.referrals.get(rid.as_ref()))
            .map(|fee| (referral_id.unwrap().into(), fee));
        for (use_token, use_amount) in use_tokens.iter() {
            account.withdraw(use_token, use_amount.0);
            virtual_account.deposit(use_token, use_amount.0);
        }
        let _ = self.internal_execute_actions(
            &mut virtual_account,
            &referral_info,
            &actions,
            ActionResult::None,
            skip_degen_price_sync.unwrap_or(false)
        );
        let mut result = HashMap::new();
        for (token, amount) in virtual_account.tokens.to_vec() {
            if amount > 0 {
                account.deposit(&token, amount);
                result.insert(token, amount.into());
            }
        }
        
        virtual_account.tokens.clear();
        self.internal_save_account(&sender_id, account);
        result
    }

    /// [AUDIT_03_reject(NOPE action is allowed by design)]
    /// [AUDIT_04]
    /// Executes generic set of actions.
    /// If referrer provided, pays referral_fee to it.
    /// If no attached deposit, outgoing tokens used in swaps must be whitelisted.
    #[payable]
    pub fn execute_actions(
        &mut self,
        actions: Vec<Action>,
        referral_id: Option<ValidAccountId>,
        skip_degen_price_sync: Option<bool>,
    ) -> ActionResult {
        self.assert_contract_running();
        assert_ne!(actions.len(), 0, "{}", ERR72_AT_LEAST_ONE_SWAP);
        let sender_id = env::predecessor_account_id();
        let mut account = self.internal_unwrap_account(&sender_id);
        // Validate that all tokens are whitelisted if no deposit (e.g. trade with access key).
        if env::attached_deposit() == 0 {
            for action in &actions {
                for token in action.tokens() {
                    assert!(
                        account.get_balance(&token).is_some() 
                            || self.is_whitelisted_token(&token),
                        "{}",
                        // [AUDIT_05]
                        ERR27_DEPOSIT_NEEDED
                    );
                }
            }
        }

        let referral_info :Option<(AccountId, u32)> = referral_id
            .as_ref().and_then(|rid| self.referrals.get(rid.as_ref()))
            .map(|fee| (referral_id.unwrap().into(), fee));
        
        let result =
            self.internal_execute_actions(&mut account, &referral_info, &actions, ActionResult::None, skip_degen_price_sync.unwrap_or(false));
        self.internal_save_account(&sender_id, account);
        result
    }

    /// Execute set of swap actions between pools.
    /// If referrer provided, pays referral_fee to it.
    /// If no attached deposit, outgoing tokens used in swaps must be whitelisted.
    #[payable]
    pub fn swap(&mut self, actions: Vec<SwapAction>, referral_id: Option<ValidAccountId>, skip_degen_price_sync: Option<bool>) -> U128 {
        U128(
            self.execute_actions(
                actions
                    .into_iter()
                    .map(|swap_action| Action::Swap(swap_action))
                    .collect(),
                referral_id,
                skip_degen_price_sync
            )
            .to_amount(),
        )
    }

    /// Execute set of swap_by_output actions between pools.
    /// If referrer provided, pays referral_fee to it.
    /// If no attached deposit, outgoing tokens used in swaps must be whitelisted.
    #[payable]
    pub fn swap_by_output(&mut self, actions: Vec<SwapByOutputAction>, referral_id: Option<ValidAccountId>, skip_degen_price_sync: Option<bool>) -> U128 {
        U128(
            self.execute_actions(
                actions
                    .into_iter()
                    .map(|swap_by_output_action| Action::SwapByOutput(swap_by_output_action))
                    .collect(),
                referral_id,
                skip_degen_price_sync,
            )
            .to_amount(),
        )
    }

    /// Add liquidity from already deposited amounts to given pool.
    #[payable]
    pub fn add_liquidity(
        &mut self,
        pool_id: u64,
        amounts: Vec<U128>,
        min_amounts: Option<Vec<U128>>,
    ) -> U128 {
        self.assert_contract_running();
        assert!(
            env::attached_deposit() > 0,
            "{}", ERR35_AT_LEAST_ONE_YOCTO
        );
        self.internal_update_unit_share_cumulative_info(pool_id);
        let prev_storage = env::storage_usage();
        let sender_id = env::predecessor_account_id();
        let mut amounts: Vec<u128> = amounts.into_iter().map(|amount| amount.into()).collect();
        let mut pool = self.pools.get(pool_id).expect(ERR85_NO_POOL);
        // feature frozenlist
        self.assert_no_frozen_tokens(pool.tokens());
        // Add amounts given to liquidity first. It will return the balanced amounts.
        let shares = pool.add_liquidity(
            &sender_id,
            &mut amounts,
            false
        );
        if let Some(min_amounts) = min_amounts {
            // Check that all amounts are above request min amounts in case of front running that changes the exchange rate.
            for (amount, min_amount) in amounts.iter().zip(min_amounts.iter()) {
                assert!(amount >= &min_amount.0, "{}", ERR86_MIN_AMOUNT);
            }
        }
        // [AUDITION_AMENDMENT] 2.3.7 Code Optimization (I)
        let mut deposits = self.internal_unwrap_account(&sender_id);
        let tokens = pool.tokens();
        // Subtract updated amounts from deposits. This will fail if there is not enough funds for any of the tokens.
        for i in 0..tokens.len() {
            deposits.withdraw(&tokens[i], amounts[i]);
        }
        self.internal_save_account(&sender_id, deposits);
        self.pools.replace(pool_id, &pool);
        self.internal_check_storage(prev_storage);
        U128(shares)
    }

    /// For stable swap pool, user can add liquidity with token's combination as his will.
    /// But there is a little fee according to the bias of token's combination with the one in the pool.
    /// pool_id: stable pool id. If simple pool is given, panic with unimplement.
    /// amounts: token's combination (in pool tokens sequence) user want to add into the pool, a 0 means absent of that token.
    /// min_shares: Slippage, if shares mint is less than it (cause of fee for too much bias), panic with  ERR68_SLIPPAGE
    #[payable]
    pub fn add_stable_liquidity(
        &mut self,
        pool_id: u64,
        amounts: Vec<U128>,
        min_shares: U128,
    ) -> U128 {
        self.assert_contract_running();
        assert!(
            env::attached_deposit() > 0,
            "{}", ERR35_AT_LEAST_ONE_YOCTO
        );
        self.internal_update_unit_share_cumulative_info(pool_id);
        let prev_storage = env::storage_usage();
        let sender_id = env::predecessor_account_id();
        let amounts: Vec<u128> = amounts.into_iter().map(|amount| amount.into()).collect();
        let mut pool = self.pools.get(pool_id).expect(ERR85_NO_POOL);
        // feature frozenlist
        self.assert_no_frozen_tokens(pool.tokens());
        // Add amounts given to liquidity first. It will return the balanced amounts.
        let mint_shares = pool.add_stable_liquidity(
            &sender_id,
            &amounts,
            min_shares.into(),
            AdminFees::new(self.admin_fee_bps),
            false
        );
        pool.assert_tvl_not_exceed_limit(pool_id);
        // [AUDITION_AMENDMENT] 2.3.7 Code Optimization (I)
        let mut deposits = self.internal_unwrap_account(&sender_id);
        let tokens = pool.tokens();
        // Subtract amounts from deposits. This will fail if there is not enough funds for any of the tokens.
        for i in 0..tokens.len() {
            deposits.withdraw(&tokens[i], amounts[i]);
        }
        self.internal_save_account(&sender_id, deposits);
        self.pools.replace(pool_id, &pool);
        self.internal_check_storage(prev_storage);
        mint_shares.into()
    }

    // #[payable]
    // pub fn add_rated_liquidity(
    //     &mut self,
    //     pool_id: u64,
    //     amounts: Vec<U128>,
    //     min_shares: U128,
    // ) -> U128 {
    //     self.add_stable_liquidity(pool_id, amounts, min_shares)
    // }

    /// Remove liquidity from the pool and add tokens into user internal account.
    #[payable]
    pub fn remove_liquidity(&mut self, pool_id: u64, shares: U128, min_amounts: Vec<U128>) -> Vec<U128> {
        assert_one_yocto();
        self.assert_contract_running();
        self.internal_update_unit_share_cumulative_info(pool_id);
        let sender_id = env::predecessor_account_id();
        let mut pool = self.pools.get(pool_id).expect(ERR85_NO_POOL);
        let mut deposits = self.internal_unwrap_account(&sender_id);
        if let Some(record) = deposits.get_shadow_record(pool_id) {
            assert!(shares.0 <= record.free_shares(pool.share_balances(&sender_id)), "Not enough free shares");
        }
        // feature frozenlist
        self.assert_no_frozen_tokens(pool.tokens());
        let amounts = pool.remove_liquidity(
            &sender_id,
            shares.into(),
            min_amounts
                .into_iter()
                .map(|amount| amount.into())
                .collect(),
            false
        );
        self.pools.replace(pool_id, &pool);
        let tokens = pool.tokens();
        for i in 0..tokens.len() {
            deposits.deposit(&tokens[i], amounts[i]);
        }
        self.internal_save_account(&sender_id, deposits);

        amounts
            .into_iter()
            .map(|amount| amount.into())
            .collect()
    }

    /// For stable swap pool, LP can use it to remove liquidity with given token amount and distribution.
    /// pool_id: the stable swap pool id. If simple pool is given, panic with Unimplement.
    /// amounts: Each tokens (in pool tokens sequence) amounts user want get, a 0 means user don't want to get that token back.
    /// max_burn_shares: This is slippage protection, if user request would burn shares more than it, panic with ERR68_SLIPPAGE
    #[payable]
    pub fn remove_liquidity_by_tokens(
        &mut self, pool_id: u64, 
        amounts: Vec<U128>, 
        max_burn_shares: U128
    ) -> U128 {
        assert_one_yocto();
        self.assert_contract_running();
        self.internal_update_unit_share_cumulative_info(pool_id);
        let sender_id = env::predecessor_account_id();
        let mut pool = self.pools.get(pool_id).expect(ERR85_NO_POOL);
        let mut deposits = self.internal_unwrap_account(&sender_id);
        let free_shares = if let Some(record) = deposits.get_shadow_record(pool_id) {
            record.free_shares(pool.share_balances(&sender_id))
        } else {
            pool.share_balances(&sender_id)
        };
        // feature frozenlist
        self.assert_no_frozen_tokens(pool.tokens());
        let burn_shares = pool.remove_liquidity_by_tokens(
            &sender_id,
            amounts
                .clone()
                .into_iter()
                .map(|amount| amount.into())
                .collect(),
            max_burn_shares.into(),
            AdminFees::new(self.admin_fee_bps),
            false
        );
        assert!(burn_shares <= free_shares, "Not enough free shares");
        self.pools.replace(pool_id, &pool);
        let tokens = pool.tokens();
        for i in 0..tokens.len() {
            deposits.deposit(&tokens[i], amounts[i].into());
        }
        self.internal_save_account(&sender_id, deposits);
        burn_shares.into()
    }

    /// anyone can trigger an update for some rated token
    pub fn update_token_rate(& self, token_id: ValidAccountId) -> PromiseOrValue<bool> {
        let caller = env::predecessor_account_id();
        let token_id: AccountId = token_id.into();
        if let Some(rate) = global_get_rate(&token_id) {
            log!("Caller {} invokes token {} rait async-update.", caller, token_id);
            rate.async_update().then(ext_self::update_token_rate_callback(
                token_id,
                &env::current_account_id(),
                NO_DEPOSIT,
                GAS_FOR_BASIC_OP,
            )).into()
        } else {
            log!("Caller {} invokes token {} rait async-update but it is not a valid token.", caller, token_id);
            PromiseOrValue::Value(true)
        }
    }

    /// the async return of update_token_rate
    #[private]
    pub fn update_token_rate_callback(&mut self, token_id: AccountId) {
        let cross_call_result = if env::promise_results_count() == 1 {
            let cross_call_result = match env::promise_result(0) {
                PromiseResult::Successful(result) => result,
                _ => env::panic(ERR124_CROSS_CALL_FAILED.as_bytes()),
            };
            cross_call_result
        } else {
            // Only SfraxRate + pyth
            assert_eq!(env::promise_results_count(), 2, "{}", ERR123_TWO_PROMISE_RESULT);
            let cross_call_result1 = match env::promise_result(0) {
                PromiseResult::Successful(result) => result,
                _ => env::panic(ERR124_CROSS_CALL_FAILED.as_bytes()),
            };
            let cross_call_result2 = match env::promise_result(1) {
                PromiseResult::Successful(result) => result,
                _ => env::panic(ERR124_CROSS_CALL_FAILED.as_bytes()),
            };
            pair_rated_price_to_vec_u8(cross_call_result1, cross_call_result2)
        };
        if let Some(mut rate) = global_get_rate(&token_id) {
            let new_rate = rate.set(&cross_call_result);
            global_set_rate(&token_id, &rate);
            log!(
                "Token {} got new rate {} from cross-contract call.",
                token_id, new_rate
            );
        }
    }

    /// anyone can trigger a batch update for degen tokens
    ///
    /// # Arguments
    ///
    /// * `token_ids` - List of token IDs.
    pub fn batch_update_degen_token_price(&self, token_ids: Vec<ValidAccountId>) {
        internal_batch_update_degen_token_price(token_ids.into_iter().map(|v| v.into()).collect());
    } 

    /// anyone can trigger an update for some degen token
    pub fn update_degen_token_price(& self, token_id: ValidAccountId) {
        let caller = env::predecessor_account_id();
        let token_id: AccountId = token_id.into();
        let degen = global_get_degen(&token_id);
        log!("Caller {} invokes token {} rait async-update.", caller, token_id);
        degen.sync_token_price(&token_id);
    }

    /// the async return of update_degen_token_price
    #[private]
    pub fn update_degen_token_price_callback(&mut self, token_id: AccountId) {
        if let Some(cross_call_result) = near_sdk::promise_result_as_success() {
            let mut degen = global_get_degen(&token_id);
            let new_degen = degen.set_price(&cross_call_result);
            global_set_degen(&token_id, &degen);
            log!(
                "Token {} got new degen {} from cross-contract call.",
                token_id, new_degen
            );
        }
    }
}

/// Internal methods implementation.
impl Contract {

    fn assert_contract_running(&self) {
        match self.state {
            RunningState::Running => (),
            _ => env::panic(ERR51_CONTRACT_PAUSED.as_bytes()),
        };
    }

    fn assert_no_frozen_tokens(&self, tokens: &[AccountId]) {
        let frozens: Vec<&String> = tokens.iter()
        .filter(
            |token| self.frozen_tokens.contains(*token)
        )
        .collect();
        assert_eq!(frozens.len(), 0, "{}", ERR52_FROZEN_TOKEN);
    }

    fn is_whitelisted_token(&self, token_id: &AccountId) -> bool {
        self.whitelisted_tokens.contains(token_id) || self.auto_whitelisted_postfix.iter().any(|postfix| token_id.ends_with(postfix))
    }

    /// Check how much storage taken costs and refund the left over back.
    /// Return the storage costs due to this call by far.
    fn internal_check_storage(&self, prev_storage: StorageUsage) -> u128 {
        let storage_cost = env::storage_usage()
            .checked_sub(prev_storage)
            .unwrap_or_default() as Balance
            * env::storage_byte_cost();

        let refund = env::attached_deposit()
            .checked_sub(storage_cost)
            .expect(
                format!(
                    "ERR_STORAGE_DEPOSIT need {}, attatched {}", 
                    storage_cost, env::attached_deposit()
                ).as_str()
            );
        if refund > 0 {
            Promise::new(env::predecessor_account_id()).transfer(refund);
        }
        storage_cost
    }

    /// Adds given pool to the list and returns it's id.
    /// If there is not enough attached balance to cover storage, fails.
    /// If too much attached - refunds it back.
    fn internal_add_pool(&mut self, mut pool: Pool) -> u64 {
        let prev_storage = env::storage_usage();
        let id = self.pools.len() as u64;
        // exchange share was registered at creation time
        pool.share_register(&env::current_account_id());
        self.pools.push(&pool);
        internal_set_swap_volume_u256_vec(id, vec![SwapVolumeU256::default(); pool.tokens().len()]);
        self.internal_check_storage(prev_storage);
        id
    }

    fn get_degen_tokens_in_actions(&self, actions: &[Action]) -> HashSet<AccountId> {
        let mut degen_tokens = HashSet::new();
        actions.iter().for_each(|action| {
            if let Pool::DegenSwapPool(p) = self.pools.get(action.get_pool_id()).expect(ERR85_NO_POOL) {
                degen_tokens.extend(p.tokens().iter().cloned());
            }
        });
        degen_tokens
    }

    /// Execute sequence of actions on given account. Modifies passed account.
    /// Returns result of the last action.
    fn internal_execute_actions(
        &mut self,
        account: &mut Account,
        referral_info: &Option<(AccountId, u32)>,
        actions: &[Action],
        prev_result: ActionResult,
        skip_degen_price_sync: bool,
    ) -> ActionResult {
        assert_all_same_action_type(actions);
        // fronzen token feature
        // [AUDITION_AMENDMENT] 2.3.8 Code Optimization (II)
        self.assert_no_frozen_tokens(
            &get_tokens_in_actions(actions)
            .into_iter()
            .map(|token| token)
            .collect::<Vec<AccountId>>()
        );

        let mut result = prev_result;
        match actions[0] {
            Action::Swap(_) => {
                for action in actions {
                    result = self.internal_execute_action(account, referral_info, action, result);
                }
            }
            Action::SwapByOutput(_) => {
                let mut prev_action: Option<&Action> = None;
                for action in actions {
                    if let Some(U128(amount_out)) = action.get_amount_out() {
                        self.finalize_prev_swap_chain(account, prev_action, &result);
                        account.deposit(action.get_token_out(), amount_out);
                    } else {
                        assert!(prev_action.unwrap().get_token_in() == action.get_token_out());
                    }
                    result = self.internal_execute_action(account, referral_info, action, result);
                    prev_action = Some(action);
                }
                self.finalize_prev_swap_chain(account, prev_action, &result);
            }
        }
        if !skip_degen_price_sync {
            let degen_token_ids = self.get_degen_tokens_in_actions(actions).into_iter().collect::<Vec<_>>();
            internal_batch_update_degen_token_price(degen_token_ids);
        }
        result
    }

    fn finalize_prev_swap_chain(&mut self, account: &mut Account, prev_action: Option<&Action>, prev_result: &ActionResult){
        if prev_action.is_some() {
            account.withdraw(prev_action.unwrap().get_token_in(), prev_result.to_amount());
        }
    }

    /// Executes single action on given account. Modifies passed account. Returns a result based on type of action.
    fn internal_execute_action(
        &mut self,
        account: &mut Account,
        referral_info: &Option<(AccountId, u32)>,
        action: &Action,
        prev_result: ActionResult,
    ) -> ActionResult {
        match action {
            Action::Swap(swap_action) => {
                let amount_in = swap_action
                    .amount_in
                    .map(|value| value.0)
                    .unwrap_or_else(|| prev_result.to_amount());
                account.withdraw(&swap_action.token_in, amount_in);
                let amount_out = self.internal_pool_swap(
                    swap_action.pool_id,
                    &swap_action.token_in,
                    amount_in,
                    &swap_action.token_out,
                    swap_action.min_amount_out.0,
                    referral_info,
                );
                account.deposit(&swap_action.token_out, amount_out);
                // [AUDIT_02]
                ActionResult::Amount(U128(amount_out))
            }
            Action::SwapByOutput(swap_by_output_action) => {
                let amount_out = swap_by_output_action
                    .amount_out
                    .map(|value| value.0)
                    .unwrap_or_else(|| prev_result.to_amount());
                let amount_in = self.internal_pool_swap_by_output(
                    swap_by_output_action.pool_id,
                    &swap_by_output_action.token_in,
                    amount_out,
                    &swap_by_output_action.token_out,
                    swap_by_output_action.max_amount_in.map(|v| v.0),
                    referral_info,
                );
                ActionResult::Amount(U128(amount_in))
            }
        }
    }

    /// Swaps given amount_in of token_in into token_out via given pool.
    /// Should be at least min_amount_out or swap will fail (prevents front running and other slippage issues).
    fn internal_pool_swap(
        &mut self,
        pool_id: u64,
        token_in: &AccountId,
        amount_in: u128,
        token_out: &AccountId,
        min_amount_out: u128,
        referral_info: &Option<(AccountId, u32)>,
    ) -> u128 {
        self.internal_update_unit_share_cumulative_info(pool_id);
        let mut pool = self.pools.get(pool_id).expect(ERR85_NO_POOL);
        // Replace pool.volumes for recording.
        let sv_u256s = internal_get_swap_volume_u256_vec_or_default(pool_id, &pool.get_volumes());
        let amount_out = pool.swap(
            token_in,
            amount_in,
            token_out,
            min_amount_out,
            AdminFees {
                admin_fee_bps: self.admin_fee_bps,
                exchange_id: env::current_account_id(),
                referral_info: referral_info.clone(),
            },
            false
        );
        let update_input_idx = match &pool {
            Pool::SimplePool(_) | Pool::StableSwapPool(_) | Pool::RatedSwapPool(_) | Pool::DegenSwapPool(_) => pool.tokens().iter().position(|id| id == token_in).expect(ERR63_MISSING_TOKEN),
        };
        let update_output_idx = match &pool {
            Pool::SimplePool(_) => update_input_idx,
            Pool::StableSwapPool(_) | Pool::RatedSwapPool(_) | Pool::DegenSwapPool(_) => pool.tokens().iter().position(|id| id == token_out).expect(ERR63_MISSING_TOKEN),
        };
        internal_update_swap_volume_u256_vec(
            pool_id,
            update_input_idx,
            update_output_idx,
            amount_in,
            amount_out,
            sv_u256s,
        );
        self.pools.replace(pool_id, &pool);
        amount_out
    }

    /// Swaps token_in into the given amount_out of token_out via a specified pool.
    /// Should be at most max_amount_in or swap will fail (prevents front running and other slippage issues).
    fn internal_pool_swap_by_output(
        &mut self,
        pool_id: u64,
        token_in: &AccountId,
        amount_out: u128,
        token_out: &AccountId,
        max_amount_in: Option<u128>,
        referral_info: &Option<(AccountId, u32)>,
    ) -> u128 {
        self.internal_update_unit_share_cumulative_info(pool_id);
        let mut pool = self.pools.get(pool_id).expect(ERR85_NO_POOL);
        // Replace pool.volumes for recording.
        let sv_u256s = internal_get_swap_volume_u256_vec_or_default(pool_id, &pool.get_volumes());
        let amount_in = pool.swap_by_output(
            token_in,
            amount_out,
            token_out,
            max_amount_in,
            AdminFees {
                admin_fee_bps: self.admin_fee_bps,
                exchange_id: env::current_account_id(),
                referral_info: referral_info.clone(),
            },
            false
        );
        let update_input_idx = match &pool {
            Pool::SimplePool(_) | Pool::StableSwapPool(_) | Pool::RatedSwapPool(_) | Pool::DegenSwapPool(_) => pool.tokens().iter().position(|id| id == token_in).expect(ERR63_MISSING_TOKEN),
        };
        let update_output_idx = match &pool {
            Pool::SimplePool(_) => update_input_idx,
            Pool::StableSwapPool(_) | Pool::RatedSwapPool(_) | Pool::DegenSwapPool(_) => pool.tokens().iter().position(|id| id == token_out).expect(ERR63_MISSING_TOKEN),
        };
        internal_update_swap_volume_u256_vec(
            pool_id,
            update_input_idx,
            update_output_idx,
            amount_in,
            amount_out,
            sv_u256s,
        );
        self.pools.replace(pool_id, &pool);
        amount_in
    }
}


impl Contract {
    fn internal_execute_actions_by_cache(
        &self,
        pool_cache: &mut HashMap<u64, Pool>,
        token_cache: &mut TokenCache,
        referral_info: &Option<(AccountId, u32)>,
        actions: &[Action],
        prev_result: ActionResult,
    ) {
        assert_all_same_action_type(actions);
        self.assert_no_frozen_tokens(
            &get_tokens_in_actions(actions)
            .into_iter()
            .map(|token| token)
            .collect::<Vec<AccountId>>()
        );

        let mut result = prev_result;
        match actions[0] {
            Action::Swap(_) => {
                for action in actions {
                    result = self.internal_execute_action_by_cache(pool_cache, token_cache, referral_info, action, result);
                }
            }
            Action::SwapByOutput(_) => {
                let mut prev_action: Option<&Action> = None;
                for action in actions {
                    if let Some(U128(amount_out)) = action.get_amount_out() {
                        self.finalize_prev_swap_chain_by_cache(token_cache, prev_action, &result);
                        token_cache.add(action.get_token_out(), amount_out);
                    } else {
                        assert!(prev_action.unwrap().get_token_in() == action.get_token_out());
                    }
                    result = self.internal_execute_action_by_cache(pool_cache, token_cache, referral_info, action, result);
                    prev_action = Some(action);
                }
                self.finalize_prev_swap_chain_by_cache(token_cache, prev_action, &result);
            }
        }
    }

    fn finalize_prev_swap_chain_by_cache(&self, token_cache: &mut TokenCache, prev_action: Option<&Action>, prev_result: &ActionResult){
        if prev_action.is_some() {
            token_cache.sub(prev_action.unwrap().get_token_in(), prev_result.to_amount());
        }
    }

    fn internal_execute_action_by_cache(
        &self,
        pool_cache: &mut HashMap<u64, Pool>,
        token_cache: &mut TokenCache,
        referral_info: &Option<(AccountId, u32)>,
        action: &Action,
        prev_result: ActionResult,
    ) -> ActionResult {
        match action {
            Action::Swap(swap_action) => {
                let amount_in = swap_action
                    .amount_in
                    .map(|value| value.0)
                    .unwrap_or_else(|| prev_result.to_amount());
                token_cache.sub(&swap_action.token_in, amount_in);
                let amount_out = self.internal_pool_swap_by_cache(
                    pool_cache,
                    swap_action.pool_id,
                    &swap_action.token_in,
                    amount_in,
                    &swap_action.token_out,
                    swap_action.min_amount_out.0,
                    referral_info,
                );
                token_cache.add(&swap_action.token_out, amount_out);
                ActionResult::Amount(U128(amount_out))
            }
            Action::SwapByOutput(swap_by_output_action) => {
                let amount_out = swap_by_output_action
                    .amount_out
                    .map(|value| value.0)
                    .unwrap_or_else(|| prev_result.to_amount());
                let amount_in = self.internal_pool_swap_by_output_by_cache(
                    pool_cache,
                    swap_by_output_action.pool_id,
                    &swap_by_output_action.token_in,
                    amount_out,
                    &swap_by_output_action.token_out,
                    swap_by_output_action.max_amount_in.map(|v| v.0),
                    referral_info,
                );
                ActionResult::Amount(U128(amount_in))
            }
        }
    }

    fn internal_pool_swap_by_cache(
        &self,
        pool_cache: &mut HashMap<u64, Pool>,
        pool_id: u64,
        token_in: &AccountId,
        amount_in: u128,
        token_out: &AccountId,
        min_amount_out: u128,
        referral_info: &Option<(AccountId, u32)>,
    ) -> u128 {
        let mut pool = pool_cache.remove(&pool_id).unwrap_or(self.pools.get(pool_id).expect(ERR85_NO_POOL));
        let amount_out = pool.swap(
            token_in,
            amount_in,
            token_out,
            min_amount_out,
            AdminFees {
                admin_fee_bps: self.admin_fee_bps,
                exchange_id: env::current_account_id(),
                referral_info: referral_info.clone(),
            },
            true
        );
        pool_cache.insert(pool_id, pool);
        amount_out
    }

    fn internal_pool_swap_by_output_by_cache(
        &self,
        pool_cache: &mut HashMap<u64, Pool>,
        pool_id: u64,
        token_in: &AccountId,
        amount_out: u128,
        token_out: &AccountId,
        max_amount_in: Option<u128>,
        referral_info: &Option<(AccountId, u32)>,
    ) -> u128 {
        let mut pool = pool_cache.remove(&pool_id).unwrap_or(self.pools.get(pool_id).expect(ERR85_NO_POOL));
        let amount_in = pool.swap_by_output(
            token_in,
            amount_out,
            token_out,
            max_amount_in,
            AdminFees {
                admin_fee_bps: self.admin_fee_bps,
                exchange_id: env::current_account_id(),
                referral_info: referral_info.clone(),
            },
            true
        );
        pool_cache.insert(pool_id, pool);
        amount_in
    }
}

#[cfg(test)]
mod tests {
    use std::convert::TryFrom;

    use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::{testing_env, Balance, MockedBlockchain};
    use near_sdk_sim::to_yocto;

    use super::*;

    /// Creates contract and a pool with tokens with 0.3% of total fee.
    fn setup_contract() -> (VMContextBuilder, Contract) {
        let mut context = VMContextBuilder::new();
        testing_env!(context.predecessor_account_id(accounts(0)).build());
        let contract = Contract::new(accounts(0), "boost_farm".to_string().try_into().unwrap(), "burrowland".to_string().try_into().unwrap(), 2000, 0);
        (context, contract)
    }

    fn deposit_tokens(
        context: &mut VMContextBuilder,
        contract: &mut Contract,
        account_id: ValidAccountId,
        token_amounts: Vec<(ValidAccountId, Balance)>,
    ) {
        if contract.storage_balance_of(account_id.clone()).is_none() {
            testing_env!(context
                .predecessor_account_id(account_id.clone())
                .attached_deposit(to_yocto("1"))
                .build());
            contract.storage_deposit(None, None);
        }
        testing_env!(context
            .predecessor_account_id(account_id.clone())
            .attached_deposit(to_yocto("1"))
            .build());
        let tokens = token_amounts
            .iter()
            .map(|(token_id, _)| token_id.clone().into())
            .collect();
        testing_env!(context.attached_deposit(1).build());
        contract.register_tokens(tokens);
        for (token_id, amount) in token_amounts {
            testing_env!(context
                .predecessor_account_id(token_id)
                .attached_deposit(1)
                .build());
            contract.ft_on_transfer(account_id.clone(), U128(amount), "".to_string());
        }
    }

    fn create_pool_with_liquidity(
        context: &mut VMContextBuilder,
        contract: &mut Contract,
        account_id: ValidAccountId,
        token_amounts: Vec<(ValidAccountId, Balance)>,
    ) -> u64 {
        let tokens = token_amounts
            .iter()
            .map(|(x, _)| x.clone())
            .collect::<Vec<_>>();
        testing_env!(context.predecessor_account_id(accounts(0)).attached_deposit(1).build());
        contract.extend_whitelisted_tokens(tokens.clone());
        testing_env!(context
            .predecessor_account_id(account_id.clone())
            .attached_deposit(env::storage_byte_cost() * 466)
            .build());
        let pool_id = contract.add_simple_pool(tokens, 25);
        testing_env!(context
            .predecessor_account_id(account_id.clone())
            .attached_deposit(to_yocto("0.03"))
            .build());
        contract.storage_deposit(None, None);
        deposit_tokens(context, contract, accounts(3), token_amounts.clone());
        testing_env!(context
            .predecessor_account_id(account_id.clone())
            .attached_deposit(to_yocto("0.0007"))
            .build());
        contract.add_liquidity(
            pool_id,
            token_amounts.into_iter().map(|(_, x)| U128(x)).collect(),
            None,
        );
        pool_id
    }

    fn swap(
        contract: &mut Contract,
        pool_id: u64,
        token_in: ValidAccountId,
        amount_in: Balance,
        token_out: ValidAccountId,
    ) -> Balance {
        contract
            .swap(
                vec![SwapAction {
                    pool_id,
                    token_in: token_in.into(),
                    amount_in: Some(U128(amount_in)),
                    token_out: token_out.into(),
                    min_amount_out: U128(1),
                }],
                None,
                None,
            )
            .0
    }

    #[test]
    fn test_basics() {
        let one_near = 10u128.pow(24);
        let (mut context, mut contract) = setup_contract();
        // add liquidity of (1,2) tokens
        create_pool_with_liquidity(
            &mut context,
            &mut contract,
            accounts(3),
            vec![(accounts(1), to_yocto("5")), (accounts(2), to_yocto("10"))],
        );
        deposit_tokens(
            &mut context,
            &mut contract,
            accounts(3),
            vec![
                (accounts(1), to_yocto("100")),
                (accounts(2), to_yocto("100")),
            ],
        );
        deposit_tokens(&mut context, &mut contract, accounts(1), vec![]);

        assert_eq!(
            contract.get_deposit(accounts(3), accounts(1)),
            to_yocto("100").into()
        );
        assert_eq!(
            contract.get_deposit(accounts(3), accounts(2)),
            to_yocto("100").into()
        );
        assert_eq!(
            contract.get_pool_total_shares(0).0,
            crate::utils::INIT_SHARES_SUPPLY
        );

        // Get price from pool :0 1 -> 2 tokens.
        let expected_out = contract.get_return(0, accounts(1), one_near.into(), accounts(2));
        assert_eq!(expected_out.0, 1663192997082117548978741);

        testing_env!(context
            .predecessor_account_id(accounts(3))
            .attached_deposit(1)
            .build());
        let amount_out = swap(&mut contract, 0, accounts(1), one_near, accounts(2));
        assert_eq!(amount_out, expected_out.0);
        assert_eq!(
            contract.get_deposit(accounts(3), accounts(1)).0,
            99 * one_near
        );
        // transfer some of token_id 2 from acc 3 to acc 1.
        testing_env!(context.predecessor_account_id(accounts(3)).build());
        contract.mft_transfer(accounts(2).to_string(), accounts(1), U128(one_near), None);
        assert_eq!(
            contract.get_deposit(accounts(3), accounts(2)).0,
            99 * one_near + amount_out
        );
        assert_eq!(contract.get_deposit(accounts(1), accounts(2)).0, one_near);

        testing_env!(context
            .predecessor_account_id(accounts(3))
            .attached_deposit(to_yocto("0.0067"))
            .build());
        contract.mft_register(":0".to_string(), accounts(1));
        testing_env!(context
            .predecessor_account_id(accounts(3))
            .attached_deposit(1)
            .build());
        // transfer 1m shares in pool 0 to acc 1.
        contract.mft_transfer(":0".to_string(), accounts(1), U128(1_000_000), None);

        testing_env!(context.predecessor_account_id(accounts(3)).build());
        contract.remove_liquidity(
            0,
            contract.get_pool_shares(0, accounts(3)),
            vec![1.into(), 2.into()],
        );
        // Exchange fees left in the pool as liquidity + 1m from transfer.
        // 33336806279123620258 v1.6.2
        // 41671007848904525323 refactor fee to support referral map
        // 41679692022771629522 fix simple pool swap admin fee algorithm
        assert_eq!(
            contract.get_pool_total_shares(0).0,
            41679692022771629522 + 1_000_000
        );

        contract.withdraw(
            accounts(1),
            contract.get_deposit(accounts(3), accounts(1)),
            None,
            None,
        );
        assert_eq!(contract.get_deposit(accounts(3), accounts(1)).0, 0);
    }

    /// Test liquidity management.
    #[test]
    fn test_liquidity() {
        let (mut context, mut contract) = setup_contract();
        deposit_tokens(
            &mut context,
            &mut contract,
            accounts(3),
            vec![
                (accounts(1), to_yocto("100")),
                (accounts(2), to_yocto("100")),
            ],
        );
        testing_env!(context
            .predecessor_account_id(accounts(3))
            .attached_deposit(to_yocto("1"))
            .build());
        let id = contract.add_simple_pool(vec![accounts(1), accounts(2)], 25);
        testing_env!(context.attached_deposit(to_yocto("0.0007")).build());
        contract.add_liquidity(id, vec![U128(to_yocto("50")), U128(to_yocto("10"))], None);
        contract.add_liquidity(id, vec![U128(to_yocto("50")), U128(to_yocto("50"))], None);
        testing_env!(context.attached_deposit(1).build());
        contract.remove_liquidity(id, U128(to_yocto("1")), vec![U128(1), U128(1)]);

        // Check that amounts add up to deposits.
        let amounts = contract.get_pool(id).amounts;
        let deposit1 = contract.get_deposit(accounts(3), accounts(1)).0;
        let deposit2 = contract.get_deposit(accounts(3), accounts(2)).0;
        assert_eq!(amounts[0].0 + deposit1, to_yocto("100"));
        assert_eq!(amounts[1].0 + deposit2, to_yocto("100"));
    }

    #[test]
    fn test_add_liquidity_rounding() {
        let (mut context, mut contract) = setup_contract();
        deposit_tokens(
            &mut context,
            &mut contract,
            accounts(3),
            vec![
                (accounts(1), to_yocto("100")),
                (accounts(2), to_yocto("100")),
            ],
        );
        testing_env!(context
            .predecessor_account_id(accounts(3))
            .attached_deposit(to_yocto("1"))
            .build());
        let id = contract.add_simple_pool(vec![accounts(1), accounts(2)], 25);
        testing_env!(context.attached_deposit(to_yocto("0.0007")).build());
        contract.add_liquidity(id, vec![U128(1000), U128(1000)], None);
        assert_eq!(1000000000000000000000000u128, contract.get_pool(id).shares_total_supply.0);

        deposit_tokens(
            &mut context,
            &mut contract,
            accounts(4),
            vec![
                (accounts(1), to_yocto("100")),
                (accounts(2), to_yocto("100")),
            ],
        );
        testing_env!(context
            .predecessor_account_id(accounts(4))
            .attached_deposit(to_yocto("1"))
            .build());
        contract.add_liquidity(id, vec![U128(2), U128(2)], None);
        assert_eq!(1000000000000000000000u128, contract.get_pool_shares(id, accounts(4)).0);

        let pool_info = contract.get_pool(id);
        assert_eq!(1001000000000000000000000u128, pool_info.shares_total_supply.0);
        assert_eq!(vec![U128(1002), U128(1002)], pool_info.amounts);

        testing_env!(context
            .attached_deposit(1)
            .build());
        let remvoe_tokens = contract.remove_liquidity(id, U128(1000000000000000000000u128 - 1), vec![U128(0), U128(0)]);
        assert_eq!(vec![U128(1), U128(1)], remvoe_tokens);

        let pool_info = contract.get_pool(id);
        assert_eq!(1000000000000000000000001u128, pool_info.shares_total_supply.0);
        assert_eq!(vec![U128(1001), U128(1001)], pool_info.amounts);

        testing_env!(context
            .attached_deposit(to_yocto("1"))
            .build());

        contract.add_liquidity(id, vec![U128(50), U128(50)], None);
        assert_eq!(48951048951048951048952u128, contract.get_pool_shares(id, accounts(4)).0);

        let pool_info = contract.get_pool(id);
        assert_eq!(vec![U128(1050), U128(1050)], pool_info.amounts);

        testing_env!(context
            .attached_deposit(1)
            .build());
        let remvoe_tokens = contract.remove_liquidity(id, U128(48951048951048951048952u128 - 1), vec![U128(0), U128(0)]);
        assert_eq!(vec![U128(48), U128(48)], remvoe_tokens);

        let pool_info = contract.get_pool(id);
        assert_eq!(1000000000000000000000001u128, pool_info.shares_total_supply.0);
        assert_eq!(vec![U128(1002), U128(1002)], pool_info.amounts);
    }
    
    #[test]
    #[should_panic(expected = "E31: adding zero amount")]
    fn test_init_zero_liquidity() {
        let (mut context, mut contract) = setup_contract();
        deposit_tokens(
            &mut context,
            &mut contract,
            accounts(3),
            vec![
                (accounts(1), to_yocto("1000000")),
                (accounts(2), to_yocto("1000000")),
            ],
        );
        testing_env!(context
            .predecessor_account_id(accounts(3))
            .attached_deposit(to_yocto("1"))
            .build());
        let id0 = contract.add_simple_pool(vec![accounts(1), accounts(2)], 1);
        testing_env!(context.attached_deposit(to_yocto("0.0007")).build());
        contract.add_liquidity(id0, vec![U128(0), U128(0)], None);
    }

    #[test]
    #[should_panic(expected = "E36: shares_total_supply overflow")]
    fn test_shares_total_supply_overflow() {
        let (mut context, mut contract) = setup_contract();
        deposit_tokens(
            &mut context,
            &mut contract,
            accounts(3),
            vec![
                (accounts(1), to_yocto("1000000")),
                (accounts(2), to_yocto("1000000")),
            ],
        );
        testing_env!(context
            .predecessor_account_id(accounts(3))
            .attached_deposit(to_yocto("1"))
            .build());
        let id0 = contract.add_simple_pool(vec![accounts(1), accounts(2)], 1);
        testing_env!(context.attached_deposit(to_yocto("0.0007")).build());
        contract.add_liquidity(id0, vec![U128(4801823983302), U128(14399)], None);
        testing_env!(context.attached_deposit(to_yocto("0.0007")).build());
        contract.add_liquidity(id0, vec![U128(340282366920167 * 4801823983302), U128(340282366920167 * 14399)], None);
        contract.swap(
            vec![SwapAction {
                pool_id: 0,
                token_in: accounts(1).into(),
                amount_in: Some(U128(12446461932933863316530306u128)),
                token_out: accounts(2).into(),
                min_amount_out: U128(0),
            }],
            None,
            None,
        );
    }

    /// Should deny creating a pool with duplicate tokens.
    #[test]
    #[should_panic(expected = "E92: token duplicated")]
    fn test_deny_duplicate_tokens_pool() {
        let (mut context, mut contract) = setup_contract();
        create_pool_with_liquidity(
            &mut context,
            &mut contract,
            accounts(3),
            vec![(accounts(1), to_yocto("5")), (accounts(1), to_yocto("10"))],
        );
    }

    /// Deny pool with a single token
    #[test]
    #[should_panic(expected = "E89: wrong token count")]
    fn test_deny_single_token_pool() {
        let (mut context, mut contract) = setup_contract();
        create_pool_with_liquidity(
            &mut context,
            &mut contract,
            accounts(3),
            vec![(accounts(1), to_yocto("5"))],
        );
    }

    /// Deny pool with a single token
    #[test]
    #[should_panic(expected = "E89: wrong token count")]
    fn test_deny_too_many_tokens_pool() {
        let (mut context, mut contract) = setup_contract();
        create_pool_with_liquidity(
            &mut context,
            &mut contract,
            accounts(3),
            vec![
                (accounts(1), to_yocto("5")),
                (accounts(2), to_yocto("10")),
                (accounts(3), to_yocto("10")),
            ],
        );
    }

    #[test]
    #[should_panic(expected = "E12: token not whitelisted")]
    fn test_deny_send_malicious_token() {
        let (mut context, mut contract) = setup_contract();
        let acc = ValidAccountId::try_from("test_user").unwrap();
        testing_env!(context
            .predecessor_account_id(acc.clone())
            .attached_deposit(to_yocto("1"))
            .build());
        contract.storage_deposit(Some(acc.clone()), None);
        testing_env!(context
            .predecessor_account_id(ValidAccountId::try_from("malicious").unwrap())
            .build());
        contract.ft_on_transfer(acc, U128(1_000), "".to_string());
    }

    #[test]
    fn test_auto_whitelisted_postfix() {
        let (mut context, mut contract) = setup_contract();
        let acc = ValidAccountId::try_from("test_user").unwrap();
        let token1 = ValidAccountId::try_from("test.abc.near").unwrap();
        let token2 = ValidAccountId::try_from("test.def.near").unwrap();
        testing_env!(context
            .predecessor_account_id(acc.clone())
            .attached_deposit(to_yocto("1"))
            .build());
        contract.storage_deposit(None, None);
        testing_env!(context.predecessor_account_id(accounts(0)).attached_deposit(1).build());
        contract.extend_auto_whitelisted_postfix(vec!["abc.near".to_string(), "def.near".to_string()]);
        testing_env!(context
            .predecessor_account_id(token1.clone())
            .attached_deposit(1)
            .build());
        contract.ft_on_transfer(acc.clone(), U128(1_000_000), "".to_string());

        testing_env!(context
            .predecessor_account_id(token2.clone())
            .attached_deposit(1)
            .build());
        contract.ft_on_transfer(acc.clone(), U128(1_000_000), "".to_string());

        testing_env!(context
            .predecessor_account_id(acc.clone())
            .attached_deposit(env::storage_byte_cost() * 500)
            .build());
        let pool_id = contract.add_simple_pool(vec![token1.clone(), token2.clone()], 25);
        testing_env!(context
            .predecessor_account_id(acc.clone())
            .attached_deposit(to_yocto("0.1"))
            .build());
        contract.add_liquidity(
            pool_id,
            vec![U128(10000), U128(10000)],
            None,
        );
        
        let actions: Vec<Action> = vec![Action::Swap(SwapAction{
            pool_id,
            token_in: token1.to_string(),
            amount_in: Some(U128(10)),
            token_out: token2.to_string(),
            min_amount_out: U128(0),
        })];
        
        testing_env!(context
            .predecessor_account_id(acc.clone())
            .build());
        contract.execute_actions(actions, None, None);
    }

    #[test]
    fn test_send_user_specific_token() {
        let (mut context, mut contract) = setup_contract();
        let acc = ValidAccountId::try_from("test_user").unwrap();
        let custom_token = ValidAccountId::try_from("custom").unwrap();
        testing_env!(context
            .predecessor_account_id(acc.clone())
            .attached_deposit(to_yocto("1"))
            .build());
        contract.storage_deposit(None, None);
        testing_env!(context.attached_deposit(1).build());
        contract.register_tokens(vec![custom_token.clone()]);
        testing_env!(context.predecessor_account_id(custom_token.clone()).build());
        contract.ft_on_transfer(acc.clone(), U128(1_000), "".to_string());
        let prev = contract.storage_balance_of(acc.clone()).unwrap();
        testing_env!(context
            .predecessor_account_id(acc.clone())
            .attached_deposit(1)
            .build());
        contract.withdraw(custom_token, U128(1_000), Some(true), None);
        let new = contract.storage_balance_of(acc.clone()).unwrap();
        // More available storage after withdrawing & unregistering the token.
        assert!(new.available.0 > prev.available.0);
    }

    #[test]
    #[should_panic(expected = "E68: slippage error")]
    fn test_deny_min_amount() {
        let (mut context, mut contract) = setup_contract();
        create_pool_with_liquidity(
            &mut context,
            &mut contract,
            accounts(3),
            vec![(accounts(1), to_yocto("1")), (accounts(2), to_yocto("1"))],
        );
        let acc = ValidAccountId::try_from("test_user").unwrap();

        deposit_tokens(
            &mut context,
            &mut contract,
            acc.clone(),
            vec![(accounts(1), 1_000_000)],
        );

        testing_env!(context
            .predecessor_account_id(acc.clone())
            .attached_deposit(1)
            .build());
        contract.swap(
            vec![SwapAction {
                pool_id: 0,
                token_in: accounts(1).into(),
                amount_in: Some(U128(1_000_000)),
                token_out: accounts(2).into(),
                min_amount_out: U128(1_000_000),
            }],
            None,
            None,
        );
    }

    #[test]
    fn test_second_storage_deposit_works() {
        let (mut context, mut contract) = setup_contract();
        testing_env!(context.attached_deposit(to_yocto("1")).build());
        contract.storage_deposit(None, None);
        testing_env!(context.attached_deposit(to_yocto("0.001")).build());
        contract.storage_deposit(None, None);
    }

    #[test]
    #[should_panic(expected = "E72: at least one swap")]
    fn test_fail_swap_no_actions() {
        let (mut context, mut contract) = setup_contract();
        testing_env!(context.attached_deposit(to_yocto("1")).build());
        contract.storage_deposit(None, None);
        testing_env!(context.attached_deposit(1).build());
        contract.swap(vec![], None, None);
    }

    /// Check that can not swap non whitelisted tokens when attaching 0 deposit (access key).
    #[test]
    #[should_panic(expected = "E27: attach 1yN to swap tokens not in whitelist")]
    fn test_fail_swap_not_whitelisted() {
        let (mut context, mut contract) = setup_contract();
        deposit_tokens(
            &mut context,
            &mut contract,
            accounts(0),
            vec![(accounts(1), 2_000_000), (accounts(2), 1_000_000)],
        );
        create_pool_with_liquidity(
            &mut context,
            &mut contract,
            accounts(0),
            vec![(accounts(1), 1_000_000), (accounts(2), 1_000_000)],
        );
        testing_env!(context.attached_deposit(1).build());
        contract.remove_whitelisted_tokens(vec![accounts(2)]);
        testing_env!(context.attached_deposit(1).build());
        contract.unregister_tokens(vec![accounts(2)]);
        testing_env!(context.attached_deposit(0).build());
        swap(&mut contract, 0, accounts(1), 10, accounts(2));
    }

    #[test]
    fn test_roundtrip_swap() {
        let (mut context, mut contract) = setup_contract();
        create_pool_with_liquidity(
            &mut context,
            &mut contract,
            accounts(3),
            vec![(accounts(1), to_yocto("5")), (accounts(2), to_yocto("10"))],
        );
        let acc = ValidAccountId::try_from("test_user").unwrap();
        deposit_tokens(
            &mut context,
            &mut contract,
            acc.clone(),
            vec![(accounts(1), 1_000_000)],
        );
        testing_env!(context
            .predecessor_account_id(acc.clone())
            .attached_deposit(1)
            .build());
        contract.swap(
            vec![
                SwapAction {
                    pool_id: 0,
                    token_in: accounts(1).into(),
                    amount_in: Some(U128(1_000)),
                    token_out: accounts(2).into(),
                    min_amount_out: U128(1),
                },
                SwapAction {
                    pool_id: 0,
                    token_in: accounts(2).into(),
                    amount_in: None,
                    token_out: accounts(1).into(),
                    min_amount_out: U128(1),
                },
            ],
            None,
            None,
        );
        // Roundtrip returns almost everything except 0.25% fee.
        assert_eq!(contract.get_deposit(acc, accounts(1)).0, 1_000_000 - 6);
    }

    #[test]
    #[should_panic(expected = "E14: LP already registered")]
    fn test_lpt_transfer() {
        // account(0) -- swap contract
        // account(1) -- token0 contract
        // account(2) -- token1 contract
        // account(3) -- user account
        // account(4) -- another user account
        let (mut context, mut contract) = setup_contract();
        deposit_tokens(
            &mut context,
            &mut contract,
            accounts(3),
            vec![
                (accounts(1), to_yocto("100")),
                (accounts(2), to_yocto("100")),
            ],
        );
        testing_env!(context
            .predecessor_account_id(accounts(3))
            .attached_deposit(to_yocto("1"))
            .build());
        let id = contract.add_simple_pool(vec![accounts(1), accounts(2)], 25);
        testing_env!(context.attached_deposit(to_yocto("0.0007")).build());
        contract.add_liquidity(id, vec![U128(to_yocto("50")), U128(to_yocto("10"))], None);
        assert_eq!(
            contract.mft_balance_of(":0".to_string(), accounts(3)).0,
            to_yocto("1")
        );
        assert_eq!(contract.mft_total_supply(":0".to_string()).0, to_yocto("1"));
        testing_env!(context.attached_deposit(1).build());
        contract.add_liquidity(id, vec![U128(to_yocto("50")), U128(to_yocto("50"))], None);
        assert_eq!(
            contract.mft_balance_of(":0".to_string(), accounts(3)).0,
            to_yocto("2") - 1
        );
        assert_eq!(contract.mft_total_supply(":0".to_string()).0, to_yocto("2") - 1);

        // register another user
        testing_env!(context
            .predecessor_account_id(accounts(4))
            .attached_deposit(to_yocto("0.00071"))
            .build());
        contract.mft_register(":0".to_string(), accounts(4));
        // make transfer to him
        testing_env!(context
            .predecessor_account_id(accounts(3))
            .attached_deposit(1)
            .build());
        contract.mft_transfer(":0".to_string(), accounts(4), U128(to_yocto("1")), None);
        assert_eq!(
            contract.mft_balance_of(":0".to_string(), accounts(3)).0,
            to_yocto("1") - 1
        );
        assert_eq!(
            contract.mft_balance_of(":0".to_string(), accounts(4)).0,
            to_yocto("1")
        );
        assert_eq!(contract.mft_total_supply(":0".to_string()).0, to_yocto("2") - 1);
        // remove lpt for account 3
        testing_env!(context
            .predecessor_account_id(accounts(3))
            .attached_deposit(1)
            .build());
        contract.remove_liquidity(id, U128(to_yocto("0.6")), vec![U128(1), U128(1)]);
        assert_eq!(
            contract.mft_balance_of(":0".to_string(), accounts(3)).0,
            to_yocto("0.4") - 1
        );
        assert_eq!(
            contract.mft_total_supply(":0".to_string()).0,
            to_yocto("1.4") - 1
        );
        // remove lpt for account 4 who got lpt from others
        if contract.storage_balance_of(accounts(4)).is_none() {
            testing_env!(context
                .predecessor_account_id(accounts(4))
                .attached_deposit(to_yocto("1"))
                .build());
            contract.storage_deposit(None, None);
        }
        testing_env!(context
            .predecessor_account_id(accounts(4))
            .attached_deposit(1)
            .build());
        contract.remove_liquidity(id, U128(to_yocto("1")), vec![U128(1), U128(1)]);
        assert_eq!(
            contract.mft_balance_of(":0".to_string(), accounts(4)).0,
            to_yocto("0")
        );
        assert_eq!(
            contract.mft_total_supply(":0".to_string()).0,
            to_yocto("0.4") - 1
        );

        // [AUDIT_13]
        // should panic cause accounts(4) not removed by a full remove liquidity
        testing_env!(context
            .predecessor_account_id(accounts(4))
            .attached_deposit(to_yocto("0.00071"))
            .build());
        contract.mft_register(":0".to_string(), accounts(4));
    }

    #[test]
    #[should_panic(expected = "E33: transfer to self")]
    fn test_lpt_transfer_self() {
        // [AUDIT_07]
        // account(0) -- swap contract
        // account(1) -- token0 contract
        // account(2) -- token1 contract
        // account(3) -- user account
        let (mut context, mut contract) = setup_contract();
        deposit_tokens(
            &mut context,
            &mut contract,
            accounts(3),
            vec![
                (accounts(1), to_yocto("100")),
                (accounts(2), to_yocto("100")),
            ],
        );
        testing_env!(context
            .predecessor_account_id(accounts(3))
            .attached_deposit(to_yocto("1"))
            .build());
        let id = contract.add_simple_pool(vec![accounts(1), accounts(2)], 25);
        testing_env!(context.attached_deposit(to_yocto("0.0007")).build());
        contract.add_liquidity(id, vec![U128(to_yocto("50")), U128(to_yocto("10"))], None);
        assert_eq!(
            contract.mft_balance_of(":0".to_string(), accounts(3)).0,
            to_yocto("1")
        );
        testing_env!(context.attached_deposit(1).build());
        contract.add_liquidity(id, vec![U128(to_yocto("50")), U128(to_yocto("50"))], None);
        assert_eq!(
            contract.mft_balance_of(":0".to_string(), accounts(3)).0,
            to_yocto("2") - 1
        );

        // make transfer to self
        testing_env!(context
            .predecessor_account_id(accounts(3))
            .attached_deposit(1)
            .build());
        contract.mft_transfer(":0".to_string(), accounts(3), U128(to_yocto("1")), None);
    }

    #[test]
    fn test_storage() {
        let (mut context, mut contract) = setup_contract();
        testing_env!(context
            .predecessor_account_id(accounts(1))
            .attached_deposit(to_yocto("1"))
            .build());
        contract.storage_deposit(Some(accounts(1)), None);
        testing_env!(context
            .predecessor_account_id(accounts(1))
            .attached_deposit(1)
            .build());
        assert_eq!(contract.storage_withdraw(None).available.0, 0);
        testing_env!(context
            .predecessor_account_id(accounts(1))
            .attached_deposit(1)
            .build());
        assert!(contract.storage_unregister(None));
    }

    #[test]
    fn test_storage_registration_only() {
        let (mut context, mut contract) = setup_contract();
        testing_env!(context
            .predecessor_account_id(accounts(1))
            .attached_deposit(to_yocto("1"))
            .build());
        let deposit1 = contract.storage_deposit(Some(accounts(1)), Some(true));
        testing_env!(context
            .predecessor_account_id(accounts(1))
            .attached_deposit(to_yocto("1"))
            .build());
        let deposit2 = contract.storage_deposit(Some(accounts(1)), Some(true));
        assert_eq!(deposit1.total, deposit2.total);
    }

    #[test]
    #[should_panic(expected = "E17: deposit less than min storage")]
    fn test_storage_deposit_less_then_min_storage() {
        let (mut context, mut contract) = setup_contract();
        testing_env!(context
            .predecessor_account_id(accounts(1))
            .attached_deposit(1)
            .build());
        contract.storage_deposit(Some(accounts(1)), Some(true));
    }

    #[test]
    fn test_instant_swap() {
        let (mut context, mut contract) = setup_contract();
        // add liquidity of (1,2) tokens
        create_pool_with_liquidity(
            &mut context,
            &mut contract,
            accounts(3),
            vec![(accounts(1), to_yocto("5")), (accounts(2), to_yocto("10"))],
        );
        deposit_tokens(
            &mut context,
            &mut contract,
            accounts(3),
            vec![
                (accounts(1), to_yocto("100")),
                (accounts(2), to_yocto("100")),
            ],
        );

        assert_eq!(
            contract.get_deposit(accounts(3), accounts(1)),
            to_yocto("100").into()
        );
        assert_eq!(
            contract.get_deposit(accounts(3), accounts(2)),
            to_yocto("100").into()
        );
        assert_eq!(
            contract.get_pool_total_shares(0).0,
            crate::utils::INIT_SHARES_SUPPLY
        );

        // Get price from pool :0 1 -> 2 tokens.
        let expected_out = contract.get_return(0, accounts(1), to_yocto("1").into(), accounts(2));
        assert_eq!(expected_out.0, 1663192997082117548978741);

        let actions_str = format!(
            "{{\"pool_id\": {}, \"token_in\": \"{}\", \"token_out\": \"{}\", \"min_amount_out\": \"{}\"}}",
            0, accounts(1), accounts(2), 1
        );

        let msg_str = format!("{{\"actions\": [{}]}}", actions_str);
        testing_env!(context
            .predecessor_account_id(accounts(1))
            .attached_deposit(1)
            .build());
        contract.ft_on_transfer(accounts(3), to_yocto("1").into(), msg_str);
    }

    #[test]
    fn test_mft_transfer_call() {
        let one_near = 10u128.pow(24);
        let (mut context, mut contract) = setup_contract();
        // add liquidity of (1,2) tokens
        create_pool_with_liquidity(
            &mut context,
            &mut contract,
            accounts(3),
            vec![(accounts(1), to_yocto("5")), (accounts(2), to_yocto("10"))],
        );
        println!("{:?}", contract.get_pools(0, 100));
        println!("{:?}", contract.get_pool(0));
        deposit_tokens(
            &mut context,
            &mut contract,
            accounts(3),
            vec![
                (accounts(1), to_yocto("100")),
                (accounts(2), to_yocto("100")),
            ],
        );
        deposit_tokens(&mut context, &mut contract, accounts(1), vec![]);

        assert_eq!(
            contract.get_deposit(accounts(3), accounts(1)),
            to_yocto("100").into()
        );
        assert_eq!(
            contract.get_deposit(accounts(3), accounts(2)),
            to_yocto("100").into()
        );
        assert_eq!(
            contract.get_pool_total_shares(0).0,
            crate::utils::INIT_SHARES_SUPPLY
        );

        // Get price from pool :0 1 -> 2 tokens.
        let expected_out = contract.get_return(0, accounts(1), one_near.into(), accounts(2));
        assert_eq!(expected_out.0, 1663192997082117548978741);

        testing_env!(context
            .predecessor_account_id(accounts(3))
            .attached_deposit(1)
            .build());
        let amount_out = swap(&mut contract, 0, accounts(1), one_near, accounts(2));
        assert_eq!(amount_out, expected_out.0);
        assert_eq!(
            contract.get_deposit(accounts(3), accounts(1)).0,
            99 * one_near
        );
        assert_eq!("ref-pool-0".to_string(), contract.mft_metadata(":0".to_string()).name);
        // transfer some of token_id 2 from acc 3 to acc 1.
        testing_env!(context.predecessor_account_id(accounts(3)).build());
        contract.mft_transfer_call(accounts(2).to_string(), accounts(1), U128(one_near), Some("mft".to_string()), "".to_string());
        assert_eq!(
            contract.get_deposit(accounts(3), accounts(2)).0,
            99 * one_near + amount_out
        );
    }

    #[test]
    fn test_stable() {
        let (mut context, mut contract) = setup_contract();
        let token_amounts = vec![(accounts(1), to_yocto("5")), (accounts(2), to_yocto("5"))];
        let tokens = token_amounts
            .iter()
            .map(|(x, _)| x.clone())
            .collect::<Vec<_>>();
        testing_env!(context.predecessor_account_id(accounts(0)).attached_deposit(1).build());
        contract.extend_whitelisted_tokens(tokens.clone());
        assert_eq!(contract.get_whitelisted_tokens(), vec![accounts(1).to_string(), accounts(2).to_string()]);
        assert_eq!(0, contract.get_user_whitelisted_tokens(accounts(3)).len());
        testing_env!(context
            .predecessor_account_id(accounts(0))
            .attached_deposit(env::storage_byte_cost() * 512)
            .build());
        let pool_id = contract.add_stable_swap_pool(tokens, vec![18, 18], 25, 240);
        println!("{:?}", contract.version());
        println!("{:?}", contract.get_stable_pool(pool_id));
        println!("{:?}", contract.get_pools(0, 100));
        println!("{:?}", contract.get_pool(0));
        assert_eq!(1, contract.get_number_of_pools());
        assert_eq!(25, contract.get_pool_fee(pool_id));
        testing_env!(context
            .predecessor_account_id(accounts(3))
            .attached_deposit(to_yocto("0.03"))
            .build());
        contract.storage_deposit(None, None);
        assert_eq!(to_yocto("0.03"), contract.get_user_storage_state(accounts(3)).unwrap().deposit.0);
        deposit_tokens(&mut context, &mut contract, accounts(3), token_amounts.clone());
        deposit_tokens(&mut context, &mut contract, accounts(0), vec![]);

        let predict = contract.predict_add_stable_liquidity(pool_id, &vec![to_yocto("4").into(), to_yocto("4").into()]);
        testing_env!(context
            .predecessor_account_id(accounts(3))
            .attached_deposit(to_yocto("0.0007"))
            .build());
        let add_liq = contract.add_stable_liquidity(
            pool_id,
            vec![to_yocto("4").into(), to_yocto("4").into()],
            U128(1),
        );
        assert_eq!(predict.0, add_liq.0);
        assert_eq!(100000000, contract.get_pool_share_price(pool_id).0);
        assert_eq!(8000000000000000000000000, contract.get_pool_shares(pool_id, accounts(3)).0);
        assert_eq!(8000000000000000000000000, contract.get_pool_total_shares(pool_id).0);
        
        let expected_out = contract.get_return(0, accounts(1), to_yocto("1").into(), accounts(2));
        assert_eq!(expected_out.0, 996947470156575219215719);

        testing_env!(context
            .predecessor_account_id(accounts(3))
            .attached_deposit(1)
            .build());
        let amount_out = swap(&mut contract, 0, accounts(1), to_yocto("1").into(), accounts(2));
        assert_eq!(amount_out, expected_out.0);
        assert_eq!(
            contract.get_deposit(accounts(3), accounts(1)).0,
            0
        );
        assert_eq!(0, contract.get_deposits(accounts(3)).get(&accounts(1).to_string()).unwrap().0);
        assert_eq!(to_yocto("1") + 996947470156575219215719, contract.get_deposits(accounts(3)).get(&accounts(2).to_string()).unwrap().0);

        let predict = contract.predict_remove_liquidity(pool_id, to_yocto("0.1").into());
        testing_env!(context
            .predecessor_account_id(accounts(3))
            .attached_deposit(1)
            .build());
        let remove_liq = contract.remove_liquidity(
            pool_id,
            to_yocto("0.1").into(),
            vec![1.into(), 1.into()],
        );
        assert_eq!(predict, remove_liq);

        let predict = contract.predict_remove_liquidity_by_tokens(pool_id, &vec![to_yocto("0.1").into(), to_yocto("0.1").into()]);
        testing_env!(context
            .predecessor_account_id(accounts(3))
            .attached_deposit(1)
            .build());
        let remove_liq_by_token = contract.remove_liquidity_by_tokens(
            pool_id,
            vec![to_yocto("0.1").into(), to_yocto("0.1").into()],
            to_yocto("1").into(),
        );
        assert_eq!(predict.0, remove_liq_by_token.0);

        testing_env!(context.predecessor_account_id(accounts(0)).attached_deposit(1).build());
        contract.remove_exchange_fee_liquidity(0, to_yocto("0.0001").into(), vec![1.into(), 1.into()]);
        testing_env!(context.predecessor_account_id(accounts(0)).attached_deposit(1).build());
        contract.withdraw_owner_token(accounts(1), to_yocto("0.00001").into(), None);
        testing_env!(context.predecessor_account_id(accounts(0)).block_timestamp(2*86400 * 1_000_000_000).attached_deposit(1).build());
        contract.stable_swap_ramp_amp(0,250, (3*86400 * 1_000_000_000).into());
        testing_env!(context.predecessor_account_id(accounts(0)).attached_deposit(1).build());
        contract.stable_swap_stop_ramp_amp(0);
    }


    #[test]
    fn test_rated() {
        let (mut context, mut contract) = setup_contract();
        let token_amounts = vec![(accounts(1), to_yocto("3")), (accounts(2), to_yocto("5"))];
        let tokens = token_amounts
            .iter()
            .map(|(x, _)| x.clone())
            .collect::<Vec<_>>();
        testing_env!(context.predecessor_account_id(accounts(0)).attached_deposit(1).build());
        contract.extend_whitelisted_tokens(tokens.clone());
        assert_eq!(contract.get_whitelisted_tokens(), vec![accounts(1).to_string(), accounts(2).to_string()]);
        assert_eq!(0, contract.get_user_whitelisted_tokens(accounts(3)).len());
        testing_env!(context
            .predecessor_account_id(accounts(0))
            .attached_deposit(env::storage_byte_cost() * 512) // required storage depends on contract_id length
            .build());
        let pool_id = contract.add_rated_swap_pool(tokens, vec![18, 18], 25, 240);
        println!("{:?}", contract.version());
        println!("{:?}", contract.get_rated_pool(pool_id));
        println!("{:?}", contract.get_pools(0, 100));
        println!("{:?}", contract.get_pool(0));
        assert_eq!(1, contract.get_number_of_pools());
        assert_eq!(25, contract.get_pool_fee(pool_id));
        testing_env!(context
            .predecessor_account_id(accounts(3))
            .attached_deposit(to_yocto("0.03"))
            .build());
        contract.storage_deposit(None, None);
        assert_eq!(to_yocto("0.03"), contract.get_user_storage_state(accounts(3)).unwrap().deposit.0);
        deposit_tokens(&mut context, &mut contract, accounts(3), token_amounts.clone());
        deposit_tokens(&mut context, &mut contract, accounts(0), vec![]);

        // set token1/token2 rate = 2.0
        let cross_call_result = near_sdk::serde_json::to_vec(&U128(2_000000000000000000000000)).unwrap();
        if let Some(mut rate) = global_get_rate(accounts(1).as_ref()) {
            rate.set(&cross_call_result);
            global_set_rate(accounts(1).as_ref(), &rate);
        }

        let pool_info = contract.get_rated_pool(pool_id);
        assert_eq!(pool_info.rates, vec![U128(2_000000000000000000000000), U128(1_000000000000000000000000)]);

        let predict = contract.predict_add_rated_liquidity(pool_id, &vec![to_yocto("2").into(), to_yocto("4").into()], &Some(pool_info.rates.clone()));
        testing_env!(context
            .predecessor_account_id(accounts(3))
            .attached_deposit(to_yocto("0.0007"))
            .build());
        let add_liq = contract.add_stable_liquidity(
            pool_id,
            vec![to_yocto("2").into(), to_yocto("4").into()],
            U128(1),
        );
        assert_eq!(predict.0, add_liq.0);
        assert_eq!(100000000, contract.get_pool_share_price(pool_id).0);
        assert_eq!(8000000000000000000000000000000, contract.get_pool_shares(pool_id, accounts(3)).0);
        assert_eq!(8000000000000000000000000000000, contract.get_pool_total_shares(pool_id).0);
        
        let expected_out = contract.get_rated_return(0, accounts(1), to_yocto("1").into(), accounts(2), &Some(pool_info.rates.clone()));
        assert_eq!(expected_out.0, 1992244454139326876254354);

        testing_env!(context
            .predecessor_account_id(accounts(3))
            .attached_deposit(1)
            .build());
        let amount_out = swap(&mut contract, 0, accounts(1), to_yocto("1").into(), accounts(2));
        assert_eq!(amount_out, expected_out.0);
        assert_eq!(
            contract.get_deposit(accounts(3), accounts(1)).0,
            0
        );
        assert_eq!(0, contract.get_deposits(accounts(3)).get(&accounts(1).to_string()).unwrap().0);
        assert_eq!(to_yocto("1") + 1992244454139326876254354, contract.get_deposits(accounts(3)).get(&accounts(2).to_string()).unwrap().0);

        let predict = contract.predict_remove_liquidity(pool_id, to_yocto("0.1").into());
        testing_env!(context
            .predecessor_account_id(accounts(3))
            .attached_deposit(1)
            .build());
        let remove_liq = contract.remove_liquidity(
            pool_id,
            to_yocto("0.1").into(),
            vec![1.into(), 1.into()],
        );
        assert_eq!(predict, remove_liq);

        let predict = contract.predict_remove_rated_liquidity_by_tokens(pool_id, &vec![to_yocto("0.1").into(), to_yocto("0.1").into()], &Some(pool_info.rates));
        testing_env!(context
            .predecessor_account_id(accounts(3))
            .attached_deposit(1)
            .build());
        let remove_liq_by_token = contract.remove_liquidity_by_tokens(
            pool_id,
            vec![to_yocto("0.1").into(), to_yocto("0.1").into()],
            to_yocto("1000000").into(),
        );
        assert_eq!(predict.0, remove_liq_by_token.0);

        testing_env!(context.predecessor_account_id(accounts(0)).attached_deposit(1).build());
        contract.remove_exchange_fee_liquidity(0, to_yocto("100").into(), vec![1.into(), 1.into()]);
        testing_env!(context.predecessor_account_id(accounts(0)).attached_deposit(1).build());
        contract.withdraw_owner_token(accounts(1), to_yocto("0.00001").into(), None);
        testing_env!(context.predecessor_account_id(accounts(0)).block_timestamp(2*86400 * 1_000_000_000).attached_deposit(1).build());
        contract.stable_swap_ramp_amp(0,250, (3*86400 * 1_000_000_000).into());
        testing_env!(context.predecessor_account_id(accounts(0)).attached_deposit(1).build());
        contract.stable_swap_stop_ramp_amp(0);
    }

    #[test]
    fn test_owner(){
        let (mut context, mut contract) = setup_contract();
        testing_env!(context.predecessor_account_id(accounts(0)).attached_deposit(1).build());
        contract.set_owner(accounts(1));
        assert_eq!(accounts(1).to_string(), contract.get_owner());
        testing_env!(context.predecessor_account_id(accounts(1)).attached_deposit(1).build());
        contract.retrieve_unmanaged_token(accounts(2), U128(1));
        testing_env!(context.predecessor_account_id(accounts(1)).attached_deposit(1).build());
        contract.extend_guardians(vec![accounts(2)]);
        assert_eq!(vec![accounts(2).to_string()], contract.get_guardians());
        testing_env!(context.predecessor_account_id(accounts(1)).attached_deposit(1).build());
        contract.remove_guardians(vec![accounts(2)]);
        assert_eq!(0, contract.get_guardians().len());
        assert_eq!(RunningState::Running, contract.metadata().state);
        testing_env!(context.predecessor_account_id(accounts(1)).attached_deposit(1).build());
        contract.change_state(RunningState::Paused);
        assert_eq!(RunningState::Paused, contract.metadata().state);
        assert_eq!(2000, contract.metadata().admin_fee_bps);
        testing_env!(context.predecessor_account_id(accounts(1)).attached_deposit(1).build());
        contract.modify_admin_fee(70);
        assert_eq!(70, contract.metadata().admin_fee_bps);
    }

    #[test]
    fn test_view_pool() {
        let (mut context, mut contract) = setup_contract();
        // add liquidity of (1,2) tokens
        create_pool_with_liquidity(
            &mut context,
            &mut contract,
            accounts(3),
            vec![(accounts(1), to_yocto("5")), (accounts(2), to_yocto("10"))],
        );
        testing_env!(context
            .predecessor_account_id(accounts(0))
            .attached_deposit(env::storage_byte_cost() * 513)
            .build());
        contract.add_stable_swap_pool(vec![accounts(4), accounts(5)], vec![18, 18], 25, 240);
        testing_env!(context
            .predecessor_account_id(accounts(0))
            .attached_deposit(env::storage_byte_cost() * 513) // required storage depends on contract_id length
            .build());
        contract.add_rated_swap_pool(vec![accounts(4), accounts(5)], vec![18, 18], 25, 240);

        println!("{:?}", contract.get_pools(0, 100));
        println!("{:?}", contract.get_pool(0));
        println!("{:?}", contract.get_pool_by_ids(vec![0,2]));
    }

    #[test]
    fn test_twap() {
        let (mut context, mut contract) = setup_contract();
        let token_amounts = vec![(accounts(1), to_yocto("5")), (accounts(2), to_yocto("5"))];
        let tokens = token_amounts
            .iter()
            .map(|(x, _)| x.clone())
            .collect::<Vec<_>>();
        testing_env!(context.predecessor_account_id(accounts(0)).attached_deposit(1).build());
        contract.extend_whitelisted_tokens(tokens.clone());
        assert_eq!(contract.get_whitelisted_tokens(), vec![accounts(1).to_string(), accounts(2).to_string()]);
        assert_eq!(0, contract.get_user_whitelisted_tokens(accounts(3)).len());
        testing_env!(context
            .predecessor_account_id(accounts(0))
            .attached_deposit(env::storage_byte_cost() * 512)
            .build());
        let pool_id = contract.add_stable_swap_pool(tokens, vec![18, 18], 25, 240);
        
        testing_env!(context
            .predecessor_account_id(accounts(3))
            .attached_deposit(to_yocto("0.03"))
            .build());
        contract.storage_deposit(None, None);
        assert_eq!(to_yocto("0.03"), contract.get_user_storage_state(accounts(3)).unwrap().deposit.0);
        deposit_tokens(&mut context, &mut contract, accounts(3), token_amounts.clone());
        deposit_tokens(&mut context, &mut contract, accounts(0), vec![]);

        testing_env!(context
            .predecessor_account_id(accounts(3))
            .attached_deposit(to_yocto("0.0007"))
            .build());
        contract.add_stable_liquidity(
            pool_id,
            vec![to_yocto("4").into(), to_yocto("4").into()],
            U128(1),
        );
        testing_env!(context.predecessor_account_id(accounts(0)).attached_deposit(1).build());
        contract.register_pool_twap_record(pool_id);

        testing_env!(context.predecessor_account_id(accounts(0)).attached_deposit(1).build());
        contract.modify_cumulative_info_record_interval_sec(0);
        for i in 1..RECORD_COUNT_LIMIT - 1 {
            testing_env!(context.block_timestamp(i as u64 * 10u64.pow(9)).build());
            contract.sync_pool_twap_record(pool_id);
            assert_eq!(i + 1, contract.get_pool_twap_info_view(pool_id).unwrap().records.len());
            assert!(contract.get_unit_share_twap_token_amounts(pool_id).is_none());
        }
        testing_env!(context.block_timestamp((RECORD_COUNT_LIMIT - 1) as u64 * 10u64.pow(9)).build());
        contract.sync_pool_twap_record(pool_id);
        assert_eq!(RECORD_COUNT_LIMIT, contract.get_pool_twap_info_view(pool_id).unwrap().records.len());
        assert!(contract.get_unit_share_twap_token_amounts(pool_id).is_some());
    }
}
