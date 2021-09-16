use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{serde_json, PromiseOrValue};
use std::collections::HashMap;

use crate::*;

/// Message parameters to receive via token function call.
#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[serde(untagged)]
enum TokenReceiverMessage {
    /// Alternative to deposit + execute actions call.
    Execute {
        referral_id: Option<ValidAccountId>,
        /// If force != 0, doesn't require user to even have account. In case of failure to deposit to the user's outgoing balance, tokens will be returned to the exchange and can be "saved" via governance.
        /// If force == 0, the account for this user still have been registered. If deposit of outgoing tokens will fail, it will deposit it back into the account.
        force: u8,
        /// List of sequential actions.
        actions: Vec<Action>,
    },
}

impl Contract {
    /// Executes set of actions on potentially virtual account.
    /// Returns amounts to send to the sender directly.
    fn internal_direct_actions(
        &mut self,
        token_in: AccountId,
        amount_in: Balance,
        sender_id: &AccountId,
        force: bool,
        referral_id: Option<AccountId>,
        actions: &[Action],
    ) -> Vec<(AccountId, Balance)> {
        // [AUDIT_12] always save back account for a resident user
        let mut is_resident_user: bool = true;

        let mut account: Account = self.internal_get_account(sender_id).unwrap_or_else(|| {
            is_resident_user = false;
            if !force {
                env::panic(ERR10_ACC_NOT_REGISTERED.as_bytes());
            } else {
                Account::new(sender_id)
            }
        });

        let tokens_snapshot: HashMap<String, u128> = account
            .tokens
            .iter()
            .map(|(token, balance)| (token, balance))
            .collect();

        account.deposit(&token_in, amount_in);
        let _ = self.internal_execute_actions(
            &mut account,
            &referral_id,
            &actions,
            // [AUDIT_02]
            ActionResult::Amount(U128(amount_in)),
        );

        let mut result = vec![];
        for (token, amount) in account.tokens.to_vec() {
            if let Some(initial_amount) = tokens_snapshot.get(&token) {
                // restore token balance to original state if have more
                // but keep cur state if have less
                if amount > *initial_amount {
                    result.push((token.clone(), amount - *initial_amount));
                    account.tokens.insert(&token, initial_amount);
                }
            } else {
                // this token not in original state
                if amount > 0 {
                    result.push((token.clone(), amount));
                }
                // should keep it unregistered
                account.tokens.remove(&token);
            }
        }
        // [AUDIT_12] always save back account for a resident user
        if is_resident_user {
            // To avoid race conditions, we actually going to insert 0 to all changed tokens and save that.
            // for instant swap, we won't increase any storage, so direct save without storage check
            self.accounts.insert(sender_id, &account.into());
        }
        result
    }
}

#[near_bindgen]
impl FungibleTokenReceiver for Contract {
    /// Callback on receiving tokens by this contract.
    /// `msg` format is either "" for deposit or `TokenReceiverMessage`.
    fn ft_on_transfer(
        &mut self,
        sender_id: ValidAccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128> {
        assert!(is_contract_running(&self.state), "{}", ERR51_CONTRACT_PAUSED);
        let token_in = env::predecessor_account_id();
        if msg.is_empty() {
            // Simple deposit.
            self.internal_deposit(sender_id.as_ref(), &token_in, amount.into());
            PromiseOrValue::Value(U128(0))
        } else {
            // instant swap
            let message =
                serde_json::from_str::<TokenReceiverMessage>(&msg).expect(ERR28_WRONG_MSG_FORMAT);
            match message {
                TokenReceiverMessage::Execute {
                    referral_id,
                    force,
                    actions,
                } => {
                    let referral_id = referral_id.map(|x| x.to_string());
                    let out_amounts = self.internal_direct_actions(
                        token_in,
                        amount.0,
                        sender_id.as_ref(),
                        force != 0,
                        referral_id,
                        &actions,
                    );
                    for (token_out, amount_out) in out_amounts.into_iter() {
                        self.internal_send_tokens(sender_id.as_ref(), &token_out, amount_out);
                    }
                    // Even if send tokens fails, we don't return funds back to sender.
                    PromiseOrValue::Value(U128(0))
                }
            }
        }
    }
}
