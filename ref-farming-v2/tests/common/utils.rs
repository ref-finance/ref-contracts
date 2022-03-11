#![allow(unused)] 
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

pub(crate) fn generate_cd_account_msg(index: u64, cd_strategy: usize) -> String{
    format!("{{\"NewCDAccount\": {{\"index\": {}, \"cd_strategy\": {}}}}}", index, cd_strategy)
}

pub(crate) fn append_cd_account_msg(index: u64) -> String{
    format!("{{\"AppendCDAccount\": {{\"index\": {}}}}}", index)
}

pub(crate) fn generate_reward_msg(farm_id: String) -> String{
    format!("{{\"Reward\": {{\"farm_id\": \"{}\"}}}}", farm_id)
}

pub(crate) fn to_nano(timestamp: u32) -> u64 {
    u64::from(timestamp) * 10u64.pow(9)
}

pub(crate) fn to_sec(timestamp: u64) -> u32 {
    (timestamp / 10u64.pow(9)) as u32
}

#[macro_export]
macro_rules! generate_user_account{
    ($root: ident, $owner: ident) => {
        let $root = init_simulator(None);
        let $owner = $root.create_user(stringify!($owner).to_string(), to_yocto("100"));
    };
    ($root: ident, $owner: ident, $($name: ident),*)=>{
        let $root = init_simulator(None);
        let $owner = $root.create_user(stringify!($owner).to_string(), to_yocto("100"));
        $(
            let $name = $root.create_user(stringify!($name).to_string(), to_yocto("100"));
        )*
    };
}

#[macro_export]
macro_rules! assert_err{
    (print $exec_func: expr)=>{
        println!("{:?}", $exec_func.promise_errors()[0].as_ref().unwrap().status());
    };
    ($exec_func: expr, $err_info: expr)=>{
        assert!(format!("{:?}", $exec_func.promise_errors()[0].as_ref().unwrap().status()).contains($err_info));
    };
}