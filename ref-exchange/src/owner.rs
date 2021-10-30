//! Implement all the relevant logic for owner of this contract.

use near_sdk::json_types::WrappedTimestamp;

use crate::*;
use crate::utils::FEE_DIVISOR;
use crate::legacy::ContractV1;

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

    /// Extend guardians. Only can be called by owner.
    #[payable]
    pub fn extend_guardians(&mut self, guardians: Vec<ValidAccountId>) {
        self.assert_owner();
        for guardian in guardians {
            self.guardians.insert(guardian.as_ref());
        }
    }

    /// Remove guardians. Only can be called by owner.
    pub fn remove_guardians(&mut self, guardians: Vec<ValidAccountId>) {
        self.assert_owner();
        for guardian in guardians {
            self.guardians.remove(guardian.as_ref());
        }
    }

    /// Change state of contract, Only can be called by owner or guardians.
    #[payable]
    pub fn change_state(&mut self, state: RunningState) {
        assert_one_yocto();
        assert!(self.is_owner_or_guardians(), "ERR_NOT_ALLOWED");

        if self.state != state {
            if state == RunningState::Running {
                // only owner can resume the contract
                self.assert_owner();
            }
            env::log(
                format!(
                    "Contract state changed from {} to {} by {}",
                    self.state, state, env::predecessor_account_id()
                )
                .as_bytes(),
            );       
            self.state = state;
        }
    }

    /// Extend whitelisted tokens with new tokens. Only can be called by owner.
    #[payable]
    pub fn extend_whitelisted_tokens(&mut self, tokens: Vec<ValidAccountId>) {
        assert!(self.is_owner_or_guardians(), "ERR_NOT_ALLOWED");
        for token in tokens {
            self.whitelisted_tokens.insert(token.as_ref());
        }
    }

    /// Remove whitelisted token. Only can be called by owner.
    pub fn remove_whitelisted_tokens(&mut self, tokens: Vec<ValidAccountId>) {
        assert!(self.is_owner_or_guardians(), "ERR_NOT_ALLOWED");
        for token in tokens {
            self.whitelisted_tokens.remove(token.as_ref());
        }
    }

    pub fn modify_admin_fee(&mut self, exchange_fee: u32, referral_fee: u32) {
        self.assert_owner();
        assert!(exchange_fee + referral_fee <= FEE_DIVISOR, "ERR_ILLEGAL_FEE");
        self.exchange_fee = exchange_fee;
        self.referral_fee = referral_fee;
    }

    /// Remove exchange fee liqudity to owner's inner account.
    /// without any storage and fee.
    #[payable]
    pub fn remove_exchange_fee_liquidity(&mut self, pool_id: u64, shares: U128, min_amounts: Vec<U128>) {
        assert_one_yocto();
        self.assert_owner();
        self.assert_contract_running();
        let ex_id = env::current_account_id();
        let owner_id = self.owner_id.clone();
        let mut pool = self.pools.get(pool_id).expect("ERR_NO_POOL");
        let amounts = pool.remove_liquidity(
            &ex_id,
            shares.into(),
            min_amounts
                .into_iter()
                .map(|amount| amount.into())
                .collect(),
        );
        self.pools.replace(pool_id, &pool);
        let tokens = pool.tokens();
        let mut deposits = self.internal_unwrap_or_default_account(&owner_id);
        for i in 0..tokens.len() {
            deposits.deposit(&tokens[i], amounts[i]);
        }
        self.internal_save_account(&owner_id, deposits);
    }

    /// Migration function from v2 to v2.
    /// For next version upgrades, change this function.
    #[init(ignore_state)]
    // [AUDIT_09]
    #[private]
    pub fn migrate() -> Self {
        let prev: ContractV1 = env::state_read().expect("ERR_NOT_INITIALIZED");
        Contract {
            owner_id: prev.owner_id,
            exchange_fee: prev.exchange_fee,
            referral_fee: prev.referral_fee,
            pools: prev.pools,
            accounts: prev.accounts,
            whitelisted_tokens: prev.whitelisted_tokens,
            guardians: UnorderedSet::new(StorageKey::Guardian),
            state: RunningState::Running,
        }
    }

    pub(crate) fn assert_owner(&self) {
        assert_eq!(
            env::predecessor_account_id(),
            self.owner_id,
            "ERR_NOT_ALLOWED"
        );
    }

    pub fn stable_swap_ramp_amp(
        &mut self,
        pool_id: u64,
        future_amp_factor: u64,
        future_amp_time: WrappedTimestamp,
    ) {
        self.assert_owner();
        let mut pool = self.pools.get(pool_id).expect("ERR_NO_POOL");
        match &mut pool {
            Pool::StableSwapPool(pool) => {
                pool.ramp_amplification(future_amp_factor as u128, future_amp_time.0)
            }
            _ => env::panic(b"ERR_NOT_STABLE_POOL"),
        }
        self.pools.replace(pool_id, &pool);
    }

    pub fn stable_swap_stop_ramp_amp(&mut self, pool_id: u64) {
        let mut pool = self.pools.get(pool_id).expect("ERR_NO_POOL");
        match &mut pool {
            Pool::StableSwapPool(pool) => pool.stop_ramp_amplification(),
            _ => env::panic(b"ERR_NOT_STABLE_POOL"),
        }
        self.assert_owner();
        self.pools.replace(pool_id, &pool);
    }

    pub(crate) fn is_owner_or_guardians(&self) -> bool {
        env::predecessor_account_id() == self.owner_id 
            || self.guardians.contains(&env::predecessor_account_id())
    }
}

