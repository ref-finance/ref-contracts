//! Account rewards is information per user about their reward tokens balances.
//! 


use std::collections::HashMap;

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{env, AccountId, Balance};

use crate::{SeedId, FarmId};

use crate::errors::*;


/// assume the average AccountId length is lower than 64 bytes.
const MAX_ACCOUNT_LENGTH: u128 = 64;
/// each entry cost MAX_ACCOUNT_LENGTH bytes, 
/// amount: Balance cost 16 bytes
/// each empty hashmap cost 4 bytes
const MIN_FARMER_LENGTH: u128 = MAX_ACCOUNT_LENGTH + 16 + 4 * 3;

/// Account deposits information and storage cost.
#[derive(BorshSerialize, BorshDeserialize, Default)]
#[cfg_attr(feature = "test", derive(Clone))]
pub struct Farmer {
    /// Native NEAR amount sent to this contract.
    /// Used for storage.
    pub amount: Balance,
    /// Amounts of various reward tokens the farmer claimed.
    pub rewards: HashMap<AccountId, Balance>,
    /// Amounts of various seed tokens the farmer staked.
    pub seeds: HashMap<SeedId, Balance>,
    /// record user_last_rps of farms
    pub farm_rps: HashMap<FarmId, Balance>,
}

impl Farmer {

    /// Adds amount to the balance of given token
    pub(crate) fn add_reward(&mut self, token: &AccountId, amount: Balance) {
        if let Some(x) = self.rewards.get_mut(token) {
            *x = *x + amount;
        } else {
            self.rewards.insert(token.clone(), amount);
        }
    }

    /// Subtract from `reward` balance.
    /// if amount == 0, subtract all reward balance.
    /// Panics if `amount` is bigger than the current balance.
    /// return actural subtract amount
    pub(crate) fn sub_reward(&mut self, token: &AccountId, amount: Balance) -> Balance {
        let value = *self.rewards.get(token).expect(ERR21_TOKEN_NOT_REG);
        assert!(value >= amount, "{}", ERR22_NOT_ENOUGH_TOKENS);
        if amount == 0 {
            self.rewards.remove(&token.clone());
            value
        } else {
            self.rewards.insert(token.clone(), value - amount);
            amount
        }
    }

    pub fn add_seed(&mut self, seed_id: &SeedId, amount: Balance) {
        if amount > 0 {
            self.seeds.insert(
                seed_id.clone(), 
                amount + self.seeds.get(seed_id).unwrap_or(&0_u128)
            );
        }
        
    }

    /// return seed remained.
    pub fn sub_seed(&mut self, seed_id: &SeedId, amount: Balance) -> Balance {
        let prev_balance = self.seeds.get(seed_id).expect(&format!("{}", ERR31_SEED_NOT_EXIST));
        assert!(prev_balance >= &amount, "{}", ERR32_NOT_ENOUGH_SEED);
        let cur_balance = prev_balance - amount;
        if cur_balance > 0 {
            self.seeds.insert(seed_id.clone(), cur_balance);
        } else {
            self.seeds.remove(seed_id);
        }
        cur_balance
    }

    pub fn get_rps(&self, farm_id: &FarmId) -> Balance {
        self.farm_rps.get(farm_id).unwrap_or(&0).clone()
    }

    pub fn set_rps(&mut self, farm_id: &FarmId, rps: Balance) {
        self.farm_rps.insert(farm_id.clone(), rps);
    }

    /// Returns amount of $NEAR necessary to cover storage used by this data structure.
    pub fn storage_usage(&self) -> Balance {
        (
            MIN_FARMER_LENGTH 
            + self.rewards.len() as u128 * (MAX_ACCOUNT_LENGTH + 16)
            + self.seeds.len() as u128 * (MAX_ACCOUNT_LENGTH + 16)
            + self.farm_rps.len() as u128 * (MAX_ACCOUNT_LENGTH + 16)
        )
        * env::storage_byte_cost()
    }

}
