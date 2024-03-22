use std::collections::HashMap;

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::{I64, U64};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{near_bindgen, PanicOnDefault};

#[derive(BorshDeserialize, BorshSerialize, Debug, Deserialize, Serialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct PythPrice {
    pub price: I64,
    /// Confidence interval around the price
    pub conf: U64,
    /// The exponent
    pub expo: i32,
    /// Unix timestamp of when this price was computed
    pub publish_time: i64,
}

#[derive(BorshDeserialize, BorshSerialize, PartialOrd, PartialEq, Eq, Hash, Clone)]
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

#[near_bindgen]
#[derive(BorshSerialize, BorshDeserialize, PanicOnDefault)]
pub struct Contract {
    price_info: HashMap<PriceIdentifier, PythPrice>,
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new() -> Self {
        Self {
            price_info: HashMap::new()
        }
    }

    pub fn set_price(&mut self, price_identifier: PriceIdentifier, pyth_price: PythPrice) {
        self.price_info.insert(price_identifier, pyth_price);
    }

    pub fn remove_price(&mut self, price_identifier: PriceIdentifier) {
        self.price_info.remove(&price_identifier);
    }

    pub fn get_price(&self, price_identifier: PriceIdentifier) -> Option<PythPrice> {
        self.price_info.get(&price_identifier).cloned()
    }
}
