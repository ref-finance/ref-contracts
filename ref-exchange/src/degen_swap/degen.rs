use super::price_oracle::{PriceOracleConfig, PriceOracleDegen};
use super::pyth_oracle::{PythOracleConfig, PythOracleDegen};

use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{env, AccountId, Balance, Promise};

use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Mutex;

use crate::pyth_oracle::PriceIdentifier;

use crate::utils::{GAS_FOR_BASIC_OP, NO_DEPOSIT};
use crate::{ext_self, DEGEN_STORAGE_KEY};
use crate::DEGEN_ORACLE_CONFIG_STORAGE_KEY;

pub static DEGENS: Lazy<Mutex<HashMap<AccountId, Degen>>> = Lazy::new(|| Mutex::new(HashMap::new()));
pub static DEGEN_ORACLE_CONFIGS: Lazy<Mutex<HashMap<String, DegenOracleConfig>>> = Lazy::new(|| Mutex::new(HashMap::new()));

pub const PRICE_ORACLE_CONFIG_KEY: &str = "PriceOracleConfig";
pub const PYTH_ORACLE_CONFIG_KEY: &str = "PythOracleConfig";

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone)]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug))]
#[serde(crate = "near_sdk::serde")]
pub enum DegenOracleConfig {
    PriceOracle(PriceOracleConfig),
    PythOracle(PythOracleConfig),
}

impl DegenOracleConfig {
    pub fn get_key(&self) -> String {
        match self {
            DegenOracleConfig::PriceOracle(_) => PRICE_ORACLE_CONFIG_KEY.to_string(),
            DegenOracleConfig::PythOracle(_) => PYTH_ORACLE_CONFIG_KEY.to_string(),
        }
    }
}

#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub struct PriceInfo {
    pub stored_degen: Balance,
    pub degen_updated_at: u64,
}

#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub enum Degen {
    PriceOracle(PriceOracleDegen),
    PythOracle(PythOracleDegen),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(crate = "near_sdk::serde")]
pub enum DegenType {
    PriceOracle {
        decimals: u8
    },
    PythOracle {
        price_identifier: PriceIdentifier,
    },
}

pub trait DegenTrait {
    fn is_price_valid(&self) -> bool;
    fn get_price_info(&self) -> &PriceInfo;
    fn async_update(&self) -> Promise;
    fn set_price(&mut self, cross_call_result: &Vec<u8>) -> u128;
}

impl Degen {

    pub fn sync_token_price(&self, token_id: &AccountId) {
        self.async_update().then(ext_self::update_degen_token_price_callback(
            token_id.clone(),
            &env::current_account_id(),
            NO_DEPOSIT,
            GAS_FOR_BASIC_OP,
        ));
    }

    pub fn new(token_id: AccountId, degen_type: DegenType) -> Self {
        match degen_type {
            DegenType::PriceOracle { decimals } => {
                Degen::PriceOracle(PriceOracleDegen::new(token_id, decimals))
            }
            DegenType::PythOracle { price_identifier } => {
                Degen::PythOracle(PythOracleDegen::new(price_identifier))
            }
        }
    }

    pub fn get_type(&self) -> String {
        match self {
            Degen::PriceOracle(_) => "PriceOracle".to_string(),
            Degen::PythOracle(_) => "PythOracle".to_string(),
        }
    }
}

impl DegenTrait for Degen {
    fn is_price_valid(&self) -> bool {
        match self {
            Degen::PriceOracle(d) => d.is_price_valid(),
            Degen::PythOracle(d) => d.is_price_valid(),
        }
    }
    fn get_price_info(&self) -> &PriceInfo {
        match self {
            Degen::PriceOracle(d) => d.get_price_info(),
            Degen::PythOracle(d) => d.get_price_info(),
        }
    }
    fn async_update(&self) -> Promise {
        match self {
            Degen::PriceOracle(d) => d.async_update(),
            Degen::PythOracle(d) => d.async_update(),
        }
    }
    fn set_price(&mut self, cross_call_result: &Vec<u8>) -> u128 {
        match self {
            Degen::PriceOracle(d) => d.set_price(cross_call_result),
            Degen::PythOracle(d) => d.set_price(cross_call_result),
        }
    }
}

pub fn init_degens_cache() {
    if DEGENS.lock().unwrap().is_empty() {
        let degens = read_degens_from_storage();
        for (token_id, degen) in &degens {
            DEGENS.lock().unwrap().insert(token_id.clone(), degen.clone());
        }
    }
}

pub fn init_degen_oracle_configs_cache() {
    if DEGEN_ORACLE_CONFIGS.lock().unwrap().is_empty() {
        let degen_oracle_configs = read_degen_oracle_configs_from_storage();
        for (config_key, config) in &degen_oracle_configs {
            DEGEN_ORACLE_CONFIGS.lock().unwrap().insert(config_key.clone(), config.clone());
        }
    }
}

