use super::{rate::RateTrait, PRECISION};
use crate::errors::ERR126_FAILED_TO_PARSE_RESULT;
use crate::utils::{GAS_FOR_BASIC_OP, NO_DEPOSIT};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{
    env, ext_contract, json_types::U128, serde_json::from_slice, AccountId, Balance, Promise,
};

// default expire time is 24 hours
const EXPIRE_TS: u64 = 24 * 3600 * 10u64.pow(9);

#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub struct LinearRate {
    /// *
    pub stored_rates: Balance,
    /// *
    pub rates_updated_at: u64,
    /// *
    pub contract_id: AccountId,
}

#[ext_contract(ext_linear)]
pub trait ExtLinear {
    //https://github.com/linear-protocol/LiNEAR/blob/f2471444396891fc6c68523c1a9c75e25eaa4637/contracts/linear/src/fungible_token/custom.rs#L10
    fn ft_price(&self) -> U128;
}

impl RateTrait for LinearRate {
    fn are_actual(&self) -> bool {
        env::block_timestamp() <= self.rates_updated_at + EXPIRE_TS  
    }
    fn get(&self) -> Balance {
        self.stored_rates
    }
    fn last_update_ts(&self) -> u64 {
        self.rates_updated_at
    }
    fn async_update(&self) -> Promise {
        ext_linear::ft_price(&self.contract_id, NO_DEPOSIT, GAS_FOR_BASIC_OP)
    }
    fn set(&mut self, cross_call_result: &Vec<u8>) -> u128 {
        if let Ok(U128(price)) = from_slice::<U128>(cross_call_result) {
            self.stored_rates = price;
            self.rates_updated_at = env::block_timestamp();
            price
        } else {
            env::panic(ERR126_FAILED_TO_PARSE_RESULT.as_bytes());
        }
    }
}

impl LinearRate {
    pub fn new(contract_id: AccountId) -> Self {
        Self {
            stored_rates: PRECISION, 
            rates_updated_at: 0,
            contract_id,
        }
    }
}