#[cfg(target_arch = "wasm32")]
mod upgrade {
    use near_sdk::env::BLOCKCHAIN_INTERFACE;
    use near_sdk::Gas;

    use super::*;

    const BLOCKCHAIN_INTERFACE_NOT_SET_ERR: &str = "Blockchain interface not set.";

    /// Gas for calling migration call.
    pub const GAS_FOR_MIGRATE_CALL: Gas = 5_000_000_000_000;

    /// Self upgrade and call migrate, optimizes gas by not loading into memory the code.
    /// Takes as input non serialized set of bytes of the code.
    #[no_mangle]
    pub extern "C" fn upgrade() {
        env::setup_panic_hook();
        env::set_blockchain_interface(Box::new(near_blockchain::NearBlockchain {}));
        let contract: Contract = env::state_read().expect("ERR_CONTRACT_IS_NOT_INITIALIZED");
        contract.assert_owner();
        let current_id = env::current_account_id().into_bytes();
        let method_name = "migrate".as_bytes().to_vec();
        unsafe {
            BLOCKCHAIN_INTERFACE.with(|b| {
                // Load input into register 0.
                b.borrow()
                    .as_ref()
                    .expect(BLOCKCHAIN_INTERFACE_NOT_SET_ERR)
                    .input(0);
                let promise_id = b
                    .borrow()
                    .as_ref()
                    .expect(BLOCKCHAIN_INTERFACE_NOT_SET_ERR)
                    .promise_batch_create(current_id.len() as _, current_id.as_ptr() as _);
                b.borrow()
                    .as_ref()
                    .expect(BLOCKCHAIN_INTERFACE_NOT_SET_ERR)
                    .promise_batch_action_deploy_contract(promise_id, u64::MAX as _, 0);
                let attached_gas = env::prepaid_gas() - env::used_gas() - GAS_FOR_MIGRATE_CALL;
                b.borrow()
                    .as_ref()
                    .expect(BLOCKCHAIN_INTERFACE_NOT_SET_ERR)
                    .promise_batch_action_function_call(
                        promise_id,
                        method_name.len() as _,
                        method_name.as_ptr() as _,
                        0 as _,
                        0 as _,
                        0 as _,
                        attached_gas,
                    );
            });
        }
    }

}
