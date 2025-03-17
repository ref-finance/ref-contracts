
use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{serde_json, PromiseOrValue};

use crate::*;

pub const VIRTUAL_ACC: &str = "@";

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct AddLiquidityInfo {
    pub pool_id: u64,
    pub amounts: Vec<U128>,
    pub min_amounts: Option<Vec<U128>>,
    pub min_shares: Option<U128>,
}

/// Message parameters to receive via token function call.
#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[serde(untagged)]
enum TokenReceiverMessage {
    /// Alternative to deposit + execute actions call.
    Execute {
        referral_id: Option<ValidAccountId>,
        /// List of sequential actions.
        actions: Vec<Action>,
        /// If not None, use ft_transfer_call
        /// to send token_out back to predecessor with this msg.
        client_echo: Option<String>,
        skip_unwrap_near: Option<bool>,
        swap_out_recipient: Option<ValidAccountId>,
        skip_degen_price_sync: Option<bool>,
    },
    HotZap {
        referral_id: Option<ValidAccountId>,
        hot_zap_actions: Vec<Action>,
        add_liquidity_infos: Vec<AddLiquidityInfo>,
        skip_degen_price_sync: Option<bool>,
    },
}

impl Contract {
    /// Executes set of actions on virtual account.
    /// Returns amounts to send to the sender directly.
    fn internal_direct_actions(
        &mut self,
        token_in: AccountId,
        amount_in: Balance,
        referral_id: Option<AccountId>,
        actions: &[Action],
        skip_degen_price_sync: bool,
    ) -> Vec<(AccountId, Balance)> {

        // let @ be the virtual account
        let mut account: Account = Account::new(&String::from(VIRTUAL_ACC));

        let referral_info :Option<(AccountId, u32)> = referral_id
            .as_ref().and_then(|rid| self.referrals.get(&rid))
            .map(|fee| (referral_id.unwrap().into(), fee));

        account.deposit(&token_in, amount_in);
        let _ = self.internal_execute_actions(
            &mut account,
            &referral_info,
            &actions,
            match actions[0] { 
                Action::Swap(_) => ActionResult::Amount(U128(amount_in)),
                Action::SwapByOutput(_) => ActionResult::None,
            },
            skip_degen_price_sync
        );

        let mut result = vec![];
        for (token, amount) in account.tokens.to_vec() {
            if amount > 0 {
                result.push((token.clone(), amount));
            }
        }
        account.tokens.clear();

        result
    }

}

