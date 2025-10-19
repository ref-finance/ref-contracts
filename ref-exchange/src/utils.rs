use std::collections::{HashSet, HashMap};
use std::convert::TryInto;

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::LookupMap;
use near_sdk::json_types::{ValidAccountId, U128};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{ext_contract, AccountId, Balance, Gas, Timestamp};
use uint::construct_uint;
use crate::errors::*;

/// Attach no deposit.
pub const NO_DEPOSIT: u128 = 0;

/// 10T gas for basic operation
pub const GAS_FOR_BASIC_OP: Gas = 10_000_000_000_000;

/// hotfix_insuffient_gas_for_mft_resolve_transfer.
pub const GAS_FOR_MFT_TRANSFER_CALL: Gas = 25_000_000_000_000;
pub const GAS_FOR_MFT_RESOLVE_TRANSFER: Gas = 20_000_000_000_000;

pub const GAS_FOR_FT_TRANSFER_CALL: Gas = 30_000_000_000_000;
pub const DEFAULT_EXTRA_TGAS: u32 = 15;

/// Amount of gas for fungible token transfers, increased to 20T to support AS token contracts.
pub const GAS_FOR_FT_TRANSFER: Gas = 20_000_000_000_000;
pub const GAS_FOR_NEAR_WITHDRAW: Gas = 20_000_000_000_000;

/// Call back for Near transfer need extra gas
pub const GAS_FOR_CB_NEAR_TRANSFER: Gas = 10_000_000_000_000;
pub const GAS_FOR_CB_FT_TRANSFER: Gas = 5_000_000_000_000;

/// Fee divisor, allowing to provide fee in bps.
pub const FEE_DIVISOR: u32 = 10_000;
pub const MAX_ADMIN_FEE_BPS: u32 = 8_000;

/// Initial shares supply on deposit of liquidity.
pub const INIT_SHARES_SUPPLY: u128 = 1_000_000_000_000_000_000_000_000;

construct_uint! {
    /// 256-bit unsigned integer.
    #[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
    #[serde(crate = "near_sdk::serde")]
    pub struct U256(4);
}

construct_uint! {
    /// 384-bit unsigned integer.
    pub struct U384(6);
}

/// Volume of swap on the given token.
#[derive(Clone, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug))]
pub struct SwapVolume {
    pub input: U128,
    pub output: U128,
}

impl Default for SwapVolume {
    fn default() -> Self {
        Self {
            input: U128(0),
            output: U128(0),
        }
    }
}

#[ext_contract(ext_wrap_near)]
pub trait WrapNear {
    fn near_withdraw(&mut self, amount: U128);
}

#[ext_contract(ext_self)]
pub trait RefExchange {
    fn exchange_callback_post_withdraw_near(
        &mut self,
        sender_id: AccountId,
        amount: U128,
    ) -> U128 ;
    fn exchange_callback_post_withdraw(
        &mut self,
        token_id: AccountId,
        sender_id: AccountId,
        amount: U128,
    );
    fn callback_on_shadow(
        &mut self,
        action: crate::account_deposit::ShadowActions,
        sender_id: AccountId,
        pool_id: u64,
        amount: U128,
        storage_fee: U128
    ) -> bool;
    fn callback_on_burrow_liquidation(
        &mut self,
        sender_id: AccountId,
        pool_id: u64,
        amount: U128,
    );
}

/// Adds given value to item stored in the given key in the LookupMap collection.
pub fn add_to_collection(c: &mut LookupMap<AccountId, Balance>, key: &String, value: Balance) {
    let prev_value = c.get(key).unwrap_or(0);
    c.insert(key, &(prev_value + value));
}

/// Checks if there are any duplicates in the given list of tokens.
pub fn check_token_duplicates(tokens: &[ValidAccountId]) {
    let token_set: HashSet<_> = tokens.iter().map(|a| a.as_ref()).collect();
    assert_eq!(token_set.len(), tokens.len(), "{}", ERR92_TOKEN_DUPLICATES);
}

/// Newton's method of integer square root.
pub fn integer_sqrt(value: U256) -> U256 {
    let mut guess: U256 = (value + U256::one()) >> 1;
    let mut res = value;
    while guess < res {
        res = guess;
        guess = (value / guess + guess) >> 1;
    }
    res
}

pub fn u128_ratio(a: u128, num: u128, denom: u128) -> u128 {
    (U256::from(a) * U256::from(num) / U256::from(denom)).as_u128()
}

pub struct TokenCache(pub HashMap<AccountId, u128>);

impl TokenCache {
    pub fn new() -> Self {
        TokenCache(HashMap::new())
    }

    pub fn add(&mut self, token_id: &AccountId, amount: u128) {
        self.0.entry(token_id.clone()).and_modify(|v| *v += amount).or_insert(amount);
    }

    pub fn sub(&mut self, token_id: &AccountId, amount: u128) {
        if amount != 0 {
            if let Some(prev) = self.0.remove(token_id) {
                assert!(amount <= prev, "{}", ERR22_NOT_ENOUGH_TOKENS);
                let remain = prev - amount;
                if remain > 0 {
                    self.0.insert(token_id.clone(), remain);
                }
            } else {
                panic!("{}", ERR22_NOT_ENOUGH_TOKENS);
            }
        }
    }
}

impl From<TokenCache> for HashMap<AccountId, U128> {
    fn from(v: TokenCache) -> Self {
        v.0.into_iter().map(|(k, v)| (k, U128(v))).collect()
    }
}

pub fn nano_to_sec(nano: u64) -> u32 {
    (nano / 10u64.pow(9)) as u32
}

pub fn to_nano(ts: u32) -> Timestamp {
    Timestamp::from(ts) * 10u64.pow(9)
}

pub fn pair_rated_price_to_vec_u8(price1: Vec<u8>, price2: Vec<u8>) -> Vec<u8> {
    let mut cross_call_result = vec![];
    let offset = (usize::BITS / u8::BITS) as usize;
    cross_call_result.extend((price1.len() + offset).to_be_bytes());
    cross_call_result.extend(price1);
    cross_call_result.extend(price2);
    cross_call_result
}

pub fn unpair_rated_price_from_vec_u8(pair_rated_price: &Vec<u8>) -> (Vec<u8>, Vec<u8>) {
    let offset = (usize::BITS / u8::BITS) as usize;
    let base_price_bytes_len = usize::from_be_bytes(pair_rated_price[0..offset].try_into().unwrap());
    (pair_rated_price[offset..base_price_bytes_len].to_vec(), pair_rated_price[base_price_bytes_len..].to_vec())
}

pub mod u128_dec_format {
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

pub mod u64_dec_format {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sqrt() {
        assert_eq!(integer_sqrt(U256::from(0)), 0.into());
        assert_eq!(integer_sqrt(U256::from(4)), 2.into());
        assert_eq!(
            integer_sqrt(U256::from(1_516_156_330_329u128)),
            U256::from(1_231_323)
        );
    }
}
