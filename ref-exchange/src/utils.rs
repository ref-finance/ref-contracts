use std::collections::HashSet;

use near_sdk::collections::LookupMap;
use near_sdk::json_types::{ValidAccountId, U128};
use near_sdk::{ext_contract, AccountId, Balance, Gas};
use uint::construct_uint;

pub const GAS_FOR_FT_TRANSFER: Gas = 10_000_000_000_000;

construct_uint! {
    /// 256-bit unsigned integer.
    pub struct U256(4);
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
