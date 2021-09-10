//! This modules captures all the code needed to migrate from previous version.
use std::collections::HashMap;
use near_sdk::collections::UnorderedMap;

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{AccountId, Balance, StorageUsage};

use crate::account_deposit::{Account};
use crate::StorageKey;


/// Account deposits information and storage cost.
#[derive(BorshSerialize, BorshDeserialize, Default, Clone)]
pub struct AccountV1 {
    /// Native NEAR amount sent to the exchange.
    /// Used for storage right now, but in future can be used for trading as well.
    pub near_amount: Balance,
    /// Amounts of various tokens deposited to this account.
    pub tokens: HashMap<AccountId, Balance>,
    pub storage_used: StorageUsage,
}

// impl From<AccountV1> for VAccount {
//     fn from(account: AccountV1) -> Self {
//         VAccount::V1(account)
//     }
// }

// impl From<VAccount> for AccountV1 {
//     fn from(v_account: VAccount) -> Self {
//         match v_account {
//             VAccount::V1(account) => {account},
//             _ => unimplemented!(),
//         }
//     }
// }

impl AccountV1 {
    pub fn into_current(&self, account_id: &AccountId) -> Account {
        let mut acc = Account {
            near_amount: self.near_amount,
            tokens: UnorderedMap::new(StorageKey::AccountTokens {
                account_id: account_id.clone(),
            }),
            storage_used: self.storage_used,
        };
        for (token_id, amount) in self.tokens.iter() {
            acc.tokens.insert(token_id, amount);
        }
        acc
    }
}
