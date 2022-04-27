//! Implement all the relevant logic for owner of this contract.

use near_sdk::json_types::WrappedTimestamp;
use near_contract_standards::fungible_token::core_impl::ext_fungible_token;

use crate::*;
use crate::utils::{FEE_DIVISOR, GAS_FOR_BASIC_OP};

#[near_bindgen]
impl Contract {
    /// Change owner. Only can be called by owner.
    #[payable]
    pub fn set_owner(&mut self, owner_id: ValidAccountId) {
        assert_one_yocto();
        self.assert_owner();
        self.owner_id = owner_id.as_ref().clone();
    }

    /// Get the owner of this account.
    pub fn get_owner(&self) -> AccountId {
        self.owner_id.clone()
    }

    /// Retrieve NEP-141 tokens that not mananged by contract to owner,
    /// Caution: Must check that `amount <= total_amount_in_account - amount_managed_by_contract` before calling !!!
    /// Returns promise of ft_transfer action.
    #[payable]
    pub fn retrieve_unmanaged_token(&mut self, token_id: ValidAccountId, amount: U128) -> Promise {
        self.assert_owner();
        assert_one_yocto();
        let token_id: AccountId = token_id.into();
        let amount: u128 = amount.into();
        assert!(amount > 0, "{}", ERR29_ILLEGAL_WITHDRAW_AMOUNT);
        env::log(
            format!(
                "Going to retrieve token {} to owner, amount: {}",
                &token_id, amount
            )
            .as_bytes(),
        ); 
        ext_fungible_token::ft_transfer(
            self.owner_id.clone(),
            U128(amount),
            None,
            &token_id,
            1,
            env::prepaid_gas() - GAS_FOR_BASIC_OP,
        )
    }

    /// Extend guardians. Only can be called by owner.
    #[payable]
    pub fn extend_guardians(&mut self, guardians: Vec<ValidAccountId>) {
        assert_one_yocto();
        self.assert_owner();
        for guardian in guardians {
            self.guardians.insert(guardian.as_ref());
        }
    }

    /// Remove guardians. Only can be called by owner.
    #[payable]
    pub fn remove_guardians(&mut self, guardians: Vec<ValidAccountId>) {
        assert_one_yocto();
        self.assert_owner();
        for guardian in guardians {
            self.guardians.remove(guardian.as_ref());
        }
    }

    /// Change state of contract, Only can be called by owner or guardians.
    #[payable]
    pub fn change_state(&mut self, state: RunningState) {
        assert_one_yocto();
        assert!(self.is_owner_or_guardians(), "{}", ERR100_NOT_ALLOWED);

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
        assert_one_yocto();
        assert!(self.is_owner_or_guardians(), "{}", ERR100_NOT_ALLOWED);
        for token in tokens {
            self.whitelisted_tokens.insert(token.as_ref());
        }
    }

    /// Remove whitelisted token. Only can be called by owner.
    #[payable]
    pub fn remove_whitelisted_tokens(&mut self, tokens: Vec<ValidAccountId>) {
        assert_one_yocto();
        assert!(self.is_owner_or_guardians(), "{}", ERR100_NOT_ALLOWED);
        for token in tokens {
            self.whitelisted_tokens.remove(token.as_ref());
        }
    }

    #[payable]
    pub fn modify_admin_fee(&mut self, exchange_fee: u32, referral_fee: u32) {
        assert_one_yocto();
        self.assert_owner();
        assert!(exchange_fee + referral_fee <= FEE_DIVISOR, "{}", ERR101_ILLEGAL_FEE);
        self.exchange_fee = exchange_fee;
        self.referral_fee = referral_fee;
    }

    /// Remove exchange fee liquidity to owner's inner account.
    /// Owner's inner account storage should be prepared in advance.
    #[payable]
    pub fn remove_exchange_fee_liquidity(&mut self, pool_id: u64, shares: U128, min_amounts: Vec<U128>) {
        assert_one_yocto();
        assert!(self.is_owner_or_guardians(), "ERR_NOT_ALLOWED");
        self.assert_contract_running();
        let ex_id = env::current_account_id();
        let owner_id = self.owner_id.clone();
        let mut pool = self.pools.get(pool_id).expect(ERR85_NO_POOL);
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
        let mut deposits = self.internal_unwrap_account(&owner_id);
        for i in 0..tokens.len() {
            deposits.deposit(&tokens[i], amounts[i]);
        }
        self.internal_save_account(&owner_id, deposits);
    }

