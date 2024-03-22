use super::{rate::RateTrait, PRECISION};
use crate::errors::{ERR126_FAILED_TO_PARSE_RESULT, ERR128_INVALID_EXTRA_INFO_MSG_FORMAT};
use crate::utils::{to_nano, u128_dec_format, u128_ratio, u64_dec_format, unpair_rated_price_from_vec_u8, GAS_FOR_BASIC_OP, NO_DEPOSIT, U256};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{
    env, ext_contract, serde_json::from_slice, AccountId, Balance, Promise, Timestamp
};

// default expire time is 24 hours
const EXPIRE_TS: u64 = 24 * 3600 * 10u64.pow(9);
const MAX_DURATION_SEC: u32 = 60 * 5;
const MIN_DURATION_SEC: u32 = 10;

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub enum SfraxExtraInfo {
    PriceOracle(price_oracle::PriceOracle),
    PythOracle(pyth_oracle::PythOracle),
}

impl SfraxExtraInfo {
    pub fn assert_valid(&self) {
        match self {
            SfraxExtraInfo::PriceOracle(e) => {
                assert!(e.maximum_staleness_duration_sec >= MIN_DURATION_SEC &&
                    e.maximum_staleness_duration_sec <= MAX_DURATION_SEC,
                    "Invalid maximum_staleness_duration_sec"
                );
            }
            SfraxExtraInfo::PythOracle(e) => {
                assert!(e.pyth_price_valid_duration_sec >= MIN_DURATION_SEC &&
                    e.pyth_price_valid_duration_sec <= MAX_DURATION_SEC,
                    "Invalid pyth_price_valid_duration_sec"
                );
            }
        }
    }
}

#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub struct SfraxRate {
    /// *
    pub stored_rates: Balance,
    /// *
    pub rates_updated_at: u64,
    /// *
    pub contract_id: AccountId,
    /// *
    pub extra_info: SfraxExtraInfo,
}

mod price_oracle {
    use super::*;

    type AssetId = String;

    #[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone)]
    #[serde(crate = "near_sdk::serde")]
    pub struct PriceOracle {
        pub oracle_id: AccountId,
        pub base_contract_id: AccountId,
        /// The maximum number of seconds expected from the oracle price call.
        pub maximum_recency_duration_sec: u32,
        /// Maximum staleness duration of the price data timestamp.
        /// Because NEAR protocol doesn't implement the gas auction right now, the only reason to
        /// delay the price updates are due to the shard congestion.
        /// This parameter can be updated in the future by the owner.
        pub maximum_staleness_duration_sec: u32,
    }

    #[derive(Serialize, Deserialize, Clone)]
    #[serde(crate = "near_sdk::serde")]
    pub struct Price {
        #[serde(with = "u128_dec_format")]
        pub multiplier: Balance,
        pub decimals: u8,
    }
    
    #[derive(Serialize, Deserialize)]
    #[serde(crate = "near_sdk::serde")]
    pub struct AssetOptionalPrice {
        pub asset_id: AssetId,
        pub price: Option<Price>,
    }
    
    #[derive(Serialize, Deserialize)]
    #[serde(crate = "near_sdk::serde")]
    pub struct PriceData {
        #[serde(with = "u64_dec_format")]
        pub timestamp: Timestamp,
        pub recency_duration_sec: u32,
        pub prices: Vec<AssetOptionalPrice>,
    }

    #[ext_contract(ext_price_oracle)]
    pub trait ExtPriceOracle {
        fn get_price_data(&self, asset_ids: Option<Vec<AssetId>>) -> PriceData;
    }
}

mod pyth_oracle {
    use super::*;
    use near_sdk::json_types::{I64, U64};

    #[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone)]
    #[serde(crate = "near_sdk::serde")]
    pub struct PythOracle {
        pub oracle_id: AccountId,
        pub base_price_identifier: PriceIdentifier,
        pub rate_price_identifier: PriceIdentifier,
        /// The valid duration to pyth price in seconds.
        pub pyth_price_valid_duration_sec: u32,
    }

