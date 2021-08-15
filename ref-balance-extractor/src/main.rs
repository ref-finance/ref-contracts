mod lookup_map;
mod state;
mod unordered_set;
mod utils;
mod vector;

use crate::lookup_map::*;
use crate::state::*;
use crate::unordered_set::*;
use crate::utils::*;
use crate::vector::*;

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::{Base64VecU8, U128};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::serde_json;
use near_sdk::{AccountId, Balance};
use std::collections::HashMap;

lazy_static_include::lazy_static_include_bytes! {
    REF_FINANCE_STATE => "res/ref-finance.near.state.45195764.json",
    REF_FARMING_STATE => "res/ref-farming.near.state.45195764.json",
    DEPOSITS =>  "res/deposit.json",
    WITHDRAWALS =>  "res/withdraw.json",
}

#[derive(Clone, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct SwapVolume {
    pub input: U128,
    pub output: U128,
}

#[derive(BorshSerialize, BorshDeserialize)]
pub enum Pool {
    SimplePool(SimplePool),
}

impl Pool {
    pub fn parse(&mut self, state: &mut State) {
        match self {
            Pool::SimplePool(simple_pool) => {
                simple_pool.parse(state);
            }
        };
    }
}

/// Account deposits information and storage cost.
#[derive(BorshSerialize, BorshDeserialize, Default, Clone)]
pub struct Account {
    /// Native NEAR amount sent to the exchange.
    /// Used for storage right now, but in future can be used for trading as well.
    pub near_amount: Balance,
    /// Amounts of various tokens deposited to this account.
    pub tokens: HashMap<AccountId, Balance>,
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct SimplePool {
    /// List of tokens in the pool.
    pub token_account_ids: Vec<AccountId>,
    /// How much NEAR this contract has.
    pub amounts: Vec<Balance>,
    /// Volumes accumulated by this pool.
    pub volumes: Vec<SwapVolume>,
    /// Fee charged for swap (gets divided by FEE_DIVISOR).
    pub total_fee: u32,
    /// Portion of the fee going to exchange.
    pub exchange_fee: u32,
    /// Portion of the fee going to referral.
    pub referral_fee: u32,
    /// Shares of the pool by liquidity providers.
    pub shares: LookupMap<AccountId, Balance>,
    /// Total number of shares.
    pub shares_total_supply: Balance,
}

impl SimplePool {
    pub fn parse(&mut self, state: &mut State) {
        self.shares.parse(state);
    }
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct Contract {
    /// Account of the owner.
    pub owner_id: AccountId,
    /// Exchange fee, that goes to exchange itself (managed by governance).
    pub exchange_fee: u32,
    /// Referral fee, that goes to referrer in the call.
    pub referral_fee: u32,
    /// List of all the pools.
    pub pools: Vector<Pool>,
    /// Accounts registered, keeping track all the amounts deposited, storage and more.
    pub accounts: LookupMap<AccountId, Account>,
    /// Set of whitelisted tokens by "owner".
    pub whitelisted_tokens: UnorderedSet<AccountId>,
}

impl Contract {
    pub fn parse(&mut self, state: &mut State) {
        // parsing pools
        self.pools.parse(state);
        for pool in self.pools.data.iter_mut() {
            pool.parse(state);
        }
        self.accounts.parse(state);

        self.whitelisted_tokens.parse(state);
    }
}

fn main() {
    let mut ref_state = parse_json_state(&REF_FINANCE_STATE);
    let mut contract =
        Contract::try_from_slice(&ref_state.remove(&b"STATE".to_vec()).unwrap()).unwrap();
    contract.parse(&mut ref_state);
    println!("{}", contract.owner_id);

    println!("{}", ref_state.len());
}
