//! This modules captures all the code needed to migrate from previous version.

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LookupMap, Vector};
use near_sdk::AccountId;

use crate::account_deposit::AccountDeposit;
use crate::pool::Pool;

/// Version before whitelisted tokens collection.
#[derive(BorshSerialize, BorshDeserialize)]
pub struct ContractV1 {
    /// Account of the owner.
    pub owner_id: AccountId,
    /// Exchange fee, that goes to exchange itself (managed by governance).
    pub exchange_fee: u32,
    /// Referral fee, that goes to referrer in the call.
    pub referral_fee: u32,
    /// List of all the pools.
    pub pools: Vector<Pool>,
    /// Balances of deposited tokens for each account.
    pub deposited_amounts: LookupMap<AccountId, AccountDeposit>,
}
