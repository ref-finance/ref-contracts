use crate::*;

pub fn read_ce_tw_from_storage() -> UnorderedSet<String> {
    if let Some(content) = env::storage_read(CLIENT_ECHO_TOKEN_ID_WHITELIST.as_bytes()) {
        UnorderedSet::try_from_slice(&content).expect("deserialize client echo token id whitelist failed.")
    } else {
        UnorderedSet::new(StorageKey::ClientEchoTokenIdWhitelistItem)
    }
}

pub fn write_ce_tw_to_storage(client_echo_token_id_whitelist: UnorderedSet<String>) {
    env::storage_write(
        CLIENT_ECHO_TOKEN_ID_WHITELIST.as_bytes(), 
        &client_echo_token_id_whitelist.try_to_vec().unwrap(),
    );
}

pub fn read_ce_sw_from_storage() -> UnorderedSet<String> {
    if let Some(content) = env::storage_read(CLIENT_ECHO_SENDER_ID_WHITELIST.as_bytes()) {
        UnorderedSet::try_from_slice(&content).expect("deserialize client echo sender id whitelist failed.")
    } else {
        UnorderedSet::new(StorageKey::ClientEchoSenderIdWhitelistItem)
    }
}

pub fn write_ce_sw_to_storage(client_echo_sender_id_whitelist: UnorderedSet<String>) {
    env::storage_write(
        CLIENT_ECHO_SENDER_ID_WHITELIST.as_bytes(),
        &client_echo_sender_id_whitelist.try_to_vec().unwrap(),
    );
}

pub fn read_ssw_from_storage() -> UnorderedSet<String> {
    if let Some(content) = env::storage_read(SECURE_SENDER_WHITELIST.as_bytes()) {
        UnorderedSet::try_from_slice(&content).expect("deserialize secure sender whitelist failed.")
    } else {
        UnorderedSet::new(StorageKey::SecureSenderWhitelistItem)
    }
}

pub fn write_ssw_to_storage(secure_sender_whitelist: UnorderedSet<String>) {
    env::storage_write(
        SECURE_SENDER_WHITELIST.as_bytes(),
        &secure_sender_whitelist.try_to_vec().unwrap(),
    );
}

fn matches_wildcard_pattern(whitelist: &UnorderedSet<String>, account_id: &AccountId) -> bool {
    // First check for exact match
    if whitelist.contains(account_id) {
        return true;
    }

    // Then check for prefix wildcard matches
    for pattern in whitelist.iter() {
        if pattern.starts_with('*') {
            let suffix = &pattern[1..];
            if account_id.ends_with(suffix) {
                return true;
            }
        }
    }

    false
}

pub fn assert_client_echo_valid(token_id: &AccountId, sender_id: &AccountId) {
    let secure_sender_whitelist = read_ssw_from_storage();

    // If sender is in secure sender whitelist (including wildcard matches), skip token validation
    if matches_wildcard_pattern(&secure_sender_whitelist, sender_id) {
        return;
    }

    // Otherwise, check both token and sender whitelists (including wildcard matches)
    let client_echo_token_id_whitelist = read_ce_tw_from_storage();
    let client_echo_sender_id_whitelist = read_ce_sw_from_storage();
    assert!(matches_wildcard_pattern(&client_echo_token_id_whitelist, token_id), "Invalid client echo token id");
    assert!(matches_wildcard_pattern(&client_echo_sender_id_whitelist, sender_id), "Invalid client echo sender id");
}

#[near_bindgen]
impl Contract {
    #[payable]
    pub fn extend_client_echo_token_id_whitelist(&mut self, token_ids: Vec<String>) {
        assert_one_yocto();
        assert!(self.is_owner_or_guardians(), "{}", ERR100_NOT_ALLOWED);
        let mut client_echo_token_id_whitelist = read_ce_tw_from_storage();
        for token_id in token_ids {
            if token_id.starts_with('*') {
                assert!(token_id.starts_with("*."), "Wildcard token id must start with '*.'");
            }
            let is_success = client_echo_token_id_whitelist.insert(&token_id);
            assert!(is_success, "Token id already exist");
        }
        write_ce_tw_to_storage(client_echo_token_id_whitelist);
    }

