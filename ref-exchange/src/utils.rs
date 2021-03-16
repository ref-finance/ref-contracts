use std::collections::HashSet;

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::LookupMap;
use near_sdk::json_types::{ValidAccountId, U128};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{ext_contract, AccountId, Balance, Gas};
use uint::construct_uint;

/// Amount of gas for fungible token transfers.
pub const GAS_FOR_FT_TRANSFER: Gas = 10_000_000_000_000;

/// Amount of gas used for upgrade function itself.
pub const GAS_FOR_UPGRADE_CALL: Gas = 50_000_000_000_000;

/// Amount of gas for deploy action.
pub const GAS_FOR_DEPLOY_CALL: Gas = 5_000_000_000_000;

/// Fee divisor, allowing to provide fee in bps.
pub const FEE_DIVISOR: u32 = 10_000;

/// Initial shares supply on deposit of liquidity.
pub const INIT_SHARES_SUPPLY: u128 = 1_000_000_000_000_000_000_000_000;

pub const ERR_NOT_REGISTERED: &str = "ERR_NOT_REGISTERED";

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

/// TODO: this should be in the near_standard_contracts
#[ext_contract(ext_fungible_token)]
pub trait FungibleToken {
    fn ft_transfer(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>);
}

/// Adds given value to item stored in the given key in the LookupMap collection.
pub fn add_to_collection(c: &mut LookupMap<AccountId, Balance>, key: &String, value: Balance) {
    let prev_value = c.get(key).unwrap_or(0);
    c.insert(key, &(prev_value + value));
}

/// Checks if there are any duplicates in the given list of tokens.
pub fn check_token_duplicates(tokens: &[ValidAccountId]) {
    let token_set: HashSet<_> = tokens.iter().map(|a| AccountId::from(a.clone())).collect();
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
