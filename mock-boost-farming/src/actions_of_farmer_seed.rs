use crate::*;
// use near_contract_standards::fungible_token::core::ext_ft_core;

#[near_bindgen]
impl Contract {
    /// convert free seed to locking mode
    #[payable]
    pub fn lock_free_seed(&mut self, seed_id: SeedId, duration_sec: u32, amount: Option<U128>) {
        assert_one_yocto();
        assert!(
            self.data().state == RunningState::Running,
            "{}", E004_CONTRACT_PAUSED
        );

        let farmer_id = env::predecessor_account_id();

        let mut farmer = self.internal_unwrap_farmer(&farmer_id);
        let mut seed = self.internal_unwrap_seed(&seed_id);

        assert!(seed.min_locking_duration_sec > 0, "{}", E300_FORBID_LOCKING);
        assert!(
            duration_sec >= seed.min_locking_duration_sec,
            "{}", E201_INVALID_DURATION
        );
        let config = self.internal_config();
        assert!(
            duration_sec <= config.maximum_locking_duration_sec,
            "{}", E201_INVALID_DURATION
        );

        self.internal_do_farmer_claim(&mut farmer, &mut seed);

        let mut farmer_seed = farmer.seeds.get(&seed_id).unwrap();
        let amount = if let Some(request) = amount {
            request.0
        } else {
            farmer_seed.free_amount
        };

        let increased_seed_power = farmer_seed.free_to_lock(amount, duration_sec, &config);
        farmer.seeds.insert(&seed_id, &farmer_seed);

        seed.total_seed_power += increased_seed_power;

        self.update_impacted_seeds(&mut farmer, &seed_id);

        self.internal_set_farmer(&farmer_id, farmer);
        self.internal_set_seed(&seed_id, seed);

        Event::SeedFreeToLock {
            farmer_id: &farmer_id,
            seed_id: &seed_id,
            amount: &U128(amount),
            increased_power: &U128(increased_seed_power),
            duration: duration_sec,
        }
        .emit();
    }

    // #[payable]
    // pub fn unlock_and_withdraw_seed(
    //     &mut self,
    //     seed_id: SeedId,
    //     unlock_amount: U128,
    //     withdraw_amount: U128,
    // ) -> PromiseOrValue<bool> {
    //     assert_one_yocto();
    //     assert!(
    //         self.data().state == RunningState::Running,
    //         E004_CONTRACT_PAUSED
    //     );

    //     let unlock_amount: Balance = unlock_amount.into();
    //     let withdraw_amount: Balance = withdraw_amount.into();

    //     let farmer_id = env::predecessor_account_id();

    //     let mut farmer = self.internal_unwrap_farmer(&farmer_id);
    //     let mut seed = self.internal_unwrap_seed(&seed_id);

    //     self.internal_do_farmer_claim(&mut farmer, &mut seed);

    //     let mut farmer_seed = farmer.seeds.get(&seed_id).unwrap();

    //     let prev = farmer_seed.get_seed_power();

    //     let decreased_seed_power = if unlock_amount > 0 {
    //         farmer_seed.unlock_to_free(unlock_amount)
    //     } else {
    //         0
    //     };
    //     let ret: PromiseOrValue<bool> = if withdraw_amount > 0 {
    //         farmer_seed.withdraw_free(withdraw_amount);
    //         self.transfer_seed_token(&farmer_id, &seed_id, withdraw_amount)
    //             .into()
    //     } else {
    //         PromiseOrValue::Value(true)
    //     };

    //     seed.total_seed_amount -= withdraw_amount;
    //     seed.total_seed_power = seed.total_seed_power - prev + farmer_seed.get_seed_power();

    //     if farmer_seed.is_empty() {
    //         farmer.seeds.remove(&seed_id);
    //     } else {
    //         farmer.seeds.insert(&seed_id, &farmer_seed);
    //     }

    //     self.update_impacted_seeds(&mut farmer, &seed_id);

    //     self.internal_set_farmer(&farmer_id, farmer);
    //     self.internal_set_seed(&seed_id, seed);

    //     if unlock_amount > 0 {
    //         Event::SeedUnlock {
    //             farmer_id: &farmer_id,
    //             seed_id: &seed_id,
    //             unlock_amount: &U128(unlock_amount),
    //             decreased_power: &U128(decreased_seed_power),
    //             slashed_seed: &U128(0),
    //         }
    //         .emit();
    //     }
    //     ret
    // }

    #[payable]
    pub fn force_unlock(&mut self, seed_id: SeedId, unlock_amount: U128) {
        assert_one_yocto();
        assert!(
            self.data().state == RunningState::Running,
            "{}", E004_CONTRACT_PAUSED
        );

        let unlock_amount: Balance = unlock_amount.into();

        let farmer_id = env::predecessor_account_id();

        let mut farmer = self.internal_unwrap_farmer(&farmer_id);
        let mut seed = self.internal_unwrap_seed(&seed_id);

        self.internal_do_farmer_claim(&mut farmer, &mut seed);

        let mut farmer_seed = farmer.seeds.get(&seed_id).unwrap();

        let (reduced_seed_power, seed_slashed) =
            farmer_seed.unlock_to_free_with_slashed(unlock_amount, seed.slash_rate);

        seed.total_seed_amount -= seed_slashed;
        seed.total_seed_power -= reduced_seed_power;

        let slashed_amount = self.data().seeds_slashed.get(&seed_id).unwrap_or(0);
        self.data_mut()
            .seeds_slashed
            .insert(&seed_id, &(slashed_amount + seed_slashed));

        farmer.seeds.insert(&seed_id, &farmer_seed);

        self.update_impacted_seeds(&mut farmer, &seed_id);

        self.internal_set_farmer(&farmer_id, farmer);
        self.internal_set_seed(&seed_id, seed);

        Event::SeedUnlock {
            farmer_id: &farmer_id,
            seed_id: &seed_id,
            unlock_amount: &U128(unlock_amount),
            decreased_power: &U128(reduced_seed_power),
            slashed_seed: &U128(seed_slashed),
        }
        .emit();
    }

    #[private]
    pub fn callback_withdraw_seed(&mut self, seed_id: SeedId, sender_id: AccountId, amount: U128) {
        assert!(
            env::promise_results_count() == 1,
            "{}", E001_PROMISE_RESULT_COUNT_INVALID
        );
        let amount: Balance = amount.into();
        match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Failed => {
                // all seed amount go to lostfound
                let seed_amount = self.data().seeds_lostfound.get(&seed_id).unwrap_or(0);
                self.data_mut()
                    .seeds_lostfound
                    .insert(&seed_id, &(seed_amount + amount));

                Event::SeedWithdraw {
                    farmer_id: &sender_id,
                    seed_id: &seed_id,
                    withdraw_amount: &U128(amount),
                    success: false,
                }
                .emit();
            }
            PromiseResult::Successful(_) => {
                Event::SeedWithdraw {
                    farmer_id: &sender_id,
                    seed_id: &seed_id,
                    withdraw_amount: &U128(amount),
                    success: true,
                }
                .emit();
            }
        }
    }
}

impl Contract {
    // fn transfer_seed_token(
    //     &mut self,
    //     farmer_id: &AccountId,
    //     seed_id: &SeedId,
    //     amount: Balance,
    // ) -> Promise {
    //     let (token, token_id) = parse_seed_id(seed_id);

    //     if let Some(token_id) = token_id {
    //         ext_multi_fungible_token::ext(token.clone())
    //             .with_attached_deposit(1)
    //             .with_static_gas(GAS_FOR_SEED_TRANSFER)
    //             .mft_transfer(
    //                 wrap_mft_token_id(&token_id),
    //                 farmer_id.clone(),
    //                 amount.into(),
    //                 None,
    //             )
    //             .then(
    //                 Self::ext(env::current_account_id())
    //                     .with_static_gas(GAS_FOR_RESOLVE_SEED_TRANSFER)
    //                     .callback_withdraw_seed(seed_id.clone(), farmer_id.clone(), amount.into()),
    //             )
    //     } else {
    //         ext_ft_core::ext(token.clone())
    //         .with_attached_deposit(1)
    //         .with_static_gas(GAS_FOR_SEED_TRANSFER)
    //         .ft_transfer(
    //             farmer_id.clone(),
    //             amount.into(),
    //             None,
    //         )
    //         .then(
    //             Self::ext(env::current_account_id())
    //                 .with_static_gas(GAS_FOR_RESOLVE_SEED_TRANSFER)
    //                 .callback_withdraw_seed(seed_id.clone(), farmer_id.clone(), amount.into()),
    //         )
    //     }
    // }
}
