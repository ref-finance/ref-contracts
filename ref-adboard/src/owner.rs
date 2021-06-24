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

    pub fn set_amm(&mut self, amm_id: ValidAccountId) {
        self.assert_owner();
        self.data_mut().amm_id = amm_id.into();
    }

    pub fn set_protected_period(&mut self, protected_period: u16) {
        self.assert_owner();
        self.data_mut().protected_period = protected_period;
    }

    pub fn set_trading_fee(&mut self, trading_fee: u16) {
        self.assert_owner();
        self.data_mut().trading_fee = trading_fee;
    }

    pub fn expand_frames(&mut self, expend_count: u16) {
        self.assert_owner();
        self.data_mut().frame_count += expend_count;
    }

    pub fn set_default_token(&mut self, token_id: ValidAccountId, sell_balance: U128) {
        self.assert_owner();
        self.data_mut().default_token_id = token_id.into();
        self.data_mut().default_sell_balance = sell_balance.into();
    }

    pub fn add_token_to_whitelist(&mut self, token_id: ValidAccountId) -> bool {
        self.assert_owner();
        self.data_mut().whitelist.insert(token_id.as_ref())
    }

    pub fn remove_token_from_whitelist(&mut self, token_id: ValidAccountId) -> bool {
        self.assert_owner();
        self.data_mut().whitelist.remove(token_id.as_ref())
    }

    pub fn repay_failure_payment(&mut self) {
        self.assert_owner();
        if let Some(item) = self.data_mut().failed_payments.pop() {
            self.handle_payment(&item.token_id, &item.receiver_id, item.amount);
        }
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

