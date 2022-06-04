use crate::*;
use near_sdk::assert_one_yocto;
use near_sdk::json_types::U128;

#[near_bindgen]
impl Contract {
    pub fn set_owner(&mut self, owner_id: ValidAccountId) {
        self.assert_owner();
        self.data_mut().owner_id = owner_id.into();
    }

    /// force clean 
    pub fn force_clean_farm(&mut self, farm_id: String) -> bool {
        self.assert_owner();
        self.internal_remove_farm_by_farm_id(&farm_id)
    }

    pub fn modify_seed_min_deposit(&mut self, seed_id: String, min_deposit: U128) {
        self.assert_owner();
        let mut farm_seed = self.get_seed(&seed_id);
        farm_seed.get_ref_mut().min_deposit = min_deposit.into();
        self.data_mut().seeds.insert(&seed_id, &farm_seed);
    }

    #[payable]
    pub fn pause_contract(&mut self) {
        assert_one_yocto();
        self.assert_owner();

        if self.data().state == RunningState::Running {
            env::log(format!("Contract paused by {}", env::predecessor_account_id()).as_bytes());       
            self.data_mut().state = RunningState::Paused;
        } else {
            env::log("Contract state is already in Paused".as_bytes());
        }
    }

    #[payable]
    pub fn resume_contract(&mut self) {
        assert_one_yocto();
        self.assert_owner();

        if self.data().state == RunningState::Paused {
            env::log(format!("Contract resumed by {}", env::predecessor_account_id()).as_bytes());       
            self.data_mut().state = RunningState::Running;
        } else {
            env::log("Contract state is already in Running".as_bytes());
        }
    }

    /// Migration function between versions.
    /// For next version upgrades, change this function.
    #[init(ignore_state)]
    #[private]
    pub fn migrate() -> Self {
        let mut contract: Contract = env::state_read().expect("ERR_NOT_INITIALIZED");
        // see if ContractData need upgrade
        contract.data = 
        match contract.data {
            VersionedContractData::V0104(data) => VersionedContractData::V0110(data.into()),
            VersionedContractData::V0110(data) => VersionedContractData::V0110(data),
        };
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