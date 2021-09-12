use std::collections::HashSet;

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::LookupMap;
use near_sdk::json_types::{ValidAccountId, U128};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{ext_contract, AccountId, Balance, Gas};
use uint::construct_uint;

/// Attach no deposit.
pub const NO_DEPOSIT: u128 = 0;
/// hotfix_insuffient_gas_for_mft_resolve_transfer, increase from 5T to 20T
pub const GAS_FOR_RESOLVE_TRANSFER: Gas = 20_000_000_000_000;

pub const GAS_FOR_FT_TRANSFER_CALL: Gas = 25_000_000_000_000 + GAS_FOR_RESOLVE_TRANSFER;

/// Amount of gas for fungible token transfers.
pub const GAS_FOR_FT_TRANSFER: Gas = 10_000_000_000_000;

/// Fee divisor, allowing to provide fee in bps.
pub const FEE_DIVISOR: u32 = 10_000;

/// Initial shares supply on deposit of liquidity.
pub const INIT_SHARES_SUPPLY: u128 = 1_000_000_000_000_000_000_000_000;

construct_uint! {
    /// 256-bit unsigned integer.
    pub struct U256(4);
}

/// Volume of swap on the given token.
#[derive(Clone, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
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

#[ext_contract(ext_self)]
pub trait RefExchange {
    fn exchange_callback_post_withdraw(
        &mut self,
        token_id: AccountId,
        sender_id: AccountId,
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
    assert_eq!(token_set.len(), tokens.len(), "ERR_TOKEN_DUPLICATES");
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
