/// this file is reserved for future work

use crate::*;
use near_sdk::{ext_contract, PromiseOrValue, PromiseResult};
use near_sdk::json_types::{U128};
use std::num::ParseIntError;
use std::str::FromStr;
use std::convert::TryInto;


pub struct BuyFrameParams {
    pub frame_id: FrameId,
    pub token_id: AccountId,
    pub sell_balance: Balance,
    pub pool_id: u64,
}

impl FromStr for BuyFrameParams {
    type Err = ParseIntError;
    
    /// frame_id||sell_token_id||sell_balance||pool_id
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split("||").collect();
        let frame_id = parts[0].parse::<u16>()?;
        let token_id = parts[1].to_string();
        let sell_balance = parts[2].parse::<u128>()?;
        let pool_id = parts[3].parse::<u64>()?;
        Ok(BuyFrameParams {
            frame_id,
            token_id,
            sell_balance,
            pool_id,
        })
    }
}

pub trait MFTTokenReceiver {
    fn mft_on_transfer(
        &mut self,
        token_id: String,
        sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128>;
}

#[ext_contract(ext_ref_amm)]
pub trait PoolPriceServer {
    fn get_return(
        &self,
        pool_id: u64,
        token_in: ValidAccountId,
        amount_in: U128,
        token_out: ValidAccountId,
    ) -> U128;
}

#[ext_contract(ext_self)]
trait FrameDealerResolver {
    fn on_frame_deal(
        &mut self,
        frame_id: FrameId,
        buyer_id: AccountId,
        msg: String,  // BuyFrameParams
    );
}

#[near_bindgen]
impl MFTTokenReceiver for Contract {
    /// Callback on receiving tokens by this contract.
    fn mft_on_transfer(
        &mut self,
        token_id: String,
        sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128> {
 
        assert_eq!(
            env::predecessor_account_id(),
            AMM_CONTRACT.to_string(),
            "ERR_ONLY_CAN_BE_CALLED_BY_REF"
        );

        let amount: u128 = amount.into();

        let params = msg.parse::<BuyFrameParams>().expect(&format!("ERR_MSG_INCORRECT"));
        
        self.assert_valid_frame_id(params.frame_id);
        let metadata = self.data().frames.get(&params.frame_id).unwrap_or_default();
        let cur_ts = env::block_timestamp();
        if metadata.protected_ts > 0 && metadata.protected_ts < cur_ts {
            env::panic(b"Frame is currently protected")
        }
        assert_eq!(token_id, metadata.token_id, "Invalid token id");
        assert_eq!(amount, metadata.token_price, "Invalid price for frame");
        assert!(self.data().whitelist.contains(&params.token_id), "Token not on whitelist");

        // query price from AMM and using callback to handle results
        ext_ref_amm::get_return(
            params.pool_id,
            token_id.clone().try_into().unwrap(),
            amount.into(),
            params.token_id.clone().try_into().unwrap(),
            &AMM_CONTRACT.to_string(),
            NO_DEPOSIT,
            XCC_GAS,
        )
        .then(ext_self::on_frame_deal(
            params.frame_id,
            sender_id.into(),
            msg.clone(),
            &env::current_account_id(),
            NO_DEPOSIT,
            XCC_GAS,
        ));

        PromiseOrValue::Value(U128(0))
    }

}

#[near_bindgen]
impl Contract {
    #[private]
    pub fn on_frame_deal(
        &mut self,
        frame_id: FrameId,
        buyer_id: AccountId,
        msg: String,  // BuyFrameParams
    ) {
        let params = msg.parse::<BuyFrameParams>().expect(&format!("ERR_MSG_INCORRECT"));
        // seller, token_in, token_in_amount
        // buyer, token_out, token_out_amount
        let token_out_amount = match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Successful(value) => {
                if let Ok(amount) = near_sdk::serde_json::from_slice::<U128>(&value) {
                    amount.0
                } else {
                    0
                }
            }
            PromiseResult::Failed => 0,
        };

        let mut metadata = self.data_mut().frames.get(&params.frame_id).unwrap_or_default();

    }
}


