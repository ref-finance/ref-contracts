//! Implement all the relevant logic for owner of this contract.

use degen_swap::degen::{global_register_degen, global_register_degen_oracle_config, global_unregister_degen, global_unregister_degen_oracle_config, DegenOracleConfig, DegenType};
use near_sdk::json_types::WrappedTimestamp;
use near_contract_standards::fungible_token::core_impl::ext_fungible_token;

use crate::*;
use crate::rated_swap::rate::{global_register_rate, global_unregister_rate, global_update_rated_token_extra_info};
use crate::utils::{FEE_DIVISOR, MAX_ADMIN_FEE_BPS, GAS_FOR_BASIC_OP};

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
            let exist = self.guardians.remove(guardian.as_ref());
            // [AUDITION_AMENDMENT] 2.3.1 Lack of Check on Guardians’ Removal
            assert!(exist, "{}", ERR104_GUARDIAN_NOT_IN_LIST);
        }
    }

    #[payable]
    pub fn extend_auto_whitelisted_postfix(&mut self, postfixes: Vec<String>) {
        assert_one_yocto();
        self.is_owner_or_guardians();
        for postfix in postfixes {
            self.auto_whitelisted_postfix.insert(postfix.clone());
        }
    }

    #[payable]
    pub fn remove_auto_whitelisted_postfix(&mut self, postfixes: Vec<String>) {
        assert_one_yocto();
        self.is_owner_or_guardians();
        for postfix in postfixes {
            let exist = self.auto_whitelisted_postfix.remove(&postfix);
            assert!(exist, "{}", ERR105_WHITELISTED_POSTFIX_NOT_IN_LIST);
        }
    }

    #[payable]
    pub fn modify_boost_farm_id(&mut self, boost_farm_id: AccountId) {
        assert_one_yocto();
        assert!(self.is_owner_or_guardians(), "{}", ERR100_NOT_ALLOWED);
        log!("Modify boost_farm_id from {} to {}", self.boost_farm_id, boost_farm_id);  
        self.boost_farm_id = boost_farm_id;
    }

    #[payable]
    pub fn modify_burrowland_id(&mut self, burrowland_id: AccountId) {
        assert_one_yocto();
        assert!(self.is_owner_or_guardians(), "{}", ERR100_NOT_ALLOWED);
        log!("Modify burrowland_id from {} to {}", self.burrowland_id, burrowland_id);  
        self.burrowland_id = burrowland_id;
    }

    #[payable]
    pub fn modify_wnear_id(&mut self, wnear_id: AccountId) {
        assert_one_yocto();
        self.assert_owner();
        log!("Modify wnear_id from {:?} to {}", self.wnear_id, wnear_id);  
        self.wnear_id = Some(wnear_id);
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
            let exist = self.whitelisted_tokens.remove(token.as_ref());
            assert!(exist, "{}", ERR53_TOKEN_NOT_IN_LIST);
        }
    }

    /// Extend frozenlist tokens with new tokens.
    #[payable]
    pub fn extend_frozenlist_tokens(&mut self, tokens: Vec<ValidAccountId>) {
        assert_one_yocto();
        assert!(self.is_owner_or_guardians(), "{}", ERR100_NOT_ALLOWED);
        for token in tokens {
            self.frozen_tokens.insert(token.as_ref());
        }
    }

    /// Remove frozenlist token.
    #[payable]
    pub fn remove_frozenlist_tokens(&mut self, tokens: Vec<ValidAccountId>) {
        assert_one_yocto();
        assert!(self.is_owner_or_guardians(), "{}", ERR100_NOT_ALLOWED);
        for token in tokens {
            let exist = self.frozen_tokens.remove(token.as_ref());
            assert!(exist, "{}", ERR53_TOKEN_NOT_IN_LIST);
        }
    }

    /// insert referral with given fee_bps
    #[payable]
    pub fn insert_referral(&mut self, referral_id: ValidAccountId, fee_bps: u32) {
        assert_one_yocto();
        assert!(self.is_owner_or_guardians(), "{}", ERR100_NOT_ALLOWED);
        let referral_id: AccountId = referral_id.into();
        assert!(fee_bps > 0 && fee_bps < FEE_DIVISOR, "{}", ERR132_ILLEGAL_REFERRAL_FEE);
        let old_fee_bps = self.referrals.insert(&referral_id, &fee_bps);
        assert!(old_fee_bps.is_none(), "{}", ERR130_REFERRAL_EXIST);
        env::log(
            format!(
                "Insert referral {} with fee_bps {}",
                referral_id, fee_bps
            )
            .as_bytes(),
        );     
    }

    /// update referral with given fee_bps
    #[payable]
    pub fn update_referral(&mut self, referral_id: ValidAccountId, fee_bps: u32) {
        assert_one_yocto();
        assert!(self.is_owner_or_guardians(), "{}", ERR100_NOT_ALLOWED);
        let referral_id: AccountId = referral_id.into();
        assert!(fee_bps > 0 && fee_bps < FEE_DIVISOR, "{}", ERR132_ILLEGAL_REFERRAL_FEE);
        let old_fee_bps = self.referrals.insert(&referral_id, &fee_bps);
        assert!(old_fee_bps.is_some(), "{}", ERR131_REFERRAL_NOT_EXIST);
        env::log(
            format!(
                "Update referral {} with new fee_bps {} where old fee_bps {}",
                referral_id, fee_bps, old_fee_bps.unwrap()
            )
            .as_bytes(),
        );     
    }

    /// remove referral
    #[payable]
    pub fn remove_referral(&mut self, referral_id: ValidAccountId) {
        assert_one_yocto();
        assert!(self.is_owner_or_guardians(), "{}", ERR100_NOT_ALLOWED);
        let referral_id: AccountId = referral_id.into();
        let old_fee_bps = self.referrals.remove(&referral_id);
        assert!(old_fee_bps.is_some(), "{}", ERR131_REFERRAL_NOT_EXIST);
        env::log(
            format!(
                "Remove referral {} where fee_bps {}",
                referral_id, old_fee_bps.unwrap()
            )
            .as_bytes(),
        );     
    }

    /// [AUDITION_AMENDMENT] 2.3.4 Improper Check on the Admin Fees
    /// As referral_fee has been set per referral, the global referral_fee is obsolete.
    #[payable]
    pub fn modify_admin_fee(&mut self, admin_fee_bps: u32) {
        assert_one_yocto();
        self.assert_owner();
        assert!(admin_fee_bps <= MAX_ADMIN_FEE_BPS, "{}", ERR101_ILLEGAL_FEE);
        self.admin_fee_bps = admin_fee_bps;
    }

    #[payable]
    pub fn modify_total_fee(&mut self, pool_id: u64, total_fee: u32) {
        assert_one_yocto();
        assert!(self.is_owner_or_guardians(), "{}", ERR100_NOT_ALLOWED);
        assert!(total_fee < FEE_DIVISOR, "{}", ERR62_FEE_ILLEGAL);
        let mut pool = self.pools.get(pool_id).expect(ERR85_NO_POOL);
        env::log(
            format!("Modify total_fee pool_id {} from {} to {}", pool_id, pool.get_fee(), total_fee).as_bytes()
        );
        pool.modify_total_fee(total_fee);
        self.pools.replace(pool_id, &pool);
    }

    /// Remove exchange fee liquidity to owner's inner account.
    /// Owner's inner account storage should be prepared in advance.
    #[payable]
    pub fn remove_exchange_fee_liquidity(&mut self, pool_id: u64, shares: U128, min_amounts: Vec<U128>) {
        assert_one_yocto();
        assert!(self.is_owner_or_guardians(), "{}", ERR100_NOT_ALLOWED);
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
            false
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
        skip_unwrap_near: Option<bool>
    ) -> Promise {
        assert_one_yocto();
        assert!(self.is_owner_or_guardians(), "{}", ERR100_NOT_ALLOWED);
        self.assert_contract_running();
        let token_id: AccountId = token_id.into();
        let amount: u128 = amount.into();
        assert!(amount > 0, "{}", ERR29_ILLEGAL_WITHDRAW_AMOUNT);
        let owner_id = self.owner_id.clone();
        let mut account = self.internal_unwrap_account(&owner_id);
        // Note: subtraction and deregistration will be reverted if the promise fails.
        account.withdraw(&token_id, amount);
        self.internal_save_account(&owner_id, account);
        self.internal_send_tokens(&owner_id, &token_id, amount, skip_unwrap_near)
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
            Pool::DegenSwapPool(pool) => {
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
            Pool::DegenSwapPool(pool) => pool.stop_ramp_amplification(),
            _ => env::panic(ERR88_NOT_STABLE_POOL.as_bytes()),
        }
        self.pools.replace(pool_id, &pool);
    }

    /// Register new rated token.
    #[payable]
    pub fn register_rated_token(&mut self, rate_type: String, token_id: ValidAccountId, extra_info: Option<String>) {
        assert_one_yocto();
        assert!(self.is_owner_or_guardians(), "{}", ERR100_NOT_ALLOWED);
        let token_id: AccountId = token_id.into();
        if global_register_rate(&rate_type, &token_id, extra_info) {
            log!("New {} typed rated token {} registered by {}", rate_type, token_id, env::predecessor_account_id());
        } else {
            env::panic(format!("Rated token {} already exist", token_id).as_bytes());
        }
    }

    /// Remove rated token. Only owner can call.
    #[payable]
    pub fn unregister_rated_token(&mut self, token_id: ValidAccountId) {
        assert_one_yocto();
        self.assert_owner();
        let token_id: AccountId = token_id.into();
        if global_unregister_rate(&token_id) {
            log!("Rated token {} removed.", token_id);
        } else {
            log!("Rated token {} not exist in rate list.", token_id);
        }
    }

    #[payable]
    pub fn update_rated_token_extra_info(&mut self, token_id: ValidAccountId, extra_info: String) {
        assert_one_yocto();
        assert!(self.is_owner_or_guardians(), "{}", ERR100_NOT_ALLOWED);
        let token_id: AccountId = token_id.into();
        global_update_rated_token_extra_info(&token_id, extra_info.clone());
        log!("Update rated token {} extra info: {}", token_id, extra_info);
    }

    /// Register new degen token.
    #[payable]
    pub fn register_degen_token(&mut self, token_id: ValidAccountId, degen_type: DegenType) {
        assert_one_yocto();
        assert!(self.is_owner_or_guardians(), "{}", ERR100_NOT_ALLOWED);
        let token_id: AccountId = token_id.into();
        if global_register_degen(&token_id, degen_type.clone()) {
            log!("New {:?} typed degen token {} registered by {}", degen_type, token_id, env::predecessor_account_id());
        } else {
            env::panic(format!("Degen token {} already exist", token_id).as_bytes());
        }
    }

    /// Remove degen token. Only owner can call.
    #[payable]
    pub fn unregister_degen_token(&mut self, token_id: ValidAccountId) {
        assert_one_yocto();
        self.assert_owner();
        let token_id: AccountId = token_id.into();
        if global_unregister_degen(&token_id) {
            log!("Degen token {} removed.", token_id);
        } else {
            log!("Degen token {} not exist in degen list.", token_id);
        }
    }

    /// Register new degen oracle config.
    #[payable]
    pub fn register_degen_oracle_config(&mut self, degen_oracle_config: DegenOracleConfig) {
        assert_one_yocto();
        assert!(self.is_owner_or_guardians(), "{}", ERR100_NOT_ALLOWED);
        if global_register_degen_oracle_config(degen_oracle_config.clone()) {
            log!("New degen oracle config {} registered by {}", degen_oracle_config.get_key(), env::predecessor_account_id());
        } else {
            env::panic(format!("Degen oracle config {} already exist", degen_oracle_config.get_key()).as_bytes());
        }
    }

    /// Remove degen oracle config. Only owner can call.
    #[payable]
    pub fn unregister_degen_oracle_config(&mut self, degen_oracle_config_key: String) {
        assert_one_yocto();
        self.assert_owner();
        if global_unregister_degen_oracle_config(&degen_oracle_config_key) {
            log!("Degen oracle config {} removed.", degen_oracle_config_key);
        } else {
            log!("Degen oracle config {} not exist in degen list.", degen_oracle_config_key);
        }
    }

    /// Update new degen oracle config.
    #[payable]
    pub fn update_degen_oracle_config(&mut self, degen_oracle_config: DegenOracleConfig) {
        assert_one_yocto();
        self.assert_owner();
        if global_update_degen_oracle_config(degen_oracle_config.clone()) {
            log!("Update oracle degen config {} registered by {}", degen_oracle_config.get_key(), env::predecessor_account_id());
        } else {
            env::panic(format!("Degen oracle config {} not exist", degen_oracle_config.get_key()).as_bytes());
        }
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

    /// Migration function from v1.6.x to v1.7.0.
    /// For next version upgrades, change this function.
    #[init(ignore_state)]
    // [AUDIT_09]
    #[private]
    pub fn migrate() -> Self {
        env::state_read().expect(ERR103_NOT_INITIALIZED)
    }
}