    #[derive(BorshDeserialize, BorshSerialize, Deserialize, Serialize)]
    #[cfg_attr(not(target_arch = "wasm32"), derive(Debug))]
    #[serde(crate = "near_sdk::serde")]
    pub struct Price {
        pub price: I64,
        /// Confidence interval around the price
        pub conf: U64,
        /// The exponent
        pub expo: i32,
        /// Unix timestamp of when this price was computed
        pub publish_time: i64,
    }

    #[derive(BorshDeserialize, BorshSerialize, PartialEq, Eq, Hash, Clone)]
    #[repr(transparent)]
    pub struct PriceIdentifier(pub [u8; 32]);

    impl<'de> near_sdk::serde::Deserialize<'de> for PriceIdentifier {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: near_sdk::serde::Deserializer<'de>,
        {
            /// A visitor that deserializes a hex string into a 32 byte array.
            struct IdentifierVisitor;

            impl<'de> near_sdk::serde::de::Visitor<'de> for IdentifierVisitor {
                /// Target type for either a hex string or a 32 byte array.
                type Value = [u8; 32];

                fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                    formatter.write_str("a hex string")
                }

                // When given a string, attempt a standard hex decode.
                fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
                where
                    E: near_sdk::serde::de::Error,
                {
                    if value.len() != 64 {
                        return Err(E::custom(format!(
                            "expected a 64 character hex string, got {}",
                            value.len()
                        )));
                    }
                    let mut bytes = [0u8; 32];
                    hex::decode_to_slice(value, &mut bytes).map_err(E::custom)?;
                    Ok(bytes)
                }
            }

            deserializer
                .deserialize_any(IdentifierVisitor)
                .map(PriceIdentifier)
        }
    }

    impl near_sdk::serde::Serialize for PriceIdentifier {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: near_sdk::serde::Serializer,
        {
            serializer.serialize_str(&hex::encode(&self.0))
        }
    }

    impl std::string::ToString for PriceIdentifier {
        fn to_string(&self) -> String {
            hex::encode(&self.0)
        }
    }

    impl std::fmt::Debug for PriceIdentifier {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", hex::encode(&self.0))
        }
    }

    #[ext_contract(ext_pyth_oracle)]
    pub trait ExtPythOracle {
        fn get_price(&self, price_identifier: PriceIdentifier) -> Option<Price>;
    }
}


