use crate::*;

#[near_bindgen]
impl Contract {
    pub fn sudo_set_owner(&mut self, onwer_id: ValidAccountId) {

        assert_eq!(
            &env::signer_account_id(),
            &env::current_account_id(),
            "Sudoer's method, only can be called by myself."
        );

        self.owner_id = onwer_id.into();
    }
}