#[cfg(target_arch = "wasm32")]
mod upgrade {
    use near_sdk::env::BLOCKCHAIN_INTERFACE;
    use near_sdk::Gas;

    use super::*;

    const BLOCKCHAIN_INTERFACE_NOT_SET_ERR: &str = "Blockchain interface not set.";

    /// Gas for calling migration call.
    // [AUDITION_AMENDMENT] 2.3.6 Lack of Check on the Gas Used by migrate()
    const GAS_TO_COMPLETE_UPGRADE_CALL: Gas = 10_000_000_000_000;
    const GAS_FOR_GET_CONFIG_CALL: Gas = 5_000_000_000_000;
    const MIN_GAS_FOR_MIGRATE_STATE_CALL: Gas = 50_000_000_000_000;

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
        let view_name = "metadata".as_bytes().to_vec();
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
                // 1st action in the Tx: "deploy contract" (code is taken from register 0)
                b.borrow()
                    .as_ref()
                    .expect(BLOCKCHAIN_INTERFACE_NOT_SET_ERR)
                    .promise_batch_action_deploy_contract(promise_id, u64::MAX as _, 0);

                let required_gas = env::used_gas() + GAS_TO_COMPLETE_UPGRADE_CALL + GAS_FOR_GET_CONFIG_CALL;
                assert!(
                    env::prepaid_gas() >= required_gas + MIN_GAS_FOR_MIGRATE_STATE_CALL,
                    "Not enough gas to complete state migration"
                );
                let migrate_state_attached_gas = env::prepaid_gas() - required_gas;
                // 2nd action in the Tx: call this_contract.migrate() with remaining gas
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
                        migrate_state_attached_gas,
                    );
                // 
                // Scheduling to return metadata after the migration is completed.
                //
                // The upgrade method attaches it as an action, so the entire upgrade including deploy
                // contract action and migration can be rolled back if the view call can't be
                // returned successfully. The view call deserializes the root state which contains the owner_id. 
                // If the contract can deserialize the current root state,
                // then it can validate the owner and execute the upgrade again (in case the previous
                // upgrade/migration went badly).
                //
                // It's an extra safety guard for the remote contract upgrades.
                b.borrow()
                    .as_ref()
                    .expect(BLOCKCHAIN_INTERFACE_NOT_SET_ERR)
                    .promise_batch_action_function_call(
                        promise_id,
                        view_name.len() as _,
                        view_name.as_ptr() as _,
                        0 as _,
                        0 as _,
                        0 as _,
                        GAS_FOR_GET_CONFIG_CALL,
                    );
            });
        }
    }

}
