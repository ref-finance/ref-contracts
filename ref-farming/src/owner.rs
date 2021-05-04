use crate::*;

#[near_bindgen]
impl Contract {
    pub fn set_owner(&mut self, owner_id: ValidAccountId) {

        assert_eq!(
            &env::signer_account_id(),
            &self.owner_id,
            "Owner's method, only can be called by owner."
        );

        self.owner_id = owner_id.into();
    }
}