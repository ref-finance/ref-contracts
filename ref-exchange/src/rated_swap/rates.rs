use super::stnear_rates::StnearRates;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{AccountId, Balance, PromiseOrValue};

#[derive(BorshSerialize, BorshDeserialize)]
pub enum Rates {
    Stnear(StnearRates),
}

pub trait RatesTrait {
    fn are_actual(&self) -> bool;
    fn get(&self) -> &Vec<Balance>;
    fn update(&self) -> PromiseOrValue<bool>;
    fn update_callback(&mut self, cross_call_result: &Vec<u8>) -> bool;
}

impl RatesTrait for Rates {
    fn are_actual(&self) -> bool {
        match self {
            Rates::Stnear(rates) => rates.are_actual(),
        }
    }
    fn get(&self) -> &Vec<Balance> {
        match self {
            Rates::Stnear(rates) => rates.get(),
        }
    }
    fn update(&self) -> PromiseOrValue<bool> {
        match self {
            Rates::Stnear(rates) => rates.update(),
        }
    }
    fn update_callback(&mut self, cross_call_result: &Vec<u8>) -> bool {
        match self {
            Rates::Stnear(rates) => rates.update_callback(cross_call_result),
        }
    }
}

impl Rates {
    pub fn new(rates_type: String, contract_id: AccountId, tokens_count: usize) -> Self {
        match rates_type.as_str() {
            "STNEAR" => Rates::Stnear(StnearRates::new(contract_id, tokens_count)),
            _ => unimplemented!(),
        }
    }
}