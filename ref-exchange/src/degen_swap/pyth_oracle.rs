use crate::*;
use super::global_get_degen_pyth_oracle_config;
use super::{degen::DegenTrait, PRECISION};
use crate::errors::ERR126_FAILED_TO_PARSE_RESULT;
use crate::{pyth_oracle, PriceInfo};
use crate::utils::{to_nano, u64_dec_format, GAS_FOR_BASIC_OP, NO_DEPOSIT, U256};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{env, serde_json::from_slice, AccountId, Promise};

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone)]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug))]
#[serde(crate = "near_sdk::serde")]
pub struct PythOracleConfig {
    pub oracle_id: AccountId,
    #[serde(with = "u64_dec_format")]
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

pub const GAS_FOR_BATCH_UPDATE_DEGEN_TOKEN_BY_PYTH_ORACLE_OP: Gas = 15_000_000_000_000;
pub const GAS_FOR_BATCH_UPDATE_DEGEN_TOKEN_BY_PYTH_ORACLE_CALLBACK: Gas = 10_000_000_000_000;

// Batch retrieve the pyth oracle prices for degen tokens.
pub fn batch_update_degen_token_by_pyth_oracle(price_id_token_id_map: HashMap<pyth_oracle::PriceIdentifier, Vec<AccountId>>) {
    let price_ids = price_id_token_id_map.keys().cloned().collect::<Vec<_>>();
    let config = global_get_degen_pyth_oracle_config();
    pyth_oracle::ext_pyth_oracle::list_prices_no_older_than(
        price_ids,
        config.pyth_price_valid_duration_sec as u64,
        &config.oracle_id,
        NO_DEPOSIT,
        GAS_FOR_BATCH_UPDATE_DEGEN_TOKEN_BY_PYTH_ORACLE_OP
    ).then(ext_self::batch_update_degen_token_by_pyth_oracle_callback(
            price_id_token_id_map,
            &env::current_account_id(),
            NO_DEPOSIT,
            GAS_FOR_BATCH_UPDATE_DEGEN_TOKEN_BY_PYTH_ORACLE_CALLBACK,
        ));
}

#[near_bindgen]
impl Contract {
    // Invalid tokens do not affect the synchronization of valid tokens, and panic will not impact the swap.
    #[private]
    pub fn batch_update_degen_token_by_pyth_oracle_callback(&mut self, price_id_token_id_map: HashMap<pyth_oracle::PriceIdentifier, Vec<AccountId>>) {
        if let Some(cross_call_result) = near_sdk::promise_result_as_success() {
            let prices = from_slice::<HashMap<pyth_oracle::PriceIdentifier, Option<pyth_oracle::Price>>>(&cross_call_result).expect(ERR126_FAILED_TO_PARSE_RESULT);
            let timestamp = env::block_timestamp();
            let config = global_get_degen_pyth_oracle_config();
            for (price_id, token_ids) in price_id_token_id_map {
                if let Some(Some(price)) = prices.get(&price_id) {
                    if price.is_valid(timestamp, config.pyth_price_valid_duration_sec) {
                        let price = if price.expo > 0 {
                            U256::from(PRECISION) * U256::from(price.price.0) * U256::from(10u128.pow(price.expo.abs() as u32))
                        } else {
                            U256::from(PRECISION) * U256::from(price.price.0) / U256::from(10u128.pow(price.expo.abs() as u32))
                        }.as_u128();
                        for token_id in token_ids {
                            let mut degen = global_get_degen(&token_id);  
                            degen.update_price_info(PriceInfo {
                                stored_degen: price,
                                degen_updated_at: timestamp
                            });
                            global_set_degen(&token_id, &degen);
                        }
                    }
                }
            }
        }
    }
}