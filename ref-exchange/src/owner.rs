//! Implement all the relevant logic for owner of this contract.

use crate::*;

#[near_bindgen]
impl Contract {
    /// Change owner. Only can be called by owner.
    pub fn set_owner(&mut self, owner_id: AccountId) {
        self.assert_owner();
        self.owner_id = owner_id;
    }

    /// Get the owner of this account.
    pub fn get_owner(&self) -> AccountId {
        self.owner_id.clone()
    }

    /// Extend whitelisted tokens with new tokens. Only can be called by owner.
    #[payable]
    pub fn extend_whitelisted_tokens(&mut self, tokens: Vec<AccountId>) {
        self.assert_owner();
        for token in tokens {
            self.whitelisted_tokens.insert(&token);
        }
    }

    /// Remove whitelisted token. Only can be called by owner.
    pub fn remove_whitelisted_tokens(&mut self, tokens: Vec<AccountId>) {
        self.assert_owner();
        for token in tokens {
            self.whitelisted_tokens.remove(&token);
        }
    }

    /// Migration function from v2 to v2.
    /// For next version upgrades, change this function.
    #[init(ignore_state)]
    pub fn migrate() -> Self {
        let contract: Contract = env::state_read().expect("ERR_NOT_INITIALIZED");
        contract
    }

    pub(crate) fn assert_owner(&self) {
        assert_eq!(
            env::predecessor_account_id(),
            self.owner_id,
            "ERR_NOT_ALLOWED"
        );
    }
}

#[cfg(target_arch = "wasm32")]
mod upgrade {
    use near_sdk::sys;
    use near_sdk::Gas;

    use super::*;

    /// Gas for calling migration call.
    pub const GAS_FOR_MIGRATE_CALL: Gas = Gas(5_000_000_000_000);

    /// Self upgrade and call migrate, optimizes gas by not loading into memory the code.
    /// Takes as input non serialized set of bytes of the code.
    #[no_mangle]
    pub extern "C" fn upgrade() {
        env::setup_panic_hook();
        let contract: Contract = env::state_read().expect("ERR_CONTRACT_IS_NOT_INITIALIZED");
        contract.assert_owner();
        let current_id = String::from(env::current_account_id()).into_bytes();
        let method_name = b"migrate".to_vec();
        unsafe {
            // Load input into register 0.
            sys::input(0);
            let promise_id =
                sys::promise_batch_create(current_id.len() as _, current_id.as_ptr() as _);
            sys::promise_batch_action_deploy_contract(promise_id, u64::MAX as _, 0);
            let attached_gas = env::prepaid_gas() - env::used_gas() - GAS_FOR_MIGRATE_CALL;
            sys::promise_batch_action_function_call(
                promise_id,
                method_name.len() as _,
                method_name.as_ptr() as _,
                0 as _,
                0 as _,
                0 as _,
                attached_gas.0,
            );
        }
    }
}