    /// Withdraw owner inner account token to owner wallet.
    /// Owner inner account should be prepared in advance.
    #[payable]
    pub fn withdraw_owner_token(
        &mut self,
        token_id: ValidAccountId,
        amount: U128,
    ) -> Promise {
        assert_one_yocto();
        assert!(self.is_owner_or_guardians(), "ERR_NOT_ALLOWED");
        self.assert_contract_running();
        let token_id: AccountId = token_id.into();
        let amount: u128 = amount.into();
        assert!(amount > 0, "{}", ERR29_ILLEGAL_WITHDRAW_AMOUNT);
        let owner_id = self.owner_id.clone();
        let mut account = self.internal_unwrap_account(&owner_id);
        // Note: subtraction and deregistration will be reverted if the promise fails.
        account.withdraw(&token_id, amount);
        self.internal_save_account(&owner_id, account);
        self.internal_send_tokens(&owner_id, &token_id, amount)
    }

    /// to eventually change a stable pool's amp factor
    /// pool_id: the target stable pool;
    /// future_amp_factor: the target amp factor, could be less or more than current one;
    /// future_amp_time: the endtime of the increasing or decreasing process;
    #[payable]
    pub fn stable_swap_ramp_amp(
        &mut self,
        pool_id: u64,
        future_amp_factor: u64,
        future_amp_time: WrappedTimestamp,
    ) {
        assert_one_yocto();
        assert!(self.is_owner_or_guardians(), "{}", ERR100_NOT_ALLOWED);
        let mut pool = self.pools.get(pool_id).expect(ERR85_NO_POOL);
        match &mut pool {
            Pool::StableSwapPool(pool) => {
                pool.ramp_amplification(future_amp_factor as u128, future_amp_time.0)
            }
            Pool::RatedSwapPool(pool) => {
                pool.ramp_amplification(future_amp_factor as u128, future_amp_time.0)
            }
            _ => env::panic(ERR88_NOT_STABLE_POOL.as_bytes()),
        }
        self.pools.replace(pool_id, &pool);
    }

    #[payable]
    pub fn stable_swap_stop_ramp_amp(&mut self, pool_id: u64) {
        assert_one_yocto();
        assert!(self.is_owner_or_guardians(), "{}", ERR100_NOT_ALLOWED);
        let mut pool = self.pools.get(pool_id).expect(ERR85_NO_POOL);
        match &mut pool {
            Pool::StableSwapPool(pool) => pool.stop_ramp_amplification(),
            Pool::RatedSwapPool(pool) => pool.stop_ramp_amplification(),
            _ => env::panic(ERR88_NOT_STABLE_POOL.as_bytes()),
        }
        self.pools.replace(pool_id, &pool);
    }

    ///
    #[payable]
    pub fn rated_swap_ramp_amp(
        &mut self,
        pool_id: u64,
        future_amp_factor: u64,
        future_amp_time: WrappedTimestamp,
    ) {
        self.stable_swap_ramp_amp(pool_id, future_amp_factor, future_amp_time)
    }

    ///
    #[payable]
    pub fn rated_swap_stop_ramp_amp(&mut self, pool_id: u64) {
        self.stable_swap_stop_ramp_amp(pool_id)
    }

    pub(crate) fn assert_owner(&self) {
        assert_eq!(
            env::predecessor_account_id(),
            self.owner_id,
            "{}", ERR100_NOT_ALLOWED
        );
    }

    pub(crate) fn is_owner_or_guardians(&self) -> bool {
        env::predecessor_account_id() == self.owner_id 
            || self.guardians.contains(&env::predecessor_account_id())
    }

    /// Migration function from v2 to v2.
    /// For next version upgrades, change this function.
    #[init(ignore_state)]
    // [AUDIT_09]
    #[private]
    pub fn migrate() -> Self {
        let contract: Contract = env::state_read().expect(ERR103_NOT_INITIALIZED);
        contract
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
        let contract: Contract = env::state_read().expect(ERR103_NOT_INITIALIZED);
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
