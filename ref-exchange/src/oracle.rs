use crate::utils::{u128_dec_format, u64_dec_format, to_nano};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{ext_contract, Balance, Timestamp};

pub mod price_oracle {
    use super::*;

    type AssetId = String;

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

    impl PriceData {
        pub fn assert_valid(&self, timestamp: u64, maximum_recency_duration_sec: u32, maximum_staleness_duration_sec: u32) {
            assert!(
                self.recency_duration_sec <= maximum_recency_duration_sec,
                "Recency duration in the oracle call is larger than allowed maximum"
            );
            assert!(
                self.timestamp <= timestamp,
                "Price data timestamp is in the future"
            );
            assert!(
                timestamp - self.timestamp <= to_nano(maximum_staleness_duration_sec),
                "Price data timestamp is too stale"
            );
        }
    }

    #[ext_contract(ext_price_oracle)]
    pub trait ExtPriceOracle {
        fn get_price_data(&self, asset_ids: Option<Vec<AssetId>>) -> PriceData;
    }
}

pub mod pyth_oracle {
    use super::*;
    use near_sdk::json_types::{I64, U64};

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

    impl Price {
        pub fn is_valid(&self, timestamp: u64, pyth_price_valid_duration_sec: u32) -> bool {
            self.price.0 > 0 && 
            self.publish_time > 0 && 
            to_nano(self.publish_time as u32 + pyth_price_valid_duration_sec) >= timestamp
        }
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
        fn list_prices_no_older_than(&self, price_ids: Vec<PriceIdentifier>, age: u64) -> HashMap<PriceIdentifier, Option<Price>>;
    }
}