    #[payable]
    pub fn remove_client_echo_token_id_whitelist(&mut self, token_ids: Vec<String>) {
        assert_one_yocto();
        self.assert_owner();
        let mut client_echo_token_id_whitelist = read_ce_tw_from_storage();
        for token_id in token_ids {
            let is_success = client_echo_token_id_whitelist.remove(&token_id);
            assert!(is_success, "Invalid token id");
        }
        write_ce_tw_to_storage(client_echo_token_id_whitelist);
    }

    pub fn get_client_echo_token_id_whitelist(&self) -> Vec<String> {
        let client_echo_token_id_whitelist = read_ce_tw_from_storage();
        client_echo_token_id_whitelist.to_vec()
    }

    #[payable]
    pub fn extend_client_echo_sender_id_whitelist(&mut self, sender_ids: Vec<String>) {
        assert_one_yocto();
        assert!(self.is_owner_or_guardians(), "{}", ERR100_NOT_ALLOWED);
        let mut client_echo_sender_id_whitelist = read_ce_sw_from_storage();
        for sender_id in sender_ids {
            let is_success = client_echo_sender_id_whitelist.insert(&sender_id);
            assert!(is_success, "Sender id already exist");
        }
        write_ce_sw_to_storage(client_echo_sender_id_whitelist);
    }

    #[payable]
    pub fn remove_client_echo_sender_id_whitelist(&mut self, sender_ids: Vec<String>) {
        assert_one_yocto();
        self.assert_owner();
        let mut client_echo_sender_id_whitelist = read_ce_sw_from_storage();
        for sender_id in sender_ids {
            let is_success = client_echo_sender_id_whitelist.remove(&sender_id);
            assert!(is_success, "Invalid sender id");
        }
        write_ce_sw_to_storage(client_echo_sender_id_whitelist);
    }

    pub fn get_client_echo_sender_id_whitelist(&self) -> Vec<String> {
        let client_echo_sender_id_whitelist = read_ce_sw_from_storage();
        client_echo_sender_id_whitelist.to_vec()
    }

    #[payable]
    pub fn extend_secure_sender_whitelist(&mut self, sender_ids: Vec<String>) {
        assert_one_yocto();
        assert!(self.is_owner_or_guardians(), "{}", ERR100_NOT_ALLOWED);
        let mut secure_sender_whitelist = read_ssw_from_storage();
        for sender_id in sender_ids {
            if sender_id.starts_with('*') {
                assert!(sender_id.starts_with("*."), "Wildcard sender id must start with '*.'");
            }
            let is_success = secure_sender_whitelist.insert(&sender_id);
            assert!(is_success, "Secure sender id already exist");
        }
        write_ssw_to_storage(secure_sender_whitelist);
    }

    #[payable]
    pub fn remove_secure_sender_whitelist(&mut self, sender_ids: Vec<String>) {
        assert_one_yocto();
        self.assert_owner();
        let mut secure_sender_whitelist = read_ssw_from_storage();
        for sender_id in sender_ids {
            let is_success = secure_sender_whitelist.remove(&sender_id);
            assert!(is_success, "Invalid secure sender id");
        }
        write_ssw_to_storage(secure_sender_whitelist);
    }

    pub fn get_secure_sender_whitelist(&self) -> Vec<String> {
        let secure_sender_whitelist = read_ssw_from_storage();
        secure_sender_whitelist.to_vec()
    }
}

#[cfg(test)]
mod tests {
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::{testing_env, MockedBlockchain, AccountId};
    use near_sdk::collections::UnorderedSet;
    use crate::{Contract, StorageKey};
    use super::matches_wildcard_pattern;

    fn get_context(predecessor_account_id: near_sdk::json_types::ValidAccountId, is_view: bool) -> VMContextBuilder {
        let mut builder = VMContextBuilder::new();
        builder
            .current_account_id(accounts(0))
            .signer_account_id(accounts(0))
            .predecessor_account_id(predecessor_account_id)
            .attached_deposit(1)
            .is_view(is_view);
        builder
    }

    fn init_contract() -> Contract {
        Contract::new(
            accounts(0),
            accounts(1), // boost_farm_id
            accounts(2), // burrowland_id
            2000,        // exchange_fee
            0,           // referral_fee
        )
    }

    #[test]
    fn test_extend_client_echo_token_id_whitelist_success() {
        let context = get_context(accounts(0), false);
        testing_env!(context.build());

        let mut contract = init_contract();
        let token_ids = vec!["token1.near".to_string(), "token2.near".to_string()];

        contract.extend_client_echo_token_id_whitelist(token_ids.clone());

        let whitelist = contract.get_client_echo_token_id_whitelist();
        assert_eq!(whitelist.len(), 2);
        assert!(whitelist.contains(&"token1.near".to_string()));
        assert!(whitelist.contains(&"token2.near".to_string()));
    }

    #[test]
    fn test_extend_client_echo_token_id_whitelist_with_wildcard() {
        let context = get_context(accounts(0), false);
        testing_env!(context.build());

        let mut contract = init_contract();
        let token_ids = vec!["*.testnet".to_string(), "token1.near".to_string()];

        contract.extend_client_echo_token_id_whitelist(token_ids);

        let whitelist = contract.get_client_echo_token_id_whitelist();
        assert_eq!(whitelist.len(), 2);
        assert!(whitelist.contains(&"*.testnet".to_string()));
        assert!(whitelist.contains(&"token1.near".to_string()));
    }

    #[test]
    #[should_panic(expected = "Wildcard token id must start with '*.'")]
    fn test_extend_client_echo_token_id_whitelist_invalid_wildcard() {
        let context = get_context(accounts(0), false);
        testing_env!(context.build());

        let mut contract = init_contract();
        let token_ids = vec!["*testnet".to_string()];

        contract.extend_client_echo_token_id_whitelist(token_ids);
    }

    #[test]
    #[should_panic(expected = "Token id already exist")]
    fn test_extend_client_echo_token_id_whitelist_duplicate() {
        let context = get_context(accounts(0), false);
        testing_env!(context.build());

        let mut contract = init_contract();
        let token_ids = vec!["token1.near".to_string()];

        contract.extend_client_echo_token_id_whitelist(token_ids.clone());
        contract.extend_client_echo_token_id_whitelist(token_ids);
    }

    #[test]
    #[should_panic(expected = "E100: no permission to invoke this")]
    fn test_extend_client_echo_token_id_whitelist_not_authorized() {
        let context = get_context(accounts(1), false);
        testing_env!(context.build());

        let mut contract = init_contract();
        let token_ids = vec!["token1.near".to_string()];

        contract.extend_client_echo_token_id_whitelist(token_ids);
    }

    #[test]
    fn test_extend_client_echo_sender_id_whitelist_success() {
        let context = get_context(accounts(0), false);
        testing_env!(context.build());

        let mut contract = init_contract();
        let sender_ids = vec!["sender1.near".to_string(), "sender2.testnet".to_string()];

        contract.extend_client_echo_sender_id_whitelist(sender_ids.clone());

        let whitelist = contract.get_client_echo_sender_id_whitelist();
        assert_eq!(whitelist.len(), 2);
        assert!(whitelist.contains(&"sender1.near".to_string()));
        assert!(whitelist.contains(&"sender2.testnet".to_string()));
    }

    #[test]
    #[should_panic(expected = "Sender id already exist")]
    fn test_extend_client_echo_sender_id_whitelist_duplicate() {
        let context = get_context(accounts(0), false);
        testing_env!(context.build());

        let mut contract = init_contract();
        let sender_ids = vec!["sender1.near".to_string()];

        contract.extend_client_echo_sender_id_whitelist(sender_ids.clone());
        contract.extend_client_echo_sender_id_whitelist(sender_ids);
    }

    #[test]
    #[should_panic(expected = "E100: no permission to invoke this")]
    fn test_extend_client_echo_sender_id_whitelist_not_authorized() {
        let context = get_context(accounts(1), false);
        testing_env!(context.build());

        let mut contract = init_contract();
        let sender_ids = vec!["sender1.near".to_string()];

        contract.extend_client_echo_sender_id_whitelist(sender_ids);
    }

    #[test]
    fn test_extend_secure_sender_whitelist_success() {
        let context = get_context(accounts(0), false);
        testing_env!(context.build());

        let mut contract = init_contract();
        let sender_ids = vec!["secure1.near".to_string(), "secure2.testnet".to_string()];

        contract.extend_secure_sender_whitelist(sender_ids.clone());

        let whitelist = contract.get_secure_sender_whitelist();
        assert_eq!(whitelist.len(), 2);
        assert!(whitelist.contains(&"secure1.near".to_string()));
        assert!(whitelist.contains(&"secure2.testnet".to_string()));
    }

    #[test]
    fn test_extend_secure_sender_whitelist_with_wildcard() {
        let context = get_context(accounts(0), false);
        testing_env!(context.build());

        let mut contract = init_contract();
        let sender_ids = vec!["*.mainnet".to_string(), "secure1.near".to_string()];

        contract.extend_secure_sender_whitelist(sender_ids);

        let whitelist = contract.get_secure_sender_whitelist();
        assert_eq!(whitelist.len(), 2);
        assert!(whitelist.contains(&"*.mainnet".to_string()));
        assert!(whitelist.contains(&"secure1.near".to_string()));
    }

    #[test]
    #[should_panic(expected = "Wildcard sender id must start with '*.'")]
    fn test_extend_secure_sender_whitelist_invalid_wildcard() {
        let context = get_context(accounts(0), false);
        testing_env!(context.build());

        let mut contract = init_contract();
        let sender_ids = vec!["*mainnet".to_string()];

        contract.extend_secure_sender_whitelist(sender_ids);
    }

    #[test]
    #[should_panic(expected = "Secure sender id already exist")]
    fn test_extend_secure_sender_whitelist_duplicate() {
        let context = get_context(accounts(0), false);
        testing_env!(context.build());

        let mut contract = init_contract();
        let sender_ids = vec!["secure1.near".to_string()];

        contract.extend_secure_sender_whitelist(sender_ids.clone());
        contract.extend_secure_sender_whitelist(sender_ids);
    }

    #[test]
    #[should_panic(expected = "E100: no permission to invoke this")]
    fn test_extend_secure_sender_whitelist_not_authorized() {
        let context = get_context(accounts(1), false);
        testing_env!(context.build());

        let mut contract = init_contract();
        let sender_ids = vec!["secure1.near".to_string()];

        contract.extend_secure_sender_whitelist(sender_ids);
    }

    #[test]
    fn test_matches_wildcard_pattern_exact_match() {
        let context = get_context(accounts(0), false);
        testing_env!(context.build());

        let mut whitelist = UnorderedSet::new(StorageKey::ClientEchoTokenIdWhitelistItem);
        whitelist.insert(&"token1.near".to_string());
        whitelist.insert(&"token2.testnet".to_string());

        let account_id: AccountId = "token1.near".parse().unwrap();
        assert!(matches_wildcard_pattern(&whitelist, &account_id));

        let account_id2: AccountId = "token2.testnet".parse().unwrap();
        assert!(matches_wildcard_pattern(&whitelist, &account_id2));
    }

    #[test]
    fn test_matches_wildcard_pattern_wildcard_match() {
        let context = get_context(accounts(0), false);
        testing_env!(context.build());

        let mut whitelist = UnorderedSet::new(StorageKey::ClientEchoTokenIdWhitelistItem);
        whitelist.insert(&"*.near".to_string());
        whitelist.insert(&"*.testnet".to_string());

        let account_id1: AccountId = "token1.near".parse().unwrap();
        assert!(matches_wildcard_pattern(&whitelist, &account_id1));

        let account_id2: AccountId = "anythingelse.near".parse().unwrap();
        assert!(matches_wildcard_pattern(&whitelist, &account_id2));

        let account_id3: AccountId = "my-token.testnet".parse().unwrap();
        assert!(matches_wildcard_pattern(&whitelist, &account_id3));
    }

