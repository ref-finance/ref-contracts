use crate::errors::{ERR41_WRONG_ACTION_RESULT, ERR77_INVALID_ACTION_TYPE};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{env, json_types::U128, AccountId, Balance};
use std::collections::HashSet;

/// Single swap action.
#[derive(Serialize, Deserialize, Clone)]
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

/// Single swap by output action.
#[derive(Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct SwapByOutputAction {
    /// Pool which should be used for swapping.
    pub pool_id: u64,
    /// Token to swap from.
    pub token_in: AccountId,
    /// The desired amount of the output token.
    /// If amount_out is None, it will take amount_in from previous step.
    /// Will fail if amount_out is None on the first step.
    pub amount_out: Option<U128>,
    /// Token to swap into.
    pub token_out: AccountId,
    /// The maximum amount of the input token that can be used for the swap.
    pub max_amount_in: Option<U128>,
}

/// Single action. Allows to execute sequence of various actions initiated by an account.
#[derive(Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
#[serde(untagged)]
pub enum Action {
    Swap(SwapAction),
    SwapByOutput(SwapByOutputAction),
}

impl Action {
    /// Returns involved tokens in this action. Useful for checking permissions and storage.
    pub fn tokens(&self) -> Vec<AccountId> {
        match self {
            Action::Swap(swap_action) => {
                vec![swap_action.token_in.clone(), swap_action.token_out.clone()]
            }
            Action::SwapByOutput(swap_by_output_action) => {
                vec![swap_by_output_action.token_in.clone(), swap_by_output_action.token_out.clone()]
            }
        }
    }

    pub fn get_pool_id(&self) -> u64 {
        match self {
            Action::Swap(swap_action) => {
                swap_action.pool_id
            }
            Action::SwapByOutput(swap_by_output_action) => {
                swap_by_output_action.pool_id
            }
        }
    }

    pub fn get_token_in(&self) -> &AccountId {
        match self {
            Action::Swap(swap_action) => {
                &swap_action.token_in
            }
            Action::SwapByOutput(swap_by_output_action) => {
                &swap_by_output_action.token_in
            }
        }
    }

    pub fn get_token_out(&self) -> &AccountId {
        match self {
            Action::Swap(swap_action) => {
                &swap_action.token_out
            }
            Action::SwapByOutput(swap_by_output_action) => {
                &swap_by_output_action.token_out
            }
        }
    }

    pub fn get_amount_out(&self) -> Option<U128> {
        match self {
            Action::Swap(_) => unimplemented!(),
            Action::SwapByOutput(swap_by_output_action) => {
                swap_by_output_action.amount_out
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
    pub fn to_amount(&self) -> Balance {
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
            Action::SwapByOutput(swap_by_output_action) => {
                tokens.insert(swap_by_output_action.token_in.clone());
                tokens.insert(swap_by_output_action.token_out.clone());
            }
        }
    }
    tokens
}

pub fn assert_all_same_action_type(actions: &[Action]) {
    if !actions.is_empty() {
        let all_same_action_type = match &actions[0] {
            Action::Swap(_) => actions.iter().all(|action| matches!(action, Action::Swap(_))),
            Action::SwapByOutput(_) => actions.iter().all(|action| matches!(action, Action::SwapByOutput(_))),
        };
        assert!(all_same_action_type, "{}", ERR77_INVALID_ACTION_TYPE);
    }
}