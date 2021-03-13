use crate::*;

/// Implements users storage management for the pool.
/// TODO: This can be improve by keeping track NEAR balances per user, allowing to keep more than fixed number of tokens.
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
            env::panic(b"ERR_DEPSOIT_LESS_THAN_MIN_STORAGE");
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

    #[allow(unused_variables)]
    fn storage_withdraw(&mut self, amount: Option<U128>) -> StorageBalance {
        // TODO: implement
        unimplemented!()
    }

    #[allow(unused_variables)]
    fn storage_unregister(&mut self, force: Option<bool>) -> bool {
        // TODO: implement
        unimplemented!()
    }

    fn storage_balance_bounds(&self) -> StorageBalanceBounds {
        StorageBalanceBounds {
            min: (MIN_ACCOUNT_DEPOSIT_LENGTH * env::storage_byte_cost()).into(),
            max: None,
        }
    }

    fn storage_balance_of(&self, account_id: ValidAccountId) -> Option<StorageBalance> {
        let deposits = self
            .deposited_amounts
            .get(account_id.as_ref())
            .expect("ERR_NO_ACCOUNT");
        Some(StorageBalance {
            total: U128(deposits.amount),
            available: U128(deposits.amount - deposits.storage_usage()),
        })
    }
}
