use crate::*;

#[near_bindgen]
impl Contract {
    /// The user donates the shares they hold to the protocol.
    ///
    /// # Arguments
    ///
    /// * `pool_id` - The pool id where the shares are located.
    /// * `amount` - The donation amount; if it's None, the entire amount will be donated.
    /// * `unregister` - If `Some(true)`, Will attempt to unregister the shares and refund the user’s storage fees.
    ///                  The storage fee will be refunded to the user's internal account first; 
    ///                  if there is no internal account, a transfer will be initiated. 
    #[payable]
    pub fn donation_share(&mut self, pool_id: u64, amount: Option<U128>, unregister: Option<bool>) {
        assert_one_yocto();
        self.assert_contract_running();
        let account_id = env::predecessor_account_id();
        let prev_storage = env::storage_usage();
        let mut pool = self.pools.get(pool_id).expect(ERR85_NO_POOL);
        let donation_amount = amount.map(|v| v.0).unwrap_or(pool.share_balances(&account_id));
        assert!(donation_amount > 0, "Invalid amount");
        pool.share_transfer(&account_id, &env::current_account_id(), donation_amount);
        if unregister == Some(true) {
            pool.share_unregister(&account_id);
        }
        self.pools.replace(pool_id, &pool);
        if prev_storage > env::storage_usage() {
            let refund = (prev_storage - env::storage_usage()) as Balance * env::storage_byte_cost();
            if let Some(mut account) = self.internal_get_account(&account_id) {
                account.near_amount += refund;
                self.internal_save_account(&account_id, account);
            } else {
                Promise::new(account_id.clone()).transfer(refund);
            }
        }
        event::Event::DonationShare { account_id: &account_id, pool_id, amount: U128(donation_amount) }.emit();
    }

    /// The user donates the tokens they hold to the owner.
    ///
    /// # Arguments
    ///
    /// * `token_id` - The id of the donated token.
    /// * `amount` - The donation amount; if it's None, the entire amount will be donated.
    /// * `unregister` - If `Some(true)`, Will attempt to unregister the tokens.
    #[payable]
    pub fn donation_token(&mut self, token_id: ValidAccountId, amount: Option<U128>, unregister: Option<bool>) {
        assert_one_yocto();
        self.assert_contract_running();
        let account_id = env::predecessor_account_id();
        let mut account = self.internal_unwrap_account(&account_id);
        let donation_amount = amount.map(|v| v.0).unwrap_or(account.get_balance(token_id.as_ref()).expect("Invalid token_id"));
        assert!(donation_amount > 0, "Invalid amount");
        account.withdraw(token_id.as_ref(), donation_amount);
        if unregister == Some(true) {
            account.unregister(token_id.as_ref());
        }
        self.internal_save_account(&account_id, account);
        let mut owner_account = self.internal_unwrap_account(&self.owner_id);
        owner_account.deposit(token_id.as_ref(), donation_amount);
        self.accounts.insert(&self.owner_id, &owner_account.into());
        event::Event::DonationToken { account_id: &account_id, token_id: token_id.as_ref(), amount: U128(donation_amount) }.emit();
    }
}
