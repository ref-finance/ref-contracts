use super::stnear_rate::StnearRate;
use super::linear_rate::LinearRate;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{env, AccountId, Balance, Promise};
use crate::ERR127_INVALID_RATE_TYPE;

use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Mutex;

use crate::RATE_STORAGE_KEY;

pub static RATES: Lazy<Mutex<HashMap<AccountId, Rate>>> = Lazy::new(|| Mutex::new(HashMap::new()));



#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub enum Rate {
    Stnear(StnearRate),
    Linear(LinearRate),
}

pub trait RateTrait {
    fn are_actual(&self) -> bool;
    fn get(&self) -> Balance;
    fn last_update_ts(&self) -> u64;
    fn async_update(&self) -> Promise;
    fn set(&mut self, cross_call_result: &Vec<u8>) -> u128;
}

impl RateTrait for Rate {
    fn are_actual(&self) -> bool {
        match self {
            Rate::Stnear(rates) => rates.are_actual(),
            Rate::Linear(rates) => rates.are_actual(),
        }
    }
    fn get(&self) -> Balance {
        match self {
            Rate::Stnear(rates) => rates.get(),
            Rate::Linear(rates) => rates.get(),
        }
    }
    fn last_update_ts(&self) -> u64 {
        match self {
            Rate::Stnear(rates) => rates.last_update_ts(),
            Rate::Linear(rates) => rates.last_update_ts(),
        }
    }
    fn async_update(&self) -> Promise {
        match self {
            Rate::Stnear(rates) => rates.async_update(),
            Rate::Linear(rates) => rates.async_update(),
        }
    }
    fn set(&mut self, cross_call_result: &Vec<u8>) -> u128 {
        match self {
            Rate::Stnear(rates) => rates.set(cross_call_result),
            Rate::Linear(rates) => rates.set(cross_call_result),
        }
    }
}

impl Rate {
    pub fn new(rates_type: String, contract_id: AccountId) -> Self {
        match rates_type.as_str() {
            "STNEAR" => Rate::Stnear(StnearRate::new(contract_id)),
            "LINEAR" => Rate::Linear(LinearRate::new(contract_id)),
            _ => unimplemented!(),
        }
    }

    pub fn get_type(&self) -> String {
        match self {
            Rate::Stnear(_) => "STNEAR".to_string(),
            Rate::Linear(_) => "LINEAR".to_string(),
        }
    }

    pub fn is_valid_rate_type(rates_type: &str) -> bool {
        match rates_type {
            "STNEAR" => true,
            "LINEAR" => true,
            _ => false,
        }
    }
}

/// Register a rate token with given type.
/// if token already exist, return false; otherwise, return true
/// if rate_type is invalid, would panic
pub fn global_register_rate(rate_type: &String, token_id: &AccountId) -> bool {
    assert!(Rate::is_valid_rate_type(rate_type.as_str()), "{}", ERR127_INVALID_RATE_TYPE);
    // read from storage
    let mut rates: HashMap<String, Rate> = if let Some(content) = env::storage_read(RATE_STORAGE_KEY.as_bytes()) {
        HashMap::try_from_slice(&content).expect("deserialize failed.")
    } else {
        HashMap::new()
    };

    if !rates.contains_key(token_id) {
        rates.insert(token_id.clone(), Rate::new(rate_type.clone(), token_id.clone()));
        // save back to storage
        env::storage_write(
            RATE_STORAGE_KEY.as_bytes(), 
            &rates.try_to_vec().unwrap(),
        );
        true
    } else {
        false
    }
}

/// Unregister a rate token.
/// if token already removed, return false; otherwise, return true
pub fn global_unregister_rate(token_id: &AccountId) -> bool {
    // read from storage
    let mut rates: HashMap<String, Rate> = if let Some(content) = env::storage_read(RATE_STORAGE_KEY.as_bytes()) {
        HashMap::try_from_slice(&content).expect("deserialize failed.")
    } else {
        HashMap::new()
    };

    if rates.contains_key(token_id) {
        rates.remove(token_id);
        // save back to storage
        env::storage_write(
            RATE_STORAGE_KEY.as_bytes(), 
            &rates.try_to_vec().unwrap(),
        );
        true
    } else {
        false
    }
}

pub fn global_get_rate(token_id: &AccountId) -> Option<Rate> {
    if RATES.lock().unwrap().is_empty() {
        let rates: HashMap<AccountId, Rate> =
            if let Some(content) = env::storage_read(RATE_STORAGE_KEY.as_bytes()) {
                HashMap::try_from_slice(&content).expect("deserialize failed.")
            } else {
                HashMap::new()
            };
        for (token_id, rate) in &rates {
            RATES.lock().unwrap().insert(token_id.clone(), rate.clone());
        }
    }
    if let Some(rate) = RATES.lock().unwrap().get(token_id) {
        Some(rate.clone())
    } else {
        None
    }
}

pub fn global_set_rate(token_id: &AccountId, rate: &Rate) {
    if RATES.lock().unwrap().is_empty() {
        let rates: HashMap<AccountId, Rate> =
            if let Some(content) = env::storage_read(RATE_STORAGE_KEY.as_bytes()) {
                HashMap::try_from_slice(&content).expect("deserialize failed.")
            } else {
                HashMap::new()
            };
        for (token_id, rate) in &rates {
            RATES.lock().unwrap().insert(token_id.clone(), rate.clone());
        }
    }

    RATES.lock().unwrap().insert(token_id.clone(), rate.clone());

    // save back to storage
    env::storage_write(
        RATE_STORAGE_KEY.as_bytes(), 
        &RATES.lock().unwrap().try_to_vec().unwrap(),
    );
}

pub fn is_global_rate_valid(token_id: &AccountId) -> bool {
    if RATES.lock().unwrap().is_empty() {
        let rates: HashMap<AccountId, Rate> =
            if let Some(content) = env::storage_read(RATE_STORAGE_KEY.as_bytes()) {
                HashMap::try_from_slice(&content).expect("deserialize failed.")
            } else {
                HashMap::new()
            };
        for (token_id, rate) in &rates {
            RATES.lock().unwrap().insert(token_id.clone(), rate.clone());
        }
    }
    if let Some(rate) = RATES.lock().unwrap().get(token_id) {
        rate.are_actual()
    } else {
        // non-rated token always has valid rate
        true
    }
}
