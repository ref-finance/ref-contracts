use crate::*;
use crate::errors::*;
use crate::utils::{
    assert_one_yocto, ext_fungible_token, ext_multi_fungible_token, ext_self, parse_seed_id,
    wrap_mft_token_id, GAS_FOR_FT_TRANSFER, GAS_FOR_RESOLVE_WITHDRAW_SEED,
};
use std::convert::TryInto;
use near_sdk::Promise;
use near_sdk::json_types::U128;
use near_sdk::collections::UnorderedSet;

#[near_bindgen]
impl Contract {
    #[payable]
    pub fn set_owner(&mut self, owner_id: ValidAccountId) {
        assert_one_yocto();
        self.assert_owner();
        self.data_mut().owner_id = owner_id.into();
    }

    /// Extend operators. Only can be called by owner.
    #[payable]
    pub fn extend_operators(&mut self, operators: Vec<ValidAccountId>) {
        assert_one_yocto();
        self.assert_owner();
        for operator in operators {
            self.data_mut().operators.insert(operator.as_ref());
        }
    }

    /// Remove operators. Only can be called by owner.
    #[payable]
    pub fn remove_operators(&mut self, operators: Vec<ValidAccountId>) {
        assert_one_yocto();
        self.assert_owner();
        for operator in operators {
            self.data_mut().operators.remove(operator.as_ref());
        }
    }

    #[payable]
    pub fn modify_default_farm_expire_sec(&mut self, farm_expire_sec: u32) {
        assert_one_yocto();
        self.assert_owner();
        self.data_mut().farm_expire_sec = farm_expire_sec;
    }

    #[payable]
    pub fn modify_seed_min_deposit(&mut self, seed_id: String, min_deposit: U128) {
        assert_one_yocto();
        assert!(self.is_owner_or_operators(), "ERR_NOT_ALLOWED");
        let mut farm_seed = self.get_seed(&seed_id);
        farm_seed.get_ref_mut().min_deposit = min_deposit.into();
        self.data_mut().seeds.insert(&seed_id, &farm_seed);
    }

    #[payable]
    pub fn modify_seed_slash_rate(&mut self, seed_id: String, slash_rate: u32) {
        assert_one_yocto();
        assert!(self.is_owner_or_operators(), "ERR_NOT_ALLOWED");
        assert!(slash_rate as u128 <= DENOM, "INVALID_SLASH_RATE");
        let mut farm_seed = self.get_seed(&seed_id);
        farm_seed.get_ref_mut().slash_rate = slash_rate;
        self.data_mut().seeds.insert(&seed_id, &farm_seed);
    }

    #[payable]
    pub fn modify_cd_strategy_item(&mut self, index: usize, lock_sec: u32, power_reward_rate: u32) {
        assert_one_yocto();
        assert!(self.is_owner_or_operators(), "ERR_NOT_ALLOWED");
        assert!(index < STRATEGY_LIMIT, "{}", ERR62_INVALID_CD_STRATEGY_INDEX);

        if lock_sec == 0 {
            self.data_mut().cd_strategy.stake_strategy[index] = CDStakeItem{
                lock_sec: 0,
                power_reward_rate: 0,
                enable: false,
            };
        } else {
            self.data_mut().cd_strategy.stake_strategy[index] = CDStakeItem{
                lock_sec,
                power_reward_rate,
                enable: true,
            };
        }
    }

    #[payable]
    pub fn modify_default_seed_slash_rate(&mut self, slash_rate: u32) {
        assert_one_yocto();
        assert!(self.is_owner_or_operators(), "ERR_NOT_ALLOWED");
        self.data_mut().cd_strategy.seed_slash_rate = slash_rate;
    }

