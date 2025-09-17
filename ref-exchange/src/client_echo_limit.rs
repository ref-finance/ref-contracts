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