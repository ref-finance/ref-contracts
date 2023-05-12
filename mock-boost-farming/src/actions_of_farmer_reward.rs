use crate::*;
// use near_contract_standards::fungible_token::core::ext_ft_core;

#[near_bindgen]
impl Contract {
    pub fn claim_reward_by_seed(&mut self, seed_id: SeedId) {
        assert!(
            self.data().state == RunningState::Running,
            "{}", E004_CONTRACT_PAUSED
        );

        let farmer_id = env::predecessor_account_id();

        let mut farmer = self.internal_unwrap_farmer(&farmer_id);
        let mut seed = self.internal_unwrap_seed(&seed_id);

        self.internal_do_farmer_claim(&mut farmer, &mut seed);

        self.internal_set_seed(&seed_id, seed);
        self.internal_set_farmer(&farmer_id, farmer);
    }

    // /// Withdraws given reward token of given user.
    // /// when amount is None, withdraw all balance of the token.
    // pub fn withdraw_reward(
    //     &mut self,
    //     token_id: AccountId,
    //     amount: Option<U128>,
    // ) -> PromiseOrValue<bool> {
    //     assert!(
    //         self.data().state == RunningState::Running,
    //         E004_CONTRACT_PAUSED
    //     );

    //     let farmer_id = env::predecessor_account_id();
    //     let mut farmer = self.internal_unwrap_farmer(&farmer_id);

    //     let total = farmer.rewards.get(&token_id).unwrap_or(&0_u128);
    //     let amount: u128 = amount.map(|v| v.into()).unwrap_or(total.clone());

    //     if amount > 0 {
    //         // Note: subtraction, will be reverted if the promise fails.
    //         farmer.sub_reward(&token_id, amount);
    //         self.internal_set_farmer(&farmer_id, farmer);

    //         ext_ft_core::ext(token_id.clone())
    //             .with_attached_deposit(1)
    //             .with_static_gas(GAS_FOR_REWARD_TRANSFER)
    //             .ft_transfer(farmer_id.clone(), amount.into(), None)
    //             .then(
    //                 Self::ext(env::current_account_id())
    //                     .with_static_gas(GAS_FOR_RESOLVE_REWARD_TRANSFER)
    //                     .callback_post_withdraw_reward(
    //                         token_id.clone(),
    //                         farmer_id.clone(),
    //                         amount.into(),
    //                     ),
    //             )
    //             .into()
    //     } else {
    //         PromiseOrValue::Value(true)
    //     }
    // }

    #[private]
    pub fn callback_post_withdraw_reward(
        &mut self,
        token_id: AccountId,
        farmer_id: AccountId,
        amount: U128,
    ) {
        assert!(
            env::promise_results_count() == 1,
            "{}", E001_PROMISE_RESULT_COUNT_INVALID
        );
        let amount: Balance = amount.into();
        match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Successful(_) => {
                Event::RewardWithdraw {
                    farmer_id: &farmer_id,
                    token_id: &token_id,
                    withdraw_amount: &U128(amount),
                    success: true,
                }
                .emit();
            }
            PromiseResult::Failed => {
                // This reverts the changes from withdraw function.
                if let Some(mut farmer) = self.internal_get_farmer(&farmer_id) {
                    farmer.add_rewards(&HashMap::from([(token_id.clone(), amount)]));
                    self.internal_set_farmer(&farmer_id, farmer);

                    Event::RewardWithdraw {
                        farmer_id: &farmer_id,
                        token_id: &token_id,
                        withdraw_amount: &U128(amount),
                        success: false,
                    }
                    .emit();
                } else {
                    Event::RewardLostfound {
                        farmer_id: &farmer_id,
                        token_id: &token_id,
                        withdraw_amount: &U128(amount),
                    }
                    .emit();
                }
            }
        }
    }
}
