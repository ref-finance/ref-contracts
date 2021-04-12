use near_sdk::json_types::{ValidAccountId, U128};
use near_sdk::{ext_contract, near_bindgen, PromiseOrValue};

use crate::utils::{GAS_FOR_FT_TRANSFER_CALL, GAS_FOR_RESOLVE_TRANSFER, NO_DEPOSIT};
use crate::*;

#[ext_contract(ext_self)]
trait ShareTokenResolver {
    fn share_resolve_transfer(
        &mut self,
        pool_id: u64,
        sender_id: AccountId,
        receiver_id: AccountId,
        amount: U128,
    ) -> U128;
}

#[ext_contract(ext_share_token_receiver)]
pub trait ShareTokenReceiver {
    fn share_on_transfer(
        &mut self,
        pool_id: u64,
        sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128>;
}

#[near_bindgen]
impl Contract {
    /// Transfer pool LP shares, if pool supports.
    #[payable]
    pub fn share_transfer(
        &mut self,
        pool_id: u64,
        receiver_id: &ValidAccountId,
        amount: U128,
        memo: Option<String>,
    ) {
        assert_one_yocto();
        let sender_id = env::predecessor_account_id();
        let mut pool = self.pools.get(pool_id).expect("ERR_NO_POOL");
        pool.share_transfer(receiver_id.as_ref(), amount.0);
        self.pools.replace(pool_id, &pool);
        log!(
            "Transfer shares {} pool: {} from {} to {}",
            pool_id,
            amount.0,
            sender_id,
            receiver_id
        );
        if let Some(memo) = memo {
            log!("Memo: {}", memo);
        }
    }

    #[payable]
    pub fn share_transfer_call(
        &mut self,
        pool_id: u64,
        receiver_id: ValidAccountId,
        amount: U128,
        memo: Option<String>,
        msg: String,
    ) -> PromiseOrValue<U128> {
        assert_one_yocto();
        let sender_id = env::predecessor_account_id();
        let mut pool = self.pools.get(pool_id).expect("ERR_NO_POOL");
        pool.share_withdraw(&sender_id, amount.0);
        if let Some(memo) = memo {
            log!("Memo: {}", memo);
        }
        ext_share_token_receiver::share_on_transfer(
            pool_id,
            sender_id.clone(),
            amount,
            msg,
            receiver_id.as_ref(),
            NO_DEPOSIT,
            env::prepaid_gas() - GAS_FOR_FT_TRANSFER_CALL,
        )
        .then(ext_self::share_resolve_transfer(
            pool_id,
            sender_id,
            receiver_id.into(),
            amount,
            &env::current_account_id(),
            NO_DEPOSIT,
            GAS_FOR_RESOLVE_TRANSFER,
        ))
        .into()
    }

    #[private]
    pub fn share_resolve_transfer(
        &mut self,
        pool_id: u64,
        sender_id: AccountId,
        receiver_id: &AccountId,
        amount: U128,
    ) -> U128 {
        let unused_amount = match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Successful(value) => {
                if let Ok(unused_amount) = near_sdk::serde_json::from_slice::<U128>(&value) {
                    std::cmp::min(amount.0, unused_amount.0)
                } else {
                    amount.0
                }
            }
            PromiseResult::Failed => amount.0,
        };
        let mut pool = self.pools.get(pool_id).expect("ERR_NO_POOL");
        // TODO: handle storage / account removal / etc.
        if unused_amount > 0 {
            pool.share_deposit(&sender_id, amount.0 - unused_amount);
        }
        pool.share_deposit(&receiver_id, amount.0 - unused_amount);
        // Returns burnt amount.
        U128(0)
    }
}
