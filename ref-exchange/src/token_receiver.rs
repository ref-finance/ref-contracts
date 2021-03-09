use crate::*;
use near_sdk::PromiseOrValue;

use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;

#[near_bindgen]
impl FungibleTokenReceiver for Contract {
    /// Callback on receiving tokens by this contract.
    fn ft_on_transfer(
        &mut self,
        sender_id: ValidAccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128> {
        let token_in = env::predecessor_account_id();
        assert!(msg.is_empty(), "ERR_MSG_INCORRECT");
        self.internal_deposit(sender_id.as_ref(), &token_in, amount.into());
        PromiseOrValue::Value(U128(0))
    }
}
