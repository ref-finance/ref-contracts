use near_sdk::{ext_contract, PromiseOrValue};

use crate::*;

use super::{rates::RatesTrait, PRECISION};

#[ext_contract(ext_metapool)]
pub trait ExtMetapool {
    //https://github.com/Narwallets/meta-pool/blob/40636d9004d28dc9cb214b9703692061c93f613c/metapool/src/owner.rs#L254
    fn get_st_near_price(&self) -> U128;
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct StnearRates {
    /// *
    pub stored_rates: Vec<Balance>,
    /// *
    pub rates_updated_at: u64,
    /// *
    pub contract_id: AccountId,
}

impl RatesTrait for StnearRates {
    fn are_actual(&self) -> bool {
        self.rates_updated_at == env::epoch_height()
    }
    fn get(&self) -> &Vec<Balance> {
        &self.stored_rates
    }
    fn update(&self) -> PromiseOrValue<bool> {
        if self.are_actual() {
            return PromiseOrValue::Value(true);
        }

        ext_metapool::get_st_near_price(&self.contract_id, NO_DEPOSIT, gas::BASE).into()
    }
    fn update_callback(&mut self, cross_call_result: &Vec<u8>) -> bool {
        if let Ok(U128(price)) = near_sdk::serde_json::from_slice::<U128>(cross_call_result) {
            self.stored_rates = vec![price, 1 * PRECISION];
            self.rates_updated_at = env::epoch_height();
        } else {
            panic!("Parse failed");
        }
        true
    }
}

impl StnearRates {
    pub fn new(contract_id: AccountId, tokens_count: usize) -> Self {
        Self {
            stored_rates: vec![1 * PRECISION; tokens_count], // all rates equals 1.0
            rates_updated_at: 0,
            contract_id,
        }
    }
}

// TODO: tests
