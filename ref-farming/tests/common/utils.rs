use near_sdk::AccountId;

pub(crate) fn dai() -> AccountId {
    AccountId::new_unchecked("dai".to_string())
}

pub(crate) fn eth() -> AccountId {
    AccountId::new_unchecked("eth".to_string())
}

pub(crate) fn swap() -> AccountId {
    AccountId::new_unchecked("swap".to_string())
}

pub(crate) fn farming_id() -> AccountId {
    AccountId::new_unchecked("farming".to_string())
}
