use near_sdk::{env, Gas};
// use crate::*;
use crate::Contract;
use uint::construct_uint;

construct_uint! {
    /// 256-bit unsigned integer.
    pub struct U256(4);
}

pub const FEE_DIVISOR: u32 = 10_000;
pub type FrameId = u16;
pub const ONE_NEAR: u128 = 1_000_000_000_000_000_000_000_000;
pub const XCC_GAS: Gas = 30_000_000_000_000;
pub const NO_DEPOSIT: u128 = 0;
pub const CONTRACT_SELF: &str = "ref-adboard.near";
pub const AMM_CONTRACT: &str = "ref-playground.near";
pub const WRAPPED_NEAR_CONTRACT: &str = "wrap.near";
pub const PROTECTED_TIMER: u64 = 60 * 60 * 1000 * 1000 * 1000; //15 minutes
pub const DEFAULT_DATA: &str = "WzIzNCzfBN8E1AQ23wTfBMkE31DfBN9Q3wTfBN9Q3wTfUN8o3wTfUN8E31DfUN8E31DfBN8E31DfBN9Q3wTfBN9Q3wTfUN8o3wTfUN8E31DfUN8E31DfBN8E31DfBN9Q3wTfBN9Q3wTfUN8E3wTPBF0";


impl Contract {

    pub(crate) fn assert_owner(&self) {
        assert_eq!(
            env::predecessor_account_id(),
            self.data().owner_id,
            "ERR_NOT_ALLOWED"
        );
    }

    pub(crate) fn assert_valid_frame_id(&self, frame_id: FrameId) {
        let frame_count = self.data().frame_count;
        assert!(frame_id < frame_count, "ERR_INVALID_FRAMEID");
    }
}

