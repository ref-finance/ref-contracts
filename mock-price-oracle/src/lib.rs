use std::collections::HashMap;

use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{near_bindgen, PanicOnDefault};
use near_sdk::{env, Balance, Timestamp};

type AssetId = String;

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Debug, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct Price {
    #[serde(with = "u128_dec_format")]
    pub multiplier: Balance,
    pub decimals: u8,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct AssetOptionalPrice {
    pub asset_id: AssetId,
    pub price: Option<Price>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct PriceData {
    #[serde(with = "u64_dec_format")]
    pub timestamp: Timestamp,
    pub recency_duration_sec: u32,

    pub prices: Vec<AssetOptionalPrice>,
}

#[near_bindgen]
#[derive(BorshSerialize, BorshDeserialize, PanicOnDefault)]
pub struct Contract {
    prices: HashMap<AssetId, Price>
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new() -> Self {
        Self {
            prices: HashMap::new(),
        }
    }

    pub fn set_price_data(&mut self, asset_id: AssetId, price: Price) {
        self.prices.insert(asset_id, price);
    }

    pub fn get_price_data(&self, asset_ids: Option<Vec<AssetId>>) -> PriceData {
        // let asset_ids = asset_ids.unwrap_or(vec![]);
        PriceData {
            timestamp: env::block_timestamp(),
            recency_duration_sec: 90,
            prices: {
                let mut res = vec![];
                if let Some(asset_ids) = asset_ids {
                    for asset_id in asset_ids {
                        res.push(AssetOptionalPrice{
                            asset_id: asset_id.clone(),
                            price: self.prices.get(&asset_id).cloned(),
                        });
                    }
                } else {
                    for (asset_id, price) in self.prices.iter() {
                        res.push(AssetOptionalPrice{
                            asset_id: asset_id.clone(),
                            price: Some(price.clone()),
                        });
                    }
                }
                res
            }
        }
    }
    
}

pub(crate) mod u128_dec_format {
    use near_sdk::serde::de;
    use near_sdk::serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(num: &u128, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&num.to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<u128, D::Error>
    where
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .parse()
            .map_err(de::Error::custom)
    }
}

pub(crate) mod u64_dec_format {
    use near_sdk::serde::de;
    use near_sdk::serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(num: &u64, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&num.to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<u64, D::Error>
    where
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .parse()
            .map_err(de::Error::custom)
    }
}