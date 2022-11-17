use crate::errors::ERR41_WRONG_ACTION_RESULT;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{env, json_types::U128, AccountId, Balance};
use std::collections::HashSet;

/// Single swap action.
#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct SwapAction {
    /// Pool which should be used for swapping.
    pub pool_id: u64,
    /// Token to swap from.
    pub token_in: AccountId,
    /// Amount to exchange.
    /// If amount_in is None, it will take amount_out from previous step.
    /// Will fail if amount_in is None on the first step.
    pub amount_in: Option<U128>,
    /// Token to swap into.
    pub token_out: AccountId,
    /// Required minimum amount of token_out.
    pub min_amount_out: U128,
}

/// Single action. Allows to execute sequence of various actions initiated by an account.
#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[serde(untagged)]
pub enum Action {
    Swap(SwapAction),
}

impl Action {
    /// Returns involved tokens in this action. Useful for checking permissions and storage.
    pub fn tokens(&self) -> Vec<AccountId> {
        match self {
            Action::Swap(swap_action) => {
                vec![swap_action.token_in.clone(), swap_action.token_out.clone()]
            }
        }
    }
}

/// Result from action execution.
#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub enum ActionResult {
    /// No result.
    None,
    /// Amount of token was received.
    /// [AUDIT_02]
    Amount(U128),
}

impl ActionResult {
    pub fn to_amount(self) -> Balance {
        match self {
            // [AUDIT_02]
            ActionResult::Amount(result) => result.0,
            _ => env::panic(ERR41_WRONG_ACTION_RESULT.as_bytes()),
        }
    }
}

/// return involved tokens in an action array
pub fn get_tokens_in_actions(actions: &[Action]) -> HashSet<AccountId> {
    let mut tokens: HashSet<AccountId> = HashSet::new();
    for action in actions {
        match action {
            Action::Swap(swap_action) => {
                tokens.insert(swap_action.token_in.clone());
                tokens.insert(swap_action.token_out.clone());
            }
        }
    }
    tokens
}
