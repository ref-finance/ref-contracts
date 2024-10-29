use crate::*;
use super::global_get_degen_price_oracle_config;
use super::{degen::DegenTrait, PRECISION};
use crate::errors::ERR126_FAILED_TO_PARSE_RESULT;
use crate::utils::{u128_ratio, u64_dec_format, GAS_FOR_BASIC_OP, NO_DEPOSIT};
use crate::oracle::price_oracle;
use crate::PriceInfo;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{env, serde_json::from_slice, AccountId, Promise};

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone)]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug))]
#[serde(crate = "near_sdk::serde")]
pub struct PriceOracleConfig {
    pub oracle_id: AccountId,
    #[serde(with = "u64_dec_format")]
    pub expire_ts: u64,
    /// The maximum number of seconds expected from the oracle price call.
    pub maximum_recency_duration_sec: u32,
    /// Maximum staleness duration of the price data timestamp.
    /// Because NEAR protocol doesn't implement the gas auction right now, the only reason to
    /// delay the price updates are due to the shard congestion.
    /// This parameter can be updated in the future by the owner.
    pub maximum_staleness_duration_sec: u32,
}

#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub struct PriceOracleDegen {
    pub price_info: Option<PriceInfo>,
    pub token_id: AccountId,
    pub decimals: u8,
}

impl PriceOracleDegen {
    pub fn new(token_id: AccountId, decimals: u8) -> Self {
        Self { 
            price_info: None,
            token_id,
            decimals,
        }
    }
}

impl DegenTrait for PriceOracleDegen {
    fn is_price_valid(&self) -> bool {
        let config = global_get_degen_price_oracle_config();
        env::block_timestamp() <= self.get_price_info().degen_updated_at + config.expire_ts
    }
    fn get_price_info(&self) -> &PriceInfo {
        self.price_info.as_ref().expect(format!("{:?} is not price", self.token_id).as_str())
    }
    fn async_update(&self) -> Promise {
        let config = global_get_degen_price_oracle_config();
        price_oracle::ext_price_oracle::get_price_data(Some(vec![self.token_id.clone(),]), &config.oracle_id, NO_DEPOSIT, GAS_FOR_BASIC_OP)
    }
    fn set_price(&mut self, cross_call_result: &Vec<u8>) -> u128 {
        let prices = from_slice::<price_oracle::PriceData>(cross_call_result).expect(ERR126_FAILED_TO_PARSE_RESULT);
        let timestamp = env::block_timestamp();
        let config = global_get_degen_price_oracle_config();
        prices.assert_valid(timestamp, config.maximum_recency_duration_sec, config.maximum_staleness_duration_sec);
        assert!(prices.prices[0].asset_id == self.token_id, "Invalid price data");
        let token_price = prices.prices[0].price.as_ref().expect("Missing token price");

        let fraction_digits = 10u128.pow((token_price.decimals - self.decimals) as u32);
        let price = u128_ratio(PRECISION, token_price.multiplier, fraction_digits as u128);
        
        self.price_info = Some(PriceInfo {
            stored_degen: price,
            degen_updated_at: timestamp
        });
        price
    }
}

pub const GAS_FOR_BATCH_UPDATE_DEGEN_TOKEN_BY_PRICE_ORACLE_OP: Gas = 10_000_000_000_000;
pub const GAS_FOR_BATCH_UPDATE_DEGEN_TOKEN_BY_PRICE_ORACLE_CALLBACK: Gas = 10_000_000_000_000;

// Batch retrieve the price oracle prices for degen tokens.
pub fn batch_update_degen_token_by_price_oracle(token_id_decimals_map: HashMap<AccountId, u8>) {
    let token_ids = token_id_decimals_map.keys().cloned().collect::<Vec<_>>();
    let config = global_get_degen_price_oracle_config();
    price_oracle::ext_price_oracle::get_price_data(
        Some(token_ids.clone()), 
        &config.oracle_id,
        NO_DEPOSIT, 
        GAS_FOR_BATCH_UPDATE_DEGEN_TOKEN_BY_PRICE_ORACLE_OP
    ).then(ext_self::batch_update_degen_token_by_price_oracle_callback(
            token_id_decimals_map,
            &env::current_account_id(),
            NO_DEPOSIT,
            GAS_FOR_BATCH_UPDATE_DEGEN_TOKEN_BY_PRICE_ORACLE_CALLBACK,
        ));
}

#[near_bindgen]
impl Contract {
    // Invalid tokens do not affect the synchronization of valid tokens, and panic will not impact the swap.
    #[private]
    pub fn batch_update_degen_token_by_price_oracle_callback(&mut self, token_id_decimals_map: HashMap<AccountId, u8>) {
        if let Some(cross_call_result) = near_sdk::promise_result_as_success() {
            let prices = from_slice::<price_oracle::PriceData>(&cross_call_result).expect(ERR126_FAILED_TO_PARSE_RESULT);
            let timestamp = env::block_timestamp();
            let config = global_get_degen_price_oracle_config();
            prices.assert_valid(timestamp, config.maximum_recency_duration_sec, config.maximum_staleness_duration_sec);
            for price_info in prices.prices {
                if let Some(token_price) = price_info.price {
                    let token_id = price_info.asset_id;
                    if let Some(decimals) = token_id_decimals_map.get(&token_id) {
                        let mut degen = global_get_degen(&token_id);
                        let fraction_digits = 10u128.pow((token_price.decimals - decimals) as u32);
                        let price = u128_ratio(PRECISION, token_price.multiplier, fraction_digits as u128);
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