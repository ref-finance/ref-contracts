use super::global_get_degen_price_oracle_config;
use super::{degen::DegenTrait, PRECISION};
use crate::errors::ERR126_FAILED_TO_PARSE_RESULT;
use crate::utils::{to_nano, u128_ratio, GAS_FOR_BASIC_OP, NO_DEPOSIT};
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
        assert!(
            prices.recency_duration_sec <= config.maximum_recency_duration_sec,
            "Recency duration in the oracle call is larger than allowed maximum"
        );
        assert!(
            prices.timestamp <= timestamp,
            "Price data timestamp is in the future"
        );
        assert!(
            timestamp - prices.timestamp <= to_nano(config.maximum_staleness_duration_sec),
            "Price data timestamp is too stale"
        );
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