use near_sdk::collections::{UnorderedMap, LookupMap};
use near_sdk::json_types::{ValidAccountId, U128};
use near_sdk::{assert_one_yocto, env, near_bindgen, AccountId, Balance};
use crate::*;


pub type LostfoundAccount = UnorderedMap<AccountId, Balance>;

// // Key for lostfound
// pub const LOSTFOUND_KEY: &str = "lostfound";

/// LookupMap is a special structure that the only data stored on chain is the prefix,
/// so we don't need to actually read it from contract state, just use the predefined prefix.
/// similarly, we don't need to actually write it back to contract state either cause it won't change.
pub fn read_lostfound() -> LookupMap<AccountId, LostfoundAccount> {
    // if let Some(content) = env::storage_read(LOSTFOUND_KEY.as_bytes()) {
    //     LookupMap::try_from_slice(&content).expect("deserialize client echo token id whitelist failed.")
    // } else {
    //     LookupMap::new(StorageKey::LostfoundAccounts)
    // }
    LookupMap::new(StorageKey::LostfoundAccounts)
}


#[near_bindgen]
impl Contract {
    pub fn get_lostfound_token(&self, account_id: ValidAccountId, token_id: ValidAccountId) -> U128 {
        if let Some(account) = self.get_lostfound_account(account_id.as_ref()) {
            account.get(token_id.as_ref()).unwrap_or(0_u128).into()
        } else {
            0_u128.into()
        }
    }

    pub fn list_lostfound_tokens(&self, account_id: ValidAccountId, from_index: Option<u64>, limit: Option<u64>) -> HashMap<AccountId, U128> {
        if let Some(account) = self.get_lostfound_account(account_id.as_ref()) {
            let keys = account.keys_as_vector();
            let from_index = from_index.unwrap_or(0);
            let limit = limit.unwrap_or(keys.len() as u64);
            (from_index..std::cmp::min(keys.len() as u64, from_index + limit))
                .map(|idx| {
                    let key = keys.get(idx).unwrap();
                    (key.clone(), account.get(&key).unwrap().into())
                })
                .collect()
        } else {
            Default::default()
        }
    }

    #[payable]
    pub fn claim_lostfound(&mut self, token_id: ValidAccountId) -> Promise {
        assert_one_yocto();
        self.assert_contract_running();
        let token_id: AccountId = token_id.into();
        self.assert_no_frozen_tokens(&[token_id.clone()]);
        let sender_id = env::predecessor_account_id();

        let amount = self.remove_lostfound_token(&sender_id, &token_id);
        assert!(amount > 0, "{}", ERR29_ILLEGAL_WITHDRAW_AMOUNT);
        self.internal_send_tokens(&sender_id, &token_id, amount, None)
    }
}

impl Contract {
    pub fn get_lostfound_account(&self, account_id: &AccountId) -> Option<LostfoundAccount> {
        let lostfound = read_lostfound();
        lostfound.get(account_id)
    }

    pub fn exist_lostfound_token(&self, account_id: &AccountId, token_id: &AccountId) -> bool {
        let lostfound = read_lostfound();
        if let Some(lostfound_account) = lostfound.get(account_id) {
            if let Some(_) = lostfound_account.get(token_id) {
                return true;
            }
        }
        false
    }

    pub fn insert_lostfound_token(&mut self, account_id: &AccountId, token_id: &AccountId, amount: Balance) -> Balance {
        let mut lostfound = read_lostfound();
        let mut lostfound_account = lostfound
            .get(account_id)
            .unwrap_or_else(|| UnorderedMap::new(StorageKey::LostfoundAccountTokens {
                account_id: account_id.clone(),
            }));
        let old_value = lostfound_account.get(token_id).unwrap_or(0_u128);
        lostfound_account.insert(token_id, &(old_value+amount));
        lostfound.insert(account_id, &lostfound_account);
        old_value
    }

    pub fn remove_lostfound_token(&mut self, account_id: &AccountId, token_id: &AccountId) -> Balance {
        let mut lostfound = read_lostfound();
        let mut lostfound_account = lostfound
            .get(account_id)
            .unwrap_or_else(|| UnorderedMap::new(StorageKey::LostfoundAccountTokens {
                account_id: account_id.clone(),
            }));
        let value = lostfound_account.remove(token_id).unwrap_or(0_u128);
        if lostfound_account.len() > 0 {
            lostfound.insert(account_id, &lostfound_account);
        } else {
            lostfound.remove(account_id);
        }
        value
    }
}