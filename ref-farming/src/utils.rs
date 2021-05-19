
use near_sdk::json_types::{U128};
use near_sdk::{env, ext_contract, Gas};
use uint::construct_uint;
use crate::{SeedId, FarmId};
use crate::errors::*;

pub const MAX_ACCOUNT_LENGTH: u128 = 64;
/// Amount of gas for fungible token transfers.
pub const GAS_FOR_FT_TRANSFER: Gas = 10_000_000_000_000;
/// Amount of gas used for upgrade function itself.
pub const GAS_FOR_UPGRADE_CALL: Gas = 50_000_000_000_000;
/// Amount of gas for deploy action.
pub const GAS_FOR_DEPLOY_CALL: Gas = 20_000_000_000_000;
pub const MFT_TAG: &str = "@";


construct_uint! {
    /// 256-bit unsigned integer.
    pub struct U256(4);
}

/// TODO: this should be in the near_standard_contracts
#[ext_contract(ext_fungible_token)]
pub trait FungibleToken {
    fn ft_transfer(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>);
}

/// TODO: this should be in the near_standard_contracts
#[ext_contract(ext_multi_fungible_token)]
pub trait MultiFungibleToken {
    fn mft_transfer(&mut self, token_id: String, receiver_id: AccountId, amount: U128, memo: Option<String>);
}

#[ext_contract(ext_self)]
pub trait TokenPostActions {
    fn callback_post_withdraw_reward(
        &mut self,
        token_id: AccountId,
        sender_id: AccountId,
        amount: U128,
    );

    fn callback_post_withdraw_ft_seed(
        &mut self,
        seed_id: SeedId,
        sender_id: AccountId,
        amount: U128,
    );

    fn callback_post_withdraw_mft_seed(
        &mut self,
        seed_id: SeedId,
        sender_id: AccountId,
        amount: U128,
    );
}

/// Assert that 1 yoctoNEAR was attached.
pub fn assert_one_yocto() {
    assert_eq!(env::attached_deposit(), 1, "Requires attached deposit of exactly 1 yoctoNEAR")
}

// return receiver_id, token_id
pub fn parse_seed_id(lpt_id: &str) -> (String, String) {
    let v: Vec<&str> = lpt_id.split(MFT_TAG).collect();
    if v.len() == 2 { // receiver_id@pool_id
        (v[0].to_string(), v[1].to_string())
    } else if v.len() == 1 { // receiver_id
        (v[0].to_string(), v[0].to_string())
    } else {
        env::panic(format!("{}", ERR33_INVALID_SEED_ID).as_bytes())
    }
}


pub fn parse_farm_id(farm_id: &FarmId) -> (String, usize) {
    let v: Vec<&str> = farm_id.split("#").collect();
    if v.len() != 2 {
        env::panic(format!("{}", ERR42_INVALID_FARM_ID).as_bytes())
    }
    (v[0].to_string(), v[1].parse::<usize>().unwrap())
}

pub fn gen_farm_id(seed_id: &SeedId, index: usize) -> FarmId {
    format!("{}#{}", seed_id, index)
}

