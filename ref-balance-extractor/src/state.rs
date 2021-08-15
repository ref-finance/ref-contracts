use crate::*;
use std::collections::BTreeMap;


#[derive(Debug, Clone, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct JsonState {
    pub jsonrpc: String,
    pub result: JsonStateResult,
    pub id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct JsonStateResult {
    pub block_hash: String,
    pub block_height: u64,
    pub proof: Vec<u8>,
    pub values: Vec<StateValue>,
}


#[derive(Debug, Clone, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct StateValue {
    key: Base64VecU8,
    value: Base64VecU8,
    proof: Vec<u8>,
}

pub type State = BTreeMap<Vec<u8>, Vec<u8>>;

pub fn parse_json_state(state: &[u8]) -> State {
    let json_state: JsonState = serde_json::from_slice(state).unwrap();
    json_state.result.values.into_iter().map(|StateValue { key, value, .. }| {
        (key.0, value.0)
    }).collect()
}