    #[test]
    fn test_matches_wildcard_pattern_no_match() {
        let context = get_context(accounts(0), false);
        testing_env!(context.build());

        let mut whitelist = UnorderedSet::new(StorageKey::ClientEchoTokenIdWhitelistItem);
        whitelist.insert(&"token1.near".to_string());
        whitelist.insert(&"*.testnet".to_string());

        let account_id1: AccountId = "token2.near".parse().unwrap();
        assert!(!matches_wildcard_pattern(&whitelist, &account_id1));

        let account_id2: AccountId = "something.mainnet".parse().unwrap();
        assert!(!matches_wildcard_pattern(&whitelist, &account_id2));
    }

    #[test]
    fn test_matches_wildcard_pattern_mixed_exact_and_wildcard() {
        let context = get_context(accounts(0), false);
        testing_env!(context.build());

        let mut whitelist = UnorderedSet::new(StorageKey::ClientEchoTokenIdWhitelistItem);
        whitelist.insert(&"specific.near".to_string());
        whitelist.insert(&"*.testnet".to_string());

        // Should match exact
        let account_id1: AccountId = "specific.near".parse().unwrap();
        assert!(matches_wildcard_pattern(&whitelist, &account_id1));

        // Should match wildcard
        let account_id2: AccountId = "any.testnet".parse().unwrap();
        assert!(matches_wildcard_pattern(&whitelist, &account_id2));

        // Should not match
        let account_id3: AccountId = "other.near".parse().unwrap();
        assert!(!matches_wildcard_pattern(&whitelist, &account_id3));
    }

    #[test]
    fn test_matches_wildcard_pattern_empty_whitelist() {
        let context = get_context(accounts(0), false);
        testing_env!(context.build());

        let whitelist = UnorderedSet::new(StorageKey::ClientEchoTokenIdWhitelistItem);

        let account_id: AccountId = "any.near".parse().unwrap();
        assert!(!matches_wildcard_pattern(&whitelist, &account_id));
    }

    #[test]
    fn test_matches_wildcard_pattern_complex_wildcards() {
        let context = get_context(accounts(0), false);
        testing_env!(context.build());

        let mut whitelist = UnorderedSet::new(StorageKey::ClientEchoTokenIdWhitelistItem);
        whitelist.insert(&"*.swap.near".to_string());
        whitelist.insert(&"*.farm.testnet".to_string());

        // Should match complex wildcard patterns
        let account_id1: AccountId = "ref.swap.near".parse().unwrap();
        assert!(matches_wildcard_pattern(&whitelist, &account_id1));

        let account_id2: AccountId = "my-protocol.farm.testnet".parse().unwrap();
        assert!(matches_wildcard_pattern(&whitelist, &account_id2));

        // Should not match partial patterns
        let account_id3: AccountId = "swap.near".parse().unwrap();
        assert!(!matches_wildcard_pattern(&whitelist, &account_id3));

        let account_id4: AccountId = "farm.testnet".parse().unwrap();
        assert!(!matches_wildcard_pattern(&whitelist, &account_id4));
    }

    #[test]
    fn test_matches_wildcard_pattern_wildcard_without_asterisk() {
        let context = get_context(accounts(0), false);
        testing_env!(context.build());

        let mut whitelist = UnorderedSet::new(StorageKey::ClientEchoTokenIdWhitelistItem);
        whitelist.insert(&"near".to_string()); // No asterisk, should be exact match only

        let account_id1: AccountId = "near".parse().unwrap();
        assert!(matches_wildcard_pattern(&whitelist, &account_id1));

        let account_id2: AccountId = "something.near".parse().unwrap();
        assert!(!matches_wildcard_pattern(&whitelist, &account_id2));
    }
}