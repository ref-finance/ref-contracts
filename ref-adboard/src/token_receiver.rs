use crate::*;
use near_sdk::{ext_contract, PromiseOrValue, PromiseResult};
use near_sdk::json_types::{U128};
use std::num::ParseIntError;
use std::str::FromStr;


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

#[ext_contract(ext_multi_fungible_token)]
pub trait MultiFungibleToken {
    fn mft_transfer(&mut self, token_id: String, receiver_id: AccountId, amount: U128, memo: Option<String>);
}

#[ext_contract(ext_self)]
trait FrameDealerResolver {
    fn on_payment(
        &mut self,
        token_id: String,
        receiver_id: AccountId,
        amount: U128,
    );
}

#[near_bindgen]
impl MFTTokenReceiver for Contract {
    /// REF_AMM calls this to start a frame buying process.
    fn mft_on_transfer(
        &mut self,
        token_id: String,
        sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128> {
 
        assert_eq!(
            env::predecessor_account_id(),
            self.data().amm_id,
            "ERR_ONLY_CAN_BE_CALLED_BY_REF"
        );

        let amount: u128 = amount.into();

        let params = msg.parse::<BuyFrameParams>().expect(&format!("ERR_MSG_INCORRECT"));
        
        self.assert_valid_frame_id(params.frame_id);
        let mut metadata = self.data().frames.get(&params.frame_id).unwrap_or(
            FrameMetadata {
                token_price: self.data().default_sell_balance,
                token_id: self.data().default_token_id.clone(),
                owner: self.data().owner_id.clone(),
                protected_ts: 0,
            }
        );
        let cur_ts = env::block_timestamp();
        if metadata.protected_ts > 0 && metadata.protected_ts < cur_ts {
            env::panic(b"Frame is currently protected")
        }
        assert_eq!(token_id, metadata.token_id, "Invalid token id");
        assert_eq!(amount, metadata.token_price, "Invalid price for frame");
        assert!(self.data().whitelist.contains(&params.token_id), "Token not on whitelist");

        // charge fee
        let fee = (
            U256::from(amount) 
            * U256::from(self.data().trading_fee) 
            / U256::from(FEE_DIVISOR)
        ).as_u128();

        self.handle_payment(&metadata.token_id, &metadata.owner, amount - fee);

        // update metadata
        metadata.owner = sender_id.clone();
        metadata.token_id = params.token_id.clone();
        metadata.token_price = params.sell_balance;
        metadata.protected_ts = env::block_timestamp() + to_nanoseconds(self.data().protected_period);
        self.data_mut().frames.insert(&params.frame_id, &metadata);

        PromiseOrValue::Value(U128(0))
    }

}

#[near_bindgen]
impl Contract {

    pub fn handle_payment(&mut self, token_id: &String, receiver_id: &AccountId, amount: u128) {
        ext_multi_fungible_token::mft_transfer(
            token_id.clone(),
            receiver_id.clone(),
            amount.into(),
            None,
            &self.data().amm_id,
            1,  // one yocto near
            XCC_GAS,
        )
        .then(ext_self::on_payment(
            token_id.clone(),
            receiver_id.clone(),
            amount.into(),
            &env::current_account_id(),
            NO_DEPOSIT,
            XCC_GAS,
        ));
    }


    #[private]
    pub fn on_payment(
        &mut self,
        token_id: String,
        receiver_id: AccountId,
        amount: U128,
    ) {

        assert_eq!(
            env::promise_results_count(),
            1,
            "Expected 1 promise result on payment"
        );
        let amount: Balance = amount.into();
        match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Failed => {
                env::log(
                    format!(
                        "Pay {} with {} {}, Callback Failed.",
                        receiver_id, amount, token_id,
                    )
                    .as_bytes(),
                );
                // Record it to lost_and_found
                self.data_mut().failed_payments.push(
                    &PaymentItem {
                        amount,
                        receiver_id,
                        token_id,
                    }
                );
            },
            PromiseResult::Successful(_) => {
                env::log(
                    format!(
                        "Pay {} with {} {}, Succeed.",
                        receiver_id, amount, token_id,
                    )
                    .as_bytes(),
                );
            }
        };
    }
}