#[near_bindgen]
impl FungibleTokenReceiver for Contract {
    /// Callback on receiving tokens by this contract.
    /// `msg` format is either "" for deposit or `TokenReceiverMessage`.
    #[allow(unreachable_code)]
    fn ft_on_transfer(
        &mut self,
        sender_id: ValidAccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128> {
        self.assert_contract_running();
        let token_in = env::predecessor_account_id();
        if msg.is_empty() {
            // Simple deposit.
            self.assert_no_frozen_tokens(&[token_in.clone()]);
            self.internal_deposit(sender_id.as_ref(), &token_in, amount.into());
            PromiseOrValue::Value(U128(0))
        } else {
            // instant swap
            let message =
                serde_json::from_str::<TokenReceiverMessage>(&msg).expect(ERR28_WRONG_MSG_FORMAT);
            match message {
                TokenReceiverMessage::Execute {
                    referral_id,
                    actions,
                    client_echo,
                    skip_unwrap_near,
                    swap_out_recipient,
                    skip_degen_price_sync,
                } => {
                    assert!(!(swap_out_recipient.is_some() && client_echo.is_some()), "client_echo and swap_out_recipient cannot have value at the same time");
                    assert_ne!(actions.len(), 0, "{}", ERR72_AT_LEAST_ONE_SWAP);
                    if client_echo.is_some() {
                        assert_client_echo_valid(&token_in, sender_id.as_ref());
                    }
                    let referral_id = referral_id.map(|x| x.to_string());
                    let out_amounts = self.internal_direct_actions(
                        token_in,
                        amount.0,
                        referral_id,
                        &actions,
                        skip_degen_price_sync.unwrap_or(false),
                    );
                    if client_echo.is_some() && sender_id.to_string() == self.burrowland_id {
                        assert!(out_amounts.len() == 1, "Invalid actions, only one out token is allowed");
                    }
                    for (token_out, amount_out) in out_amounts.into_iter() {
                        if let Some(ref message) = client_echo {
                            self.internal_send_token_with_msg(sender_id.as_ref(), &token_out, amount_out, message.clone());
                        } else {
                            self.internal_send_tokens(swap_out_recipient.as_ref().unwrap_or(&sender_id).as_ref(), &token_out, amount_out, skip_unwrap_near);
                        }
                    }
                    // Even if send tokens fails, we don't return funds back to sender.
                    PromiseOrValue::Value(U128(0))
                }
                TokenReceiverMessage::HotZap { 
                    referral_id, 
                    hot_zap_actions, 
                    add_liquidity_infos,
                    skip_degen_price_sync,
                } => {
                    assert!(hot_zap_actions.len() > 0 && add_liquidity_infos.len() > 0);
                    let sender_id: AccountId = sender_id.into();
                    let mut account = self.internal_unwrap_account(&sender_id);      
                    let all_tokens = self.get_hot_zap_tokens(&hot_zap_actions, &add_liquidity_infos);
                    for token_id in all_tokens.iter() {
                        assert!(
                            self.is_whitelisted_token(token_id) 
                                || account.get_balance(token_id).is_some(),
                            "{}",
                            ERR12_TOKEN_NOT_WHITELISTED
                        );
                    }
                    self.assert_no_frozen_tokens(&all_tokens);
                    let referral_id = referral_id.map(|x| x.to_string());
                    let out_amounts = self.internal_direct_actions(
                        token_in,
                        amount.0,
                        referral_id,
                        &hot_zap_actions,
                        skip_degen_price_sync.unwrap_or(false)
                    );

                    let mut token_cache = TokenCache::new();
                    for (out_token_id, out_amount) in out_amounts {
                        token_cache.add(&out_token_id, out_amount);
                    }

                    let prev_storage = env::storage_usage();
                    for add_liquidity_info in add_liquidity_infos {
                        let mut pool = self.pools.get(add_liquidity_info.pool_id).expect(ERR85_NO_POOL);
                        let tokens_in_pool = match &pool {
                            Pool::SimplePool(p) => p.token_account_ids.clone(),
                            Pool::RatedSwapPool(p) => p.token_account_ids.clone(),
                            Pool::StableSwapPool(p) => p.token_account_ids.clone(),
                            Pool::DegenSwapPool(p) => p.token_account_ids.clone(),
                        };
                        
                        let mut add_liquidity_amounts = add_liquidity_info.amounts.iter().map(|v| v.0).collect();

                        match pool {
                            Pool::SimplePool(_) => {
                                pool.add_liquidity(
                                    &sender_id,
                                    &mut add_liquidity_amounts,
                                    false
                                );
                                let min_amounts = add_liquidity_info.min_amounts.expect("Need input min_amounts");
                                // Check that all amounts are above request min amounts in case of front running that changes the exchange rate.
                                for (amount, min_amount) in add_liquidity_amounts.iter().zip(min_amounts.iter()) {
                                    assert!(amount >= &min_amount.0, "{}", ERR86_MIN_AMOUNT);
                                }
                            },
                            Pool::StableSwapPool(_) | Pool::RatedSwapPool(_) | Pool::DegenSwapPool(_) => {
                                let min_shares = add_liquidity_info.min_shares.expect("Need input min_shares");
                                pool.add_stable_liquidity(
                                    &sender_id,
                                    &add_liquidity_amounts,
                                    min_shares.into(),
                                    AdminFees::new(self.admin_fee_bps),
                                    false
                                );
                            }
                        };
                        pool.assert_tvl_not_exceed_limit(add_liquidity_info.pool_id);

                        for (cost_token_id, cost_amount) in tokens_in_pool.iter().zip(add_liquidity_amounts.into_iter()) {
                            token_cache.sub(cost_token_id, cost_amount);
                        }

                        self.pools.replace(add_liquidity_info.pool_id, &pool);
                    }

                    if env::storage_usage() > prev_storage {
                        let storage_cost = (env::storage_usage() - prev_storage) as Balance * env::storage_byte_cost();
                        account.near_amount = account.near_amount.checked_sub(storage_cost).expect(ERR11_INSUFFICIENT_STORAGE);
                    }

                    for (remain_token_id, remain_amount) in token_cache.0.iter() {
                        account.deposit(remain_token_id, *remain_amount);
                    }

                    self.internal_save_account(&sender_id, account);

                    env::log(
                        format!(
                            "HotZap remain internal account assets: {:?}",
                            token_cache.0
                        )
                        .as_bytes(),
                    );

                    PromiseOrValue::Value(U128(0))
                }
            }
        }
    }
}

impl Contract {
    pub fn get_hot_zap_tokens(&self, hot_zap_actions: &Vec<Action>, add_liquidity_infos: &Vec<AddLiquidityInfo>) -> Vec<AccountId> {
        let mut all_tokens = HashSet::new();
        for action in hot_zap_actions {
            all_tokens.extend(action.tokens());
        }
        for add_liquidity_info in add_liquidity_infos {
            let pool = self.pools.get(add_liquidity_info.pool_id).expect(ERR85_NO_POOL);
            all_tokens.extend(pool.tokens().to_vec());
        }
        all_tokens.into_iter().collect::<Vec<_>>()
    }
}