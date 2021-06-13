use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{serde_json, PromiseOrValue};

use crate::*;

/// Message parameters to receive via token function call.
#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[serde(tag = "type")]
enum TokenReceiverMessage {
    Swap {
        referral_id: Option<ValidAccountId>,
        /// If force != 0, doesn't require user to even have account. In case of failure to deposit to the user's outgoing balance, tokens will be returned to the exchange and can be "saved" via governance.
        /// If force == 0, the account for this user still have been registered. If deposit of outgoing tokens will fail, it will deposit it back into the account.
        force: u8,
        pool_id: u64,
        token_out: ValidAccountId,
        min_amount_out: U128,
    },
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
                TokenReceiverMessage::Swap {
                    referral_id,
                    force,
                    pool_id,
                    token_out,
                    min_amount_out,
                } => {
                    let amount_out = self.internal_swap(
                        pool_id,
                        &token_in,
                        amount.0,
                        token_out.as_ref(),
                        min_amount_out.0,
                        &referral_id.map(|x| x.to_string()),
                    );
                    if force == 0 {
                        // If not forced, make sure there is enough deposit to add the token to the account.
                        // To avoid race conditions, we actually going to insert 0 of `token_out`.
                        let mut account = self.get_account_deposits(sender_id.as_ref());
                        account.deposit(token_out.as_ref(), 0);
                        self.accounts.insert(sender_id.as_ref(), &account);
                    }
                    self.internal_send_tokens(sender_id.as_ref(), token_out.as_ref(), amount_out);
                    // Even if send tokens fails, we don't return funds back to sender.
                    PromiseOrValue::Value(U128(0))
                }
            }
        }
    }
}
