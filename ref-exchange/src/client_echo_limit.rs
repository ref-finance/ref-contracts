use crate::*;

pub fn read_ce_tw_from_storage() -> UnorderedSet<AccountId> {
    if let Some(content) = env::storage_read(CLIENT_ECHO_TOKEN_ID_WHITHELIST.as_bytes()) {
        UnorderedSet::try_from_slice(&content).expect("deserialize client echo token id whitelist failed.")
    } else {
        UnorderedSet::new(StorageKey::ClientEchoTokenIdWhitelistItem)
    }
}

pub fn write_ce_tw_to_storage(client_echo_token_id_whitelist: UnorderedSet<AccountId>) {
    env::storage_write(
        CLIENT_ECHO_TOKEN_ID_WHITHELIST.as_bytes(), 
        &client_echo_token_id_whitelist.try_to_vec().unwrap(),
    );
}

pub fn read_ce_sw_from_storage() -> UnorderedSet<AccountId> {
    if let Some(content) = env::storage_read(CLIENT_ECHO_SENDER_ID_WHITHELIST.as_bytes()) {
        UnorderedSet::try_from_slice(&content).expect("deserialize client echo sender id whitelist failed.")
    } else {
        UnorderedSet::new(StorageKey::ClientEchoSenderIdWhitelistItem)
    }
}

pub fn write_ce_sw_to_storage(client_echo_sender_id_whitelist: UnorderedSet<AccountId>) {
    env::storage_write(
        CLIENT_ECHO_SENDER_ID_WHITHELIST.as_bytes(), 
        &client_echo_sender_id_whitelist.try_to_vec().unwrap(),
    );
}

pub fn assert_client_echo_valid(token_id: &AccountId, sender_id: &AccountId) {
    let client_echo_token_id_whitelist = read_ce_tw_from_storage();
    let client_echo_sender_id_whitelist = read_ce_sw_from_storage();
    assert!(client_echo_token_id_whitelist.contains(token_id), "Invalid client echo token id");
    assert!(client_echo_sender_id_whitelist.contains(sender_id), "Invalid client echo sender id");
}

#[near_bindgen]
impl Contract {
    #[payable]
    pub fn extend_client_echo_token_id_whitelist(&mut self, token_ids: Vec<ValidAccountId>) {
        assert_one_yocto();
        assert!(self.is_owner_or_guardians(), "{}", ERR100_NOT_ALLOWED);
        let mut client_echo_token_id_whitelist = read_ce_tw_from_storage();
        for token_id in token_ids {
            let is_success = client_echo_token_id_whitelist.insert(token_id.as_ref());
            assert!(is_success, "Token id already exist");
        }
        write_ce_tw_to_storage(client_echo_token_id_whitelist);
    }

    #[payable]
    pub fn remove_client_echo_token_id_whitelist(&mut self, token_ids: Vec<ValidAccountId>) {
        assert_one_yocto();
        self.assert_owner();
        let mut client_echo_token_id_whitelist = read_ce_tw_from_storage();
        for token_id in token_ids {
            let is_success = client_echo_token_id_whitelist.remove(token_id.as_ref());
            assert!(is_success, "Invalid token id");
        }
        write_ce_tw_to_storage(client_echo_token_id_whitelist);
    }

    pub fn get_client_echo_token_id_whitelist(&self) -> Vec<AccountId> {
        let client_echo_token_id_whitelist = read_ce_tw_from_storage();
        client_echo_token_id_whitelist.to_vec()
    }

    #[payable]
    pub fn extend_client_echo_sender_id_whitelist(&mut self, sender_ids: Vec<ValidAccountId>) {
        assert_one_yocto();
        assert!(self.is_owner_or_guardians(), "{}", ERR100_NOT_ALLOWED);
        let mut client_echo_sender_id_whitelist = read_ce_sw_from_storage();
        for sender_id in sender_ids {
            let is_success = client_echo_sender_id_whitelist.insert(sender_id.as_ref());
            assert!(is_success, "Sender id already exist");
        }
        write_ce_sw_to_storage(client_echo_sender_id_whitelist);
    }

    #[payable]
    pub fn remove_client_echo_sender_id_whitelist(&mut self, sender_ids: Vec<ValidAccountId>) {
        assert_one_yocto();
        self.assert_owner();
        let mut client_echo_sender_id_whitelist = read_ce_sw_from_storage();
        for sender_id in sender_ids {
            let is_success = client_echo_sender_id_whitelist.remove(sender_id.as_ref());
            assert!(is_success, "Invalid sender id");
        }
        write_ce_sw_to_storage(client_echo_sender_id_whitelist);
    }

    pub fn get_client_echo_sender_id_whitelist(&self) -> Vec<AccountId> {
        let client_echo_sender_id_whitelist = read_ce_sw_from_storage();
        client_echo_sender_id_whitelist.to_vec()
    }
}