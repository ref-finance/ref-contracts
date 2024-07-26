use crate::*;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use crate::utils::u128_dec_format;

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug))]
pub struct DegenPoolLimitInfo {
    #[serde(with = "u128_dec_format")]
    pub tvl_limit: u128
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug))]
pub enum VDegenPoolLimitInfo {
    Current(DegenPoolLimitInfo),
}

impl From<VDegenPoolLimitInfo> for DegenPoolLimitInfo {
    fn from(v: VDegenPoolLimitInfo) -> Self {
        match v {
            VDegenPoolLimitInfo::Current(c) => c,
        }
    }
}

impl From<DegenPoolLimitInfo> for VDegenPoolLimitInfo {
    fn from(c: DegenPoolLimitInfo) -> Self {
        VDegenPoolLimitInfo::Current(c)
    }
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug))]
pub enum VPoolLimitInfo {
    DegenPoolLimit(VDegenPoolLimitInfo)
}

impl VPoolLimitInfo {
    pub fn get_degen_pool_limit(self) -> DegenPoolLimitInfo {
        match self {
            VPoolLimitInfo::DegenPoolLimit(l) => l.into(),
        }
    }
}

pub fn read_pool_limit_from_storage() -> UnorderedMap<u64, VPoolLimitInfo> {
    if let Some(content) = env::storage_read(POOL_LIMIT.as_bytes()) {
        UnorderedMap::try_from_slice(&content).expect("deserialize pool limit info failed.")
    } else {
        UnorderedMap::new(StorageKey::PoolLimit)
    }
}

pub fn write_pool_limit_to_storage(pool_limit: UnorderedMap<u64, VPoolLimitInfo>) {
    env::storage_write(
        POOL_LIMIT.as_bytes(), 
        &pool_limit.try_to_vec().unwrap(),
    );
}
