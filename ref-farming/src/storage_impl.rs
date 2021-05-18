use near_contract_standards::storage_management::{
    StorageBalance, StorageBalanceBounds, StorageManagement,
};

use std::convert::TryInto;

use near_sdk::json_types::{ValidAccountId, U128};
use near_sdk::{assert_one_yocto, env, log, near_bindgen, Promise, Balance};

use crate::errors::*;
use crate::*;
use crate::farmer::MIN_FARMER_LENGTH;
use crate::utils::MAX_ACCOUNT_LENGTH;



/// Implements users storage management for the pool.
#[near_bindgen]
impl StorageManagement for Contract {
    #[payable]
    fn storage_deposit(
        &mut self,
        account_id: Option<ValidAccountId>,
        registration_only: Option<bool>,
    ) -> StorageBalance {

        let amount = env::attached_deposit();
        let account_id = account_id
            .map(|a| a.into())
            .unwrap_or_else(|| env::predecessor_account_id());
        let registration_only = registration_only.unwrap_or(false);

        let (locked, deposited) = self.internal_farmer_storage(&account_id);
        if deposited == 0 {  // new account register
            if amount < Contract::suggested_min_storage_usage() {
                env::panic(format!("{}", ERR11_INSUFFICIENT_STORAGE).as_bytes());
            }
            if registration_only {
                self.internal_register_account(&account_id, Contract::suggested_min_storage_usage());
                let refund = amount - Contract::suggested_min_storage_usage();
                if refund > 0 {
                    Promise::new(env::predecessor_account_id()).transfer(refund);
                }
            } else {
                self.internal_register_account(&account_id, amount);
            }
        } else {  // old account, only can complement storage fee
            if registration_only {
                log!("ERR_ACC_REGISTERED");
                if amount > 0 {
                    Promise::new(env::predecessor_account_id()).transfer(amount);
                }
            } else {
                if amount+deposited < locked {
                    env::panic(format!("{}", ERR11_INSUFFICIENT_STORAGE).as_bytes());
                }
                self.internal_register_account(&account_id, amount);
            }
        }
        self.storage_balance_of(account_id.try_into().unwrap())
            .unwrap()
    }

    #[payable]
    fn storage_withdraw(&mut self, amount: Option<U128>) -> StorageBalance {
        assert_one_yocto();

        let account_id = env::predecessor_account_id();        
        let (locked, deposited) = self.internal_farmer_storage(&account_id);
        if deposited > 0 {
            let amount = amount.map(|a| a.0).unwrap_or(deposited);
            assert!(deposited >= locked + amount, "{}", ERR11_INSUFFICIENT_STORAGE);
            Promise::new(account_id.clone()).transfer(amount);
            StorageBalance {
                total: locked.into(),
                available: (deposited - amount).into(),
            }
        } else {
            env::panic(format!("{}", ERR10_ACC_NOT_REGISTERED).as_bytes());
        }
    }

    #[allow(unused_variables)]
    #[payable]
    fn storage_unregister(&mut self, force: Option<bool>) -> bool {
        assert_one_yocto();

        let account_id = env::predecessor_account_id();
        if let Some(farmer) = self.get_farmer_wrapped(&account_id) {
            // TODO: figure out force option logic.
            assert!(
                farmer.get_ref().rewards.is_empty(),
                "{}", ERR12_STORAGE_UNREGISTER_REWARDS_NOT_EMPTY
            );
            assert!(
                farmer.get_ref().seeds.is_empty(),
                "{}", ERR13_STORAGE_UNREGISTER_SEED_NOT_EMPTY
            );
            self.data_mut().farmers.remove(&account_id);
            self.data_mut().farmer_count -= 1;
            Promise::new(account_id.clone()).transfer(farmer.get_ref().amount);
            true
        } else {
            false
        }
    }

    fn storage_balance_bounds(&self) -> StorageBalanceBounds {
        StorageBalanceBounds {
            min: Contract::suggested_min_storage_usage().into(),
            max: None,
        }
    }

    fn storage_balance_of(&self, account_id: ValidAccountId) -> Option<StorageBalance> {
        let (locked, deposited) = self.internal_farmer_storage(account_id.as_ref()); 
        if locked > 0 {
            Some(StorageBalance {
                total: U128(locked),
                available: U128(deposited),
            })
        } else {
           None
        }
    }
}

impl Contract {

    /// return storage used by given account, and his deposited storage fee 
    pub(crate) fn internal_farmer_storage(
        &self, 
        account_id: &AccountId
    ) -> (Balance, Balance) {
        let farmer = self.get_farmer_wrapped(account_id);
        if let Some(farmer) = farmer {
            (farmer.get_ref().storage_usage(), farmer.get_ref().amount)
        } else {
           (0, 0)
        }
    }

    pub(crate) fn assert_storage_usage(&self, account_id: &AccountId) {
        let (locked, deposited) = self.internal_farmer_storage(account_id);
        assert!(
            deposited > 0,
            "{}",
            ERR10_ACC_NOT_REGISTERED
        );
        assert!(
            locked <= deposited,
            "{}",
            ERR11_INSUFFICIENT_STORAGE
        );
    }

    /// Returns minimal storage usage possible.
    /// 5 reward tokens, 5 seed tokens, 10 farms as assumption.
    pub(crate) fn suggested_min_storage_usage() -> Balance {
        (
            MIN_FARMER_LENGTH 
            + 2_u128 * 5_u128 * (MAX_ACCOUNT_LENGTH + 16)
            + 10_u128 * (MAX_ACCOUNT_LENGTH + 32)
        ) * env::storage_byte_cost()
    }

    pub(crate) fn internal_register_account(&mut self, account_id: &AccountId, amount: Balance) {

        if let Some(mut farmer) = self.get_farmer_wrapped(&account_id) {
            farmer.get_ref_mut().amount += amount;
            self.data_mut().farmers.insert(&account_id, &farmer);
        } else {
            self.data_mut().farmers.insert(&account_id, &VersionedFarmer::new(amount));
            self.data_mut().farmer_count += 1;
        }
    }

}

