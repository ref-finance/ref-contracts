use crate::utils::{SwapVolume, U256};
use crate::*;

#[derive(Clone, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug))]
pub struct SwapVolumeU256 {
    pub input: U256,
    pub output: U256,
}

impl Default for SwapVolumeU256 {
    fn default() -> Self {
        Self {
            input: U256::from(0),
            output: U256::from(0),
        }
    }
}

impl From<&SwapVolume> for SwapVolumeU256 {
    fn from(sv: &SwapVolume) -> Self {
        SwapVolumeU256 {
            input: U256::from(sv.input.0),
            output: U256::from(sv.output.0),
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug))]
pub struct SwapVolumeU256View {
    pub input: String,
    pub output: String,
}

impl From<SwapVolumeU256> for SwapVolumeU256View {
    fn from(sv_u256: SwapVolumeU256) -> Self {
        SwapVolumeU256View {
            input: sv_u256.input.to_string(),
            output: sv_u256.output.to_string(),
        }
    }
}

pub fn internal_get_swap_volume_u256_vec_or_default(
    pool_id: u64,
    svs: &Vec<SwapVolume>,
) -> Vec<SwapVolumeU256> {
    let swap_volume_u256_map = LookupMap::new(SWAP_VOLUME_KEY.as_bytes());
    match swap_volume_u256_map.get(&(pool_id as u32)) {
        Some(sv_u256s) => sv_u256s,
        None => svs.iter().map(|v| v.into()).collect(),
    }
}

pub fn internal_set_swap_volume_u256_vec(pool_id: u64, sv_u256s: Vec<SwapVolumeU256>) {
    let mut swap_volume_u256_map = LookupMap::new(SWAP_VOLUME_KEY.as_bytes());
    swap_volume_u256_map.insert(&(pool_id as u32), &sv_u256s);
}

pub fn internal_update_swap_volume_u256_vec(
    pool_id: u64,
    update_input_idx: usize,
    update_output_idx: usize,
    amount_in: u128,
    amount_out: u128,
    mut sv_u256s: Vec<SwapVolumeU256>,
) {
    sv_u256s[update_input_idx].input = sv_u256s[update_input_idx]
        .input
        .checked_add(amount_in.into())
        .expect(
            format!(
                "pool {} update idx {} input swap volume u256 overflow",
                pool_id, update_input_idx
            )
            .as_str(),
        );
    sv_u256s[update_output_idx].output = sv_u256s[update_output_idx]
        .output
        .checked_add(amount_out.into())
        .expect(
            format!(
                "pool {} update idx {} output swap volume u256 overflow",
                pool_id, update_output_idx
            )
            .as_str(),
        );
    internal_set_swap_volume_u256_vec(pool_id, sv_u256s);
}
