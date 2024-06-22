use super::global_get_degen_pyth_oracle_config;
use super::{degen::DegenTrait, PRECISION};
use crate::errors::ERR126_FAILED_TO_PARSE_RESULT;
use crate::{pyth_oracle, PriceInfo};
use crate::utils::{to_nano, GAS_FOR_BASIC_OP, NO_DEPOSIT, U256};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{env, serde_json::from_slice, AccountId, Promise};

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone)]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug))]
#[serde(crate = "near_sdk::serde")]
pub struct PythOracleConfig {
    pub oracle_id: AccountId,
    pub expire_ts: u64,
    /// The valid duration to pyth price in seconds.
    pub pyth_price_valid_duration_sec: u32,
}

#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub struct PythOracleDegen {
    pub price_info: Option<PriceInfo>,
    pub price_identifier: pyth_oracle::PriceIdentifier,
}

impl PythOracleDegen {
    pub fn new(price_identifier: pyth_oracle::PriceIdentifier) -> Self {
        Self { 
            price_info: None,
            price_identifier
        }
    }
}

impl DegenTrait for PythOracleDegen {
    fn is_price_valid(&self) -> bool {
        let config = global_get_degen_pyth_oracle_config();
        env::block_timestamp() <= self.get_price_info().degen_updated_at + config.expire_ts
    }
    fn get_price_info(&self) -> &PriceInfo {
        self.price_info.as_ref().expect(format!("{:?} is not price", self.price_identifier).as_str())
    }
    fn async_update(&self) -> Promise {
        let config = global_get_degen_pyth_oracle_config();
        pyth_oracle::ext_pyth_oracle::get_price(self.price_identifier.clone(), &config.oracle_id, NO_DEPOSIT, GAS_FOR_BASIC_OP)
    }
    fn set_price(&mut self, cross_call_result: &Vec<u8>) -> u128 {
        let token_price = from_slice::<pyth_oracle::Price>(&cross_call_result).expect(ERR126_FAILED_TO_PARSE_RESULT);
        let timestamp = env::block_timestamp();
        let config = global_get_degen_pyth_oracle_config();
        assert!(token_price.price.0 > 0, "Invalid pyth price: {}", token_price.price.0);
        assert!(token_price.publish_time > 0 && to_nano(token_price.publish_time as u32 + config.pyth_price_valid_duration_sec) >= timestamp, "Pyth price publish_time is too stale");

        let price = if token_price.expo > 0 {
            U256::from(PRECISION) * U256::from(token_price.price.0) * U256::from(10u128.pow(token_price.expo.abs() as u32))
        } else {
            U256::from(PRECISION) * U256::from(token_price.price.0) / U256::from(10u128.pow(token_price.expo.abs() as u32))
        }.as_u128();

        self.price_info = Some(PriceInfo {
            stored_degen: price,
            degen_updated_at: timestamp
        });
        price
    }
}