impl RateTrait for SfraxRate {
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
        match &self.extra_info {
            SfraxExtraInfo::PriceOracle(o) => {
                price_oracle::ext_price_oracle::get_price_data(Some(vec![o.base_contract_id.clone(), self.contract_id.clone()]), &o.oracle_id, NO_DEPOSIT, GAS_FOR_BASIC_OP)
            },
            SfraxExtraInfo::PythOracle(o) => {
                pyth_oracle::ext_pyth_oracle::get_price(o.base_price_identifier.clone(), &o.oracle_id, NO_DEPOSIT, GAS_FOR_BASIC_OP).and(
                    pyth_oracle::ext_pyth_oracle::get_price(o.rate_price_identifier.clone(), &o.oracle_id, NO_DEPOSIT, GAS_FOR_BASIC_OP)
                )
            }
        }
    }
    fn set(&mut self, cross_call_result: &Vec<u8>) -> u128 {
        let timestamp = env::block_timestamp();
        match &self.extra_info {
            SfraxExtraInfo::PriceOracle(o) => {
                if let Ok(prices) = from_slice::<price_oracle::PriceData>(cross_call_result) {
                    assert!(
                        prices.recency_duration_sec <= o.maximum_recency_duration_sec,
                        "Recency duration in the oracle call is larger than allowed maximum"
                    );
                    assert!(
                        prices.timestamp <= timestamp,
                        "Price data timestamp is in the future"
                    );
                    assert!(
                        timestamp - prices.timestamp <= to_nano(o.maximum_staleness_duration_sec),
                        "Price data timestamp is too stale"
                    );
                    assert!(prices.prices[0].asset_id == o.base_contract_id && prices.prices[1].asset_id == self.contract_id, "");
                    let base_price = prices.prices[0].price.as_ref().expect("Missing base token price");
                    let rate_price = prices.prices[1].price.as_ref().expect("Missing rate token price");
                    assert!(base_price.decimals == rate_price.decimals, "Token decimals inconsistency, base: {}, rate: {}", base_price.decimals, rate_price.decimals);

                    let price = u128_ratio(PRECISION, rate_price.multiplier, base_price.multiplier);
                    self.stored_rates = price;
                    self.rates_updated_at = env::block_timestamp();
                    price
                } else {
                    env::panic(ERR126_FAILED_TO_PARSE_RESULT.as_bytes());
                }
            },
            SfraxExtraInfo::PythOracle(o) => {
                let (base_price_vec_u8, rate_price_vec_u8) = unpair_rated_price_from_vec_u8(cross_call_result);
                let base_price_info = from_slice::<pyth_oracle::Price>(&base_price_vec_u8).expect(ERR126_FAILED_TO_PARSE_RESULT);
                let rate_price_info = from_slice::<pyth_oracle::Price>(&rate_price_vec_u8).expect(ERR126_FAILED_TO_PARSE_RESULT);
                
                assert!(base_price_info.price.0 > 0, "Invalid pyth base price: {}", base_price_info.price.0);
                assert!(rate_price_info.price.0 > 0, "Invalid pyth rate price: {}", rate_price_info.price.0);
                assert!(base_price_info.publish_time > 0 && to_nano(base_price_info.publish_time as u32 + o.pyth_price_valid_duration_sec) >= env::block_timestamp(), "Pyth base price publish_time is too stale");
                assert!(rate_price_info.publish_time > 0 && to_nano(rate_price_info.publish_time as u32 + o.pyth_price_valid_duration_sec) >= env::block_timestamp(), "Pyth rate price publish_time is too stale");

                let base_price = if base_price_info.expo > 0 {
                    U256::from(PRECISION) * U256::from(base_price_info.price.0) * U256::from(10u128.pow(base_price_info.expo.abs() as u32))
                } else {
                    U256::from(PRECISION) * U256::from(base_price_info.price.0) / U256::from(10u128.pow(base_price_info.expo.abs() as u32))
                };

                let rate_price = if rate_price_info.expo > 0 {
                    U256::from(PRECISION) * U256::from(rate_price_info.price.0) * U256::from(10u128.pow(rate_price_info.expo.abs() as u32))
                } else {
                    U256::from(PRECISION) * U256::from(rate_price_info.price.0) / U256::from(10u128.pow(rate_price_info.expo.abs() as u32))
                };

                let price = (U256::from(PRECISION) * rate_price / base_price).as_u128();
                
                self.stored_rates = price;
                self.rates_updated_at = env::block_timestamp();
                price
            }
        }
    }
}

impl SfraxRate {
    pub fn new(contract_id: AccountId, extra_info_string: String) -> Self {
        let extra_info =
                near_sdk::serde_json::from_str::<SfraxExtraInfo>(&extra_info_string).expect(ERR128_INVALID_EXTRA_INFO_MSG_FORMAT);
        extra_info.assert_valid();
        Self {
            stored_rates: PRECISION, 
            rates_updated_at: 0,
            contract_id,
            extra_info,
        }
    }

    pub fn update_extra_info(&mut self, extra_info_string: String) {
        let extra_info =
                near_sdk::serde_json::from_str::<SfraxExtraInfo>(&extra_info_string).expect(ERR128_INVALID_EXTRA_INFO_MSG_FORMAT);
        extra_info.assert_valid();
        self.extra_info = extra_info;
    }
}


