use crate::*;
use near_sdk::StorageUsage;

#[derive(BorshDeserialize)]
pub struct FungibleToken {
    /// AccountID -> Account balance.
    pub accounts: LookupMap<AccountId, Balance>,

    /// Total supply of the all token.
    pub total_supply: Balance,

    /// The storage size in bytes for one account.
    pub account_storage_usage: StorageUsage,
}

#[derive(BorshDeserialize)]
pub struct TokenContract {
    pub ft: FungibleToken,
}

impl TokenContract {
    pub fn parse(&mut self, state: &mut State) {
        self.ft.accounts.parse(state);
    }
}