pub fn read_degens_from_storage() -> HashMap<String, Degen> {
    if let Some(content) = env::storage_read(DEGEN_STORAGE_KEY.as_bytes()) {
        HashMap::try_from_slice(&content).expect("deserialize failed.")
    } else {
        HashMap::new()
    }
}

pub fn write_degens_to_storage(degens: HashMap<String, Degen>) {
    env::storage_write(
        DEGEN_STORAGE_KEY.as_bytes(), 
        &degens.try_to_vec().unwrap(),
    );
}

pub fn read_degen_oracle_configs_from_storage() -> HashMap<String, DegenOracleConfig> {
    if let Some(content) = env::storage_read(DEGEN_ORACLE_CONFIG_STORAGE_KEY.as_bytes()) {
        HashMap::try_from_slice(&content).expect("deserialize failed.")
    } else {
        HashMap::new()
    }
}

pub fn write_degen_oracle_configs_to_storage(degen_oracle_configs: HashMap<String, DegenOracleConfig>) {
    env::storage_write(
        DEGEN_ORACLE_CONFIG_STORAGE_KEY.as_bytes(), 
        &degen_oracle_configs.try_to_vec().unwrap(),
    );
}

pub fn global_register_degen(token_id: &AccountId, degen_type: DegenType) -> bool {
    let mut degens = read_degens_from_storage();

    if !degens.contains_key(token_id) {
        degens.insert(token_id.clone(), Degen::new(token_id.clone(), degen_type));
        write_degens_to_storage(degens);
        true
    } else {
        false
    }
}

pub fn global_register_degen_oracle_config(config: DegenOracleConfig) -> bool {
    let mut degen_oracle_configs = read_degen_oracle_configs_from_storage();

    let config_key = config.get_key();
    if !degen_oracle_configs.contains_key(&config_key) {
        degen_oracle_configs.insert(config_key, config);
        write_degen_oracle_configs_to_storage(degen_oracle_configs);
        true
    } else {
        false
    }
}

pub fn global_unregister_degen(token_id: &AccountId) -> bool {
    let mut degens = read_degens_from_storage();

    if degens.contains_key(token_id) {
        degens.remove(token_id);
        write_degens_to_storage(degens);
        true
    } else {
        false
    }
}

pub fn global_unregister_degen_oracle_config(config_key: &String) -> bool {
    let mut degen_oracle_configs = read_degen_oracle_configs_from_storage();

    if degen_oracle_configs.contains_key(config_key) {
        degen_oracle_configs.remove(config_key);
        write_degen_oracle_configs_to_storage(degen_oracle_configs);
        true
    } else {
        false
    }
}

pub fn global_update_degen_oracle_config(config: DegenOracleConfig) -> bool {
    let mut degen_oracle_configs = read_degen_oracle_configs_from_storage();

    let config_key = config.get_key();
    if degen_oracle_configs.contains_key(&config_key) {
        degen_oracle_configs.insert(config_key, config);
        write_degen_oracle_configs_to_storage(degen_oracle_configs);
        true
    } else {
        false
    }

}

pub fn global_get_degen(token_id: &AccountId) -> Degen {
    init_degens_cache();
    DEGENS.lock().unwrap().get(token_id).expect(format!("{} is not degen token", token_id).as_str()).clone()
}

pub fn global_get_degen_price_oracle_config() -> PriceOracleConfig {
    init_degen_oracle_configs_cache();
    if let Some(DegenOracleConfig::PriceOracle(price_oracle_config)) = DEGEN_ORACLE_CONFIGS.lock().unwrap().get(&PRICE_ORACLE_CONFIG_KEY.to_string()) {
        price_oracle_config.clone()
    } else {
        env::panic("price oracle degen config is not init".as_bytes());
    }
}

pub fn global_get_degen_pyth_oracle_config() -> PythOracleConfig {
    init_degen_oracle_configs_cache();
    if let Some(DegenOracleConfig::PythOracle(pyth_oracle_config)) = DEGEN_ORACLE_CONFIGS.lock().unwrap().get(&PYTH_ORACLE_CONFIG_KEY.to_string()) {
        pyth_oracle_config.clone()
    } else {
        env::panic("pyth oracle degen config is not init".as_bytes());
    }
}

pub fn global_set_degen(token_id: &AccountId, degen: &Degen) {
    init_degens_cache();
    DEGENS.lock().unwrap().insert(token_id.clone(), degen.clone());
    env::storage_write(
        DEGEN_STORAGE_KEY.as_bytes(), 
        &DEGENS.lock().unwrap().try_to_vec().unwrap(),
    );
}

pub fn is_global_degen_price_valid(token_id: &AccountId) -> bool {
    init_degens_cache();
    DEGENS.lock().unwrap().get(token_id).expect(format!("{} is not degen token", token_id).as_str()).is_price_valid()
}