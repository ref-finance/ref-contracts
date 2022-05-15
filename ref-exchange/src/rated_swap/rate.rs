use super::stnear_rate::StnearRate;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{env, AccountId, Balance, Promise};

use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Mutex;

pub static RATES: Lazy<Mutex<HashMap<AccountId, Rate>>> = Lazy::new(|| Mutex::new(HashMap::new()));
pub const RATE_STORAGE_KEY: &str = "rate_key";



#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub enum Rate {
    Stnear(StnearRate),
}

pub trait RateTrait {
    fn are_actual(&self) -> bool;
    fn get(&self) -> Balance;
    fn async_update(&self) -> Promise;
    fn set(&mut self, cross_call_result: &Vec<u8>) -> u128;
}

impl RateTrait for Rate {
    fn are_actual(&self) -> bool {
        match self {
            Rate::Stnear(rates) => rates.are_actual(),
        }
    }
    fn get(&self) -> Balance {
        match self {
            Rate::Stnear(rates) => rates.get(),
        }
    }
    fn async_update(&self) -> Promise {
        match self {
            Rate::Stnear(rates) => rates.async_update(),
        }
    }
    fn set(&mut self, cross_call_result: &Vec<u8>) -> u128 {
        match self {
            Rate::Stnear(rates) => rates.set(cross_call_result),
        }
    }
}

impl Rate {
    pub fn new(rates_type: String, contract_id: AccountId) -> Self {
        match rates_type.as_str() {
            "STNEAR" => Rate::Stnear(StnearRate::new(contract_id)),
            _ => unimplemented!(),
        }
    }
}

pub fn global_add_rate(rate_type: &String, token_id: &AccountId) {
    // read from storage
    let mut rates = if let Some(content) = env::storage_read(RATE_STORAGE_KEY.as_bytes()) {
        HashMap::try_from_slice(&content).expect("deserialize failed.")
    } else {
        HashMap::new()
    };

    // if rates.contains_key(token_id) {
    //     env::panic(format!("Already has token {}.", token_id.clone()).as_bytes());
    // }
    if !rates.contains_key(token_id) {
        rates.insert(token_id.clone(), Rate::new(rate_type.clone(), token_id.clone()));

        // save back to storage
        env::storage_write(
            RATE_STORAGE_KEY.as_bytes(), 
            &rates.try_to_vec().unwrap(),
        );
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
        // non rated token always has valid rate
        true
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
