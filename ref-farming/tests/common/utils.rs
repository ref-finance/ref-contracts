use std::convert::TryFrom;
use near_sdk::{AccountId};
use near_sdk::json_types::{ValidAccountId};


pub(crate) fn dai() -> AccountId {
    "dai".to_string()
}

pub(crate) fn eth() -> AccountId {
    "eth".to_string()
}

pub(crate) fn swap() -> AccountId {
    "swap".to_string()
}

pub(crate) fn farming_id() -> AccountId {
    "farming".to_string()
}

pub(crate) fn to_va(a: AccountId) -> ValidAccountId {
    ValidAccountId::try_from(a).unwrap()
}
