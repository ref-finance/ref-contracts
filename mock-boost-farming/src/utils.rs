// use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{Balance, Timestamp, Duration, AccountId, Gas, ext_contract};
use near_sdk::json_types::U128;
use crate::errors::{E406_INVALID_FARM_ID, E308_INVALID_SEED_ID};

uint::construct_uint!(
    pub struct U256(4);
);

pub const DEFAULT_SEED_SLASH_RATE: u32 = 200;
pub const DEFAULT_SEED_MIN_LOCKING_DURATION_SEC: DurationSec = 3600 * 24 * 30;
pub const DEFAULT_MAX_NUM_FARMS_PER_SEED: u32 = 32;
pub const DEFAULT_MAX_NUM_FARMS_PER_BOOSTER: u32 = 64;
pub const DEFAULT_MAX_LOCKING_DURATION_SEC: DurationSec = 3600 * 24 * 30 * 12; 
pub const DEFAULT_MAX_LOCKING_REWARD_RATIO: u32 = 20000;
pub const MIN_LOCKING_REWARD_RATIO: u32 = 10000; 
pub const MAX_NUM_SEEDS_PER_BOOSTER: usize = 16;

pub const STORAGE_BALANCE_MIN_BOUND: u128 = 100_000_000_000_000_000_000_000;
pub const TGAS: Gas = 1_000_000_000_000;
pub const GAS_FOR_SEED_TRANSFER: Gas = 20 * TGAS;
pub const GAS_FOR_RESOLVE_SEED_TRANSFER: Gas = 10 * TGAS;
pub const GAS_FOR_REWARD_TRANSFER: Gas = 20 * TGAS;
pub const GAS_FOR_RESOLVE_REWARD_TRANSFER: Gas = 10 * TGAS;

pub const NANOS_PER_DAY: Duration = 24 * 60 * 60 * 10u64.pow(9);
pub const MIN_SEED_DEPOSIT: u128 = 1_000_000_000_000_000_000;
pub const BP_DENOM: u128 = 10000;
pub const MFT_TAG: &str = ":";
pub const SEED_TAG: &str = "@";
pub const FARM_ID_PREFIX: &str = "#";
pub type SeedId = String;
pub type FarmId = String;
pub type DurationSec = u32;

pub const NO_DEPOSIT: Balance = 0;
pub const ONE_YOCTO: Balance = 1;



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

pub fn to_nano(sec: u32) -> Timestamp {
    Timestamp::from(sec) * 10u64.pow(9)
}

pub fn nano_to_sec(nano: u64) -> u32 {
    (nano / 10u64.pow(9)) as u32
}

pub(crate) fn u128_ratio(a: u128, num: u128, denom: u128) -> Balance {
    (U256::from(a) * U256::from(num) / U256::from(denom)).as_u128()
}

pub fn gen_farm_id(seed_id: &SeedId, index: usize) -> FarmId {
    format!("{}{}{}", seed_id, FARM_ID_PREFIX, index)
}

pub fn parse_farm_id(farm_id: &FarmId) -> (SeedId, u32) {
    let pos = farm_id.rfind(FARM_ID_PREFIX).expect(E406_INVALID_FARM_ID);
    let (seed_id, last) = farm_id.split_at(pos);
    (seed_id.to_string(), (last.split_at(1).1).parse::<u32>().unwrap())
}

pub fn parse_seed_id(seed_id: &SeedId) -> (AccountId, Option<String>) {
    let v: Vec<&str> = seed_id.split(SEED_TAG).collect();
    if v.len() == 1 {
        let token: AccountId = v[0].parse().unwrap();
        (token, None)
    } else if v.len() == 2 {
        let token: AccountId = v[0].parse().unwrap();
        (token, Some(v[1].to_string()))
    } else {
        panic!("{}", E308_INVALID_SEED_ID)
    }
}

#[ext_contract(ext_multi_fungible_token)]
pub trait MultiFungibleToken {
    fn mft_transfer(
        &mut self,
        token_id: String,
        receiver_id: AccountId,
        amount: U128,
        memo: Option<String>,
    );
}

#[ext_contract(ext_self)]
pub trait TokenPostActions {
    fn callback_post_withdraw_reward(
        &mut self,
        token_id: AccountId,
        farmer_id: AccountId,
        amount: U128,
    );

    fn callback_withdraw_seed(&mut self, seed_id: SeedId, sender_id: AccountId, amount: U128);

    fn callback_withdraw_seed_slashed(&mut self, seed_id: SeedId, amount: U128);

    fn callback_withdraw_seed_lostfound(&mut self, seed_id: SeedId, sender_id: AccountId, amount: U128);
}

pub fn wrap_mft_token_id(token_id: &str) -> String {
    format!("{}{}", MFT_TAG, token_id)
}