    /// Owner retrieve those slashed seed
    #[payable]
    pub fn withdraw_seed_slashed(&mut self, seed_id: SeedId) -> Promise {
        assert_one_yocto();
        assert!(self.is_owner_or_operators(), "ERR_NOT_ALLOWED");
        self.assert_owner();
        let sender_id = self.data().owner_id.clone();
        // update inner state
        let amount = self.data_mut().seeds_slashed.remove(&seed_id).unwrap();
        assert!(amount > 0, "{}", ERR32_NOT_ENOUGH_SEED);

        let (receiver_id, token_id) = parse_seed_id(&seed_id);
        if receiver_id == token_id {
            ext_fungible_token::ft_transfer(
                sender_id.clone().try_into().unwrap(),
                amount.into(),
                None,
                &seed_id,
                1, // one yocto near
                GAS_FOR_FT_TRANSFER,
            )
            .then(ext_self::callback_withdraw_seed_slashed(
                seed_id.clone(),
                amount.into(),
                &env::current_account_id(),
                0,
                GAS_FOR_RESOLVE_WITHDRAW_SEED,
            ))
        } else {
            ext_multi_fungible_token::mft_transfer(
                wrap_mft_token_id(&token_id),
                sender_id.clone().try_into().unwrap(),
                amount.into(),
                None,
                &receiver_id,
                1, // one yocto near
                GAS_FOR_FT_TRANSFER,
            )
            .then(ext_self::callback_withdraw_seed_slashed(
                seed_id.clone(),
                amount.into(),
                &env::current_account_id(),
                0,
                GAS_FOR_RESOLVE_WITHDRAW_SEED,
            ))
        }
    }

    /// owner help to return those who lost seed when withdraw,
    /// It's owner's responsibility to verify amount and seed id before calling
    #[payable]
    pub fn return_seed_lostfound(&mut self, sender_id: ValidAccountId, seed_id: SeedId, amount: Balance) -> Promise {
        assert_one_yocto();
        self.assert_owner();
        let sender_id: AccountId = sender_id.into();
        // update inner state
        let max_amount = self.data().seeds_lostfound.get(&seed_id).unwrap();
        assert!(amount <= max_amount, "{}", ERR32_NOT_ENOUGH_SEED);
        self.data_mut().seeds_lostfound.insert(&seed_id, &(max_amount - amount));

        let (receiver_id, token_id) = parse_seed_id(&seed_id);
        if receiver_id == token_id {
            ext_fungible_token::ft_transfer(
                sender_id.clone().try_into().unwrap(),
                amount.into(),
                None,
                &seed_id,
                1, // one yocto near
                GAS_FOR_FT_TRANSFER,
            )
            .then(ext_self::callback_withdraw_seed_lostfound(
                seed_id.clone(),
                sender_id.clone(),
                amount.into(),
                &env::current_account_id(),
                0,
                GAS_FOR_RESOLVE_WITHDRAW_SEED,
            ))
        } else {
            ext_multi_fungible_token::mft_transfer(
                wrap_mft_token_id(&token_id),
                sender_id.clone().try_into().unwrap(),
                amount.into(),
                None,
                &receiver_id,
                1, // one yocto near
                GAS_FOR_FT_TRANSFER,
            )
            .then(ext_self::callback_withdraw_seed_lostfound(
                seed_id.clone(),
                sender_id.clone(),
                amount.into(),
                &env::current_account_id(),
                0,
                GAS_FOR_RESOLVE_WITHDRAW_SEED,
            ))
        }
    }

    /// Migration function between versions.
    /// For next version upgrades, change this function.
    #[init(ignore_state)]
    #[private]
    pub fn migrate() -> Self {
        let mut contract: Contract = env::state_read().expect("ERR_NOT_INITIALIZED");
        let data = match contract.data {
            VersionedContractData::V200(data) => {
                ContractData {
                    owner_id: data.owner_id,
                    farmer_count: data.farmer_count,
                    seeds: data.seeds,
                    seeds_slashed: data.seeds_slashed,
                    seeds_lostfound: data.seeds_lostfound,
                    farmers: data.farmers,
                    farms: data.farms,
                    outdated_farms: data.outdated_farms,
                    reward_info: data.reward_info,
                    cd_strategy: data.cd_strategy,
                    farm_expire_sec: DEFAULT_FARM_EXPIRE_SEC,
                    operators: UnorderedSet::new(StorageKeys::Operator),
                }
            },
            VersionedContractData::V201(data) => data,
        };
        contract.data = VersionedContractData::V201(data);
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

#[cfg(target_arch = "wasm32")]
mod upgrade {
    use near_sdk::env::BLOCKCHAIN_INTERFACE;
    use near_sdk::Gas;

    use super::*;

    const BLOCKCHAIN_INTERFACE_NOT_SET_ERR: &str = "Blockchain interface not set.";

    /// Gas for calling migration call.
    pub const GAS_FOR_MIGRATE_CALL: Gas = 10_000_000_000_000;

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