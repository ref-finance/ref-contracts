use std::convert::TryInto;
use near_sdk::json_types::{U128};
use near_sdk::{AccountId, Balance, PromiseResult};

use crate::utils::{
    assert_one_yocto, ext_multi_fungible_token, ext_fungible_token, 
    ext_self, wrap_mft_token_id, parse_seed_id, GAS_FOR_FT_TRANSFER, GAS_FOR_RESOLVE_WITHDRAW_SEED
};
use crate::errors::*;
use crate::farm_seed::SeedType;
use crate::farmer::CDAccount;
use crate::*;


#[near_bindgen]
impl Contract {
    #[payable]
    pub fn remove_cd_account(&mut self, index: u64) {
        assert_one_yocto();
        let sender_id = env::predecessor_account_id();
        // update inner state
        let (seed_type, cd_account, amount) = self.internal_remove_cd_account(&sender_id, index);

        match seed_type {
            SeedType::FT => {
                ext_fungible_token::ft_transfer(
                    sender_id.clone().try_into().unwrap(),
                    amount.into(),
                    None,
                    &cd_account.seed_id,
                    1,  // one yocto near
                    GAS_FOR_FT_TRANSFER,
                )
                .then(ext_self::callback_remove_cd_account_ft_seed(
                    cd_account.seed_id.clone(),
                    sender_id,
                    amount.into(),
                    cd_account.clone(),
                    &env::current_account_id(),
                    0,
                    GAS_FOR_RESOLVE_WITHDRAW_SEED,
                ));
            }
            SeedType::MFT => {
                let (receiver_id, token_id) = parse_seed_id(&cd_account.seed_id);
                ext_multi_fungible_token::mft_transfer(
                    wrap_mft_token_id(&token_id),
                    sender_id.clone().try_into().unwrap(),
                    amount.into(),
                    None,
                    &receiver_id,
                    1,  // one yocto near
                    GAS_FOR_FT_TRANSFER,
                )
                .then(ext_self::callback_remove_cd_account_mft_seed(
                    cd_account.seed_id.clone(),
                    sender_id,
                    amount.into(),
                    cd_account.clone(),
                    &env::current_account_id(),
                    0,
                    GAS_FOR_RESOLVE_WITHDRAW_SEED,
                ));
            }
        }
    }

    #[private]
    pub fn callback_remove_cd_account_ft_seed(
        &mut self,
        seed_id: SeedId,
        sender_id: AccountId,
        amount: U128,
        cd_account: CDAccount
    ) -> U128 {
        assert_eq!(
            env::promise_results_count(),
            1,
            "{}",
            ERR25_CALLBACK_POST_WITHDRAW_INVALID
        );
        let amount: Balance = amount.into();
        match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Failed => {
                env::log(
                    format!(
                        "{} withdraw {} ft seed with amount {}, Callback Failed.",
                        sender_id, seed_id, amount,
                    )
                    .as_bytes(),
                );
                // revert withdraw, equal to deposit, claim reward to update user reward_per_seed
                self.internal_claim_user_reward_by_seed_id(&sender_id, &seed_id);
                let mut farm_seed = self.get_seed(&seed_id);
                let mut farmer = self.get_farmer(&sender_id);

                farm_seed.get_ref_mut().seed_type = SeedType::FT;//TODO power
                farm_seed.get_ref_mut().add_seed_amount(amount);
                farm_seed.get_ref_mut().add_seed_power(amount);
                farmer.get_ref_mut().add_seed_amount(&seed_id, amount);
                farmer.get_ref_mut().add_seed_power(&seed_id, amount);
                farmer.get_ref_mut().cd_accounts.push(&cd_account);
                self.data_mut().seeds.insert(&seed_id, &farm_seed);
                self.data_mut().farmers.insert(&sender_id, &farmer);
                0.into()
            },
            PromiseResult::Successful(_) => {
                env::log(
                    format!(
                        "{} withdraw {} ft seed with amount {}, Succeed.",
                        sender_id, seed_id, amount,
                    )
                    .as_bytes(),
                );
                amount.into()
            }
        }
    }

    #[private]
    pub fn callback_remove_cd_account_mft_seed(
        &mut self,
        seed_id: SeedId,
        sender_id: AccountId,
        amount: U128,
        cd_account: CDAccount
    ) -> U128 {
        assert_eq!(
            env::promise_results_count(),
            1,
            "{}",
            ERR25_CALLBACK_POST_WITHDRAW_INVALID
        );
        let amount: Balance = amount.into();
        match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Failed => {
                env::log(
                    format!(
                        "{} withdraw {} mft seed with amount {}, Callback Failed.",
                        sender_id, seed_id, amount,
                    )
                    .as_bytes(),
                );
                // revert withdraw, equal to deposit, claim reward to update user reward_per_seed
                self.internal_claim_user_reward_by_seed_id(&sender_id, &seed_id);
                let mut farm_seed = self.get_seed(&seed_id);
                let mut farmer = self.get_farmer(&sender_id);

                farm_seed.get_ref_mut().seed_type = SeedType::MFT;
                farm_seed.get_ref_mut().add_seed_amount(amount); //TODO power
                farm_seed.get_ref_mut().add_seed_power(amount);
                farmer.get_ref_mut().add_seed_amount(&seed_id, amount);
                farmer.get_ref_mut().add_seed_power(&seed_id, amount);
                farmer.get_ref_mut().cd_accounts.push(&cd_account);
                self.data_mut().seeds.insert(&seed_id, &farm_seed);
                self.data_mut().farmers.insert(&sender_id, &farmer);
                0.into()
            },
            PromiseResult::Successful(_) => {
                env::log(
                    format!(
                        "{} withdraw {} mft seed with amount {}, Succeed.",
                        sender_id, seed_id, amount,
                    )
                    .as_bytes(),
                );
                amount.into()
            }
        }
    }
}