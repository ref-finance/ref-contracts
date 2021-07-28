use crate::*;

use near_sdk::{Promise};
use near_sdk::json_types::U128;
use crate::utils::{GAS_FOR_DEPLOY_CALL, GAS_FOR_UPGRADE_CALL};

#[near_bindgen]
impl Contract {
    pub fn set_owner(&mut self, owner_id: ValidAccountId) {

        self.assert_owner();

        self.data_mut().owner_id = owner_id.into();
    }

    pub fn clean_farm_by_seed(&mut self, seed_id: String) {
        self.assert_owner();
        if let Some(_) = self.get_seed_wrapped(&seed_id) {
            self.internal_remove_farm(&seed_id);
        }
    }

    pub fn modify_seed_min_deposit(&mut self, seed_id: String, min_deposit: U128) {
        self.assert_owner();
        let mut farm_seed = self.get_seed(&seed_id);
        farm_seed.get_ref_mut().min_deposit = min_deposit.into();
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

    pub(crate) fn assert_owner(&self) {
        assert_eq!(
            env::predecessor_account_id(),
            self.data().owner_id,
            "ERR_NOT_ALLOWED"
        );
    }
}

