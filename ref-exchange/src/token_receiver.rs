use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{serde_json, PromiseOrValue};

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
        let mut initial_account = self.accounts.get(sender_id).unwrap_or_else(|| {
            if !force {
                env::panic(ERR10_ACC_NOT_REGISTERED.as_bytes());
            } else {
                Account::default()
            }
        });
        initial_account.deposit(&token_in, amount_in);
        let mut account = initial_account.clone();
        let _ = self.internal_execute_actions(
            &mut account,
            &referral_id,
            &actions,
            ActionResult::Amount(amount_in),
        );
        let mut result = vec![];
        for (token, amount) in account.tokens.clone().into_iter() {
            let value = initial_account.tokens.get(&token);
            // Remove tokens that were transient from the account.
            if amount == 0 && value.is_none() {
                account.tokens.remove(&token);
            } else {
                let initial_amount = *value.unwrap_or(&0);
                if amount > initial_amount {
                    result.push((token.clone(), amount - initial_amount));
                    account.tokens.insert(token, initial_amount);
                }
            }
        }
        if !force {
            // If not forced, make sure there is enough deposit to add all tokens to the account.
            // To avoid race conditions, we actually going to insert 0 to all changed tokens and save that.
            self.internal_save_account(sender_id, account);
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
        let token_in = env::predecessor_account_id();
        if msg.is_empty() {
            // Simple deposit.
            self.internal_deposit(sender_id.as_ref(), &token_in, amount.into());
            PromiseOrValue::Value(U128(0))
        } else {
            let message =
                serde_json::from_str::<TokenReceiverMessage>(&msg).expect("ERR_MSG_WRONG_FORMAT");
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
