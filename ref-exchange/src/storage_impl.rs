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
        let amount = env::attached_deposit();
        let account_id = account_id
            .map(|a| a.into())
            .unwrap_or_else(|| env::predecessor_account_id());
        let registration_only = registration_only.unwrap_or(false);
        let min_balance = self.storage_balance_bounds().min.0;
        if amount < min_balance {
            env::panic(b"ERR_DEPOSIT_LESS_THAN_MIN_STORAGE");
        }
        if registration_only {
            // Registration only setups the account but doesn't leave space for tokens.
            if self.deposited_amounts.contains_key(&account_id) {
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
        let account_id = env::predecessor_account_id();
        let account_deposit = self.get_account_deposits(&account_id);
        let available = account_deposit.storage_available();
        let amount = amount.map(|a| a.0).unwrap_or(available);
        assert!(amount <= available, "ERR_STORAGE_WITHDRAW_TOO_MUCH");
        Promise::new(account_id.clone()).transfer(amount);
        self.storage_balance_of(account_id.try_into().unwrap())
            .unwrap()
    }

    #[allow(unused_variables)]
    #[payable]
    fn storage_unregister(&mut self, force: Option<bool>) -> bool {
        assert_one_yocto();
        let account_id = env::predecessor_account_id();
        if let Some(account_deposit) = self.deposited_amounts.get(&account_id) {
            // TODO: figure out force option logic.
            assert!(
                account_deposit.tokens.is_empty(),
                "ERR_STORAGE_UNREGISTER_TOKENS_NOT_EMPTY"
            );
            self.deposited_amounts.remove(&account_id);
            Promise::new(account_id.clone()).transfer(account_deposit.amount);
            true
        } else {
            false
        }
    }

    fn storage_balance_bounds(&self) -> StorageBalanceBounds {
        StorageBalanceBounds {
            min: AccountDeposit::min_storage_usage().into(),
            max: None,
        }
    }

    fn storage_balance_of(&self, account_id: ValidAccountId) -> Option<StorageBalance> {
        self.deposited_amounts
            .get(account_id.as_ref())
            .map(|deposits| StorageBalance {
                total: U128(deposits.amount),
                available: U128(deposits.storage_available()),
            })
    }
}
