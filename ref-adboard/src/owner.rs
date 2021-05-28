use crate::*;

use near_sdk::{Promise, Gas};

/// Amount of gas used for upgrade function itself.
pub const GAS_FOR_UPGRADE_CALL: Gas = 50_000_000_000_000;
/// Amount of gas for deploy action.
pub const GAS_FOR_DEPLOY_CALL: Gas = 20_000_000_000_000;

#[near_bindgen]
impl Contract {
    pub fn set_owner(&mut self, owner_id: ValidAccountId) {

        self.assert_owner();

        self.data_mut().owner_id = owner_id.into();
    }

    pub fn add_token_to_whitelist(&mut self, token_id: ValidAccountId) -> bool {
        self.assert_owner();
        self.data_mut().whitelist.insert(token_id.as_ref())
    }

    pub fn remove_token_from_whitelist(&mut self, token_id: ValidAccountId) -> bool {
        self.assert_owner();
        self.data_mut().whitelist.remove(token_id.as_ref())
    }

    /// Upgrades given contract. Only can be called by owner.
    /// if `migrate` is true, calls `migrate()` function right after deployment.
    /// TODO: consider adding extra grace period in case `owner` got attacked.
    pub fn upgrade(
        &self,
        #[serializer(borsh)] code: Vec<u8>,
        #[serializer(borsh)] migrate: bool,
    ) -> Promise {
        self.assert_owner();
        let mut promise = Promise::new(env::current_account_id()).deploy_contract(code);
        if migrate {
            promise = promise.function_call(
                "migrate".into(),
                vec![],
                0,
                env::prepaid_gas() - GAS_FOR_UPGRADE_CALL - GAS_FOR_DEPLOY_CALL,
            );
        }
        promise
    }

    /// Migration function between versions.
    /// For next version upgrades, change this function.
    #[init(ignore_state)]
    pub fn migrate() -> Self {
        assert_eq!(
            env::predecessor_account_id(),
            env::current_account_id(),
            "ERR_NOT_ALLOWED"
        );
        let contract: Contract = env::state_read().expect("ERR_NOT_INITIALIZED");
        contract
    }

    
}

