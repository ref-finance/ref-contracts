use std::collections::HashMap;

use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{serde_json, PromiseOrValue};

use crate::*;

pub const VIRTUAL_ACC: &str = "@";

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
    },
    HotZap {
        referral_id: Option<ValidAccountId>,
        hot_zap_actions: Vec<Action>,

        pool_id: u64,
        min_amounts: Option<Vec<U128>>,
        min_shares: Option<U128>,
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
            ActionResult::Amount(U128(amount_in)),
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
                } => {
                    let referral_id = referral_id.map(|x| x.to_string());
                    let out_amounts = self.internal_direct_actions(
                        token_in,
                        amount.0,
                        referral_id,
                        &actions,
                    );
                    for (token_out, amount_out) in out_amounts.into_iter() {
                        self.internal_send_tokens(sender_id.as_ref(), &token_out, amount_out);
                    }
                    // Even if send tokens fails, we don't return funds back to sender.
                    PromiseOrValue::Value(U128(0))
                }
                TokenReceiverMessage::HotZap { 
                    referral_id, 
                    hot_zap_actions, 

                    pool_id, 
                    min_amounts,
                    min_shares,
                } => {
                    let sender_id: AccountId = sender_id.into();
                    let mut account = self.internal_unwrap_account(&sender_id);

                    let referral_id = referral_id.map(|x| x.to_string());
                    let out_amounts = self.internal_direct_actions(
                        token_in,
                        amount.0,
                        referral_id,
                        &hot_zap_actions,
                    );

                    let mut remain_assets = HashMap::new(); 
                    for (out_token_id, out_amount) in out_amounts {
                        account.deposit(&out_token_id, out_amount);
                        remain_assets.insert(out_token_id, U128(out_amount));
                    }
                    
                    let mut pool = self.pools.get(pool_id).expect(ERR85_NO_POOL);
                    let tokens_in_pool = match &pool {
                        Pool::SimplePool(p) => p.token_account_ids.clone(),
                        Pool::RatedSwapPool(p) => p.token_account_ids.clone(),
                        Pool::StableSwapPool(p) => p.token_account_ids.clone(),
                    };

                    let mut add_liquidity_amounts = vec![];
                    for token_id in tokens_in_pool.iter() {
                        add_liquidity_amounts.push(
                            remain_assets.get(token_id).expect(&format!("actions result missing token : {:?}", token_id)).0
                        )
                    }

                    match pool {
                        Pool::SimplePool(_) => {
                            pool.add_liquidity(
                                &sender_id,
                                &mut add_liquidity_amounts,
                                false
                            );
                            if let Some(min_amounts) = min_amounts {
                                // Check that all amounts are above request min amounts in case of front running that changes the exchange rate.
                                for (amount, min_amount) in add_liquidity_amounts.iter().zip(min_amounts.iter()) {
                                    assert!(amount >= &min_amount.0, "{}", ERR86_MIN_AMOUNT);
                                }
                            }
                        },
                        Pool::StableSwapPool(_) | Pool::RatedSwapPool(_) => {
                            let min_shares = min_shares.expect("Need input min_shares");
                            pool.add_stable_liquidity(
                                &sender_id,
                                &add_liquidity_amounts,
                                min_shares.into(),
                                AdminFees::new(self.admin_fee_bps),
                                false
                            );
                        }
                    };

                    for i in 0..tokens_in_pool.len() {
                        account.withdraw(&tokens_in_pool[i], add_liquidity_amounts[i]);
                        let amount = remain_assets.remove(&tokens_in_pool[i]).unwrap().0;
                        let remain = amount - add_liquidity_amounts[i];
                        if remain > 0 {
                            remain_assets.insert(tokens_in_pool[i].clone(), U128(remain));
                        }
                    }
                    
                    self.internal_save_account(&sender_id, account);
                    self.pools.replace(pool_id, &pool);

                    env::log(
                        format!(
                            "HotZap remain internal account assets: {:?}",
                            remain_assets
                        )
                        .as_bytes(),
                    );

                    PromiseOrValue::Value(U128(0))
                }
            }
        }
    }
}