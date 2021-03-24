//! Implement all the relevant logic for owner of this contract.

use crate::*;

use crate::legacy::ContractV1;
use crate::utils::{GAS_FOR_DEPLOY_CALL, GAS_FOR_UPGRADE_CALL};

#[near_bindgen]
impl Contract {
    /// Change owner. Only can be called by owner.
    pub fn set_owner(&mut self, owner_id: ValidAccountId) {
        self.assert_owner();
        self.owner_id = owner_id.as_ref().clone();
    }

    /// Get the owner of this account.
    pub fn get_owner(&self) -> AccountId {
        self.owner_id.clone()
    }

    /// Extend whitelisted tokens with new tokens. Only can be called by owner.
    #[payable]
    pub fn extend_whitelisted_tokens(&mut self, tokens: Vec<ValidAccountId>) {
        self.assert_owner();
        for token in tokens {
            self.whitelisted_tokens.insert(token.as_ref());
        }
    }

    /// Remove whitelisted token. Only can be called by owner.
    pub fn remove_whitelisted_token(&mut self, token: ValidAccountId) {
        self.assert_owner();
        self.whitelisted_tokens.remove(token.as_ref());
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

    /// Migration function from v1 to v2.
    /// For next version upgrades, change this function.
    #[init(ignore_state)]
    pub fn migrate() -> Self {
        let contract_v1: ContractV1 = env::state_read().expect("ERR_NOT_INITIALIZED");
        Self {
            owner_id: contract_v1.owner_id,
            exchange_fee: contract_v1.exchange_fee,
            referral_fee: contract_v1.referral_fee,
            pools: contract_v1.pools,
            deposited_amounts: contract_v1.deposited_amounts,
            whitelisted_tokens: UnorderedSet::new(b"w".to_vec()),
        }
    }

    pub(crate) fn assert_owner(&self) {
        assert_eq!(
            env::predecessor_account_id(),
            self.owner_id,
            "ERR_NOT_ALLOWED"
        );
    }
}
