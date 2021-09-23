use crate::*;

/// Implements users storage management for the pool.
#[near_bindgen]
impl StorageManagement for Contract {
    #[payable]
    fn storage_deposit(
        &mut self,
        account_id: Option<ValidAccountId>,
        registration_only: Option<bool>,
    ) -> StorageBalance {
        self.assert_contract_running();
        let amount = env::attached_deposit();
        let account_id = account_id
            .map(|a| a.into())
            .unwrap_or_else(|| env::predecessor_account_id());
        let registration_only = registration_only.unwrap_or(false);
        let min_balance = self.storage_balance_bounds().min.0;
        let already_registered = self.accounts.contains_key(&account_id);
        if amount < min_balance && !already_registered {
            env::panic(b"ERR_DEPOSIT_LESS_THAN_MIN_STORAGE");
        }
        if registration_only {
            // Registration only setups the account but doesn't leave space for tokens.
            if already_registered {
                log!("ERR_ACC_REGISTERED");
                if amount > 0 {
                    Promise::new(env::predecessor_account_id()).transfer(amount);
                }
            } else {
                self.internal_register_account(&account_id, min_balance);
                let refund = amount - min_balance;
                if refund > 0 {
                    Promise::new(env::predecessor_account_id()).transfer(refund);
                }
            }
        } else {
            self.internal_register_account(&account_id, amount);
        }
        self.storage_balance_of(account_id.try_into().unwrap())
            .unwrap()
    }

    #[payable]
    fn storage_withdraw(&mut self, amount: Option<U128>) -> StorageBalance {
        assert_one_yocto();
        self.assert_contract_running();
        let account_id = env::predecessor_account_id();
        let amount = amount.unwrap_or(U128(0)).0;
        let withdraw_amount = self.internal_storage_withdraw(&account_id, amount);
        Promise::new(account_id.clone()).transfer(withdraw_amount);
        self.storage_balance_of(account_id.try_into().unwrap())
            .unwrap()
    }

    #[allow(unused_variables)]
    #[payable]
    fn storage_unregister(&mut self, force: Option<bool>) -> bool {
        assert_one_yocto();
        self.assert_contract_running();
        let account_id = env::predecessor_account_id();
        if let Some(account_deposit) = self.internal_get_account(&account_id) {
            // TODO: figure out force option logic.
            assert!(
                account_deposit.tokens.is_empty(),
                "ERR_STORAGE_UNREGISTER_TOKENS_NOT_EMPTY"
            );
            self.accounts.remove(&account_id);
            Promise::new(account_id.clone()).transfer(account_deposit.near_amount);
            true
        } else {
            false
        }
    }

    fn storage_balance_bounds(&self) -> StorageBalanceBounds {
        StorageBalanceBounds {
            min: Account::min_storage_usage().into(),
            max: None,
        }
    }

    fn storage_balance_of(&self, account_id: ValidAccountId) -> Option<StorageBalance> {
        self.internal_get_account(account_id.as_ref())
            .map(|account| 
                { 
                    StorageBalance {
                        total: U128(account.near_amount),
                        available: U128(account.storage_available()),
                    } 
                })
    }
}
