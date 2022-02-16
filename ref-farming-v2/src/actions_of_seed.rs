
use std::convert::TryInto;
use near_sdk::json_types::{U128};
use near_sdk::{AccountId, Balance, PromiseResult};

use crate::utils::{
    assert_one_yocto, ext_multi_fungible_token, ext_fungible_token, 
    ext_self, wrap_mft_token_id, parse_seed_id, GAS_FOR_FT_TRANSFER, GAS_FOR_RESOLVE_WITHDRAW_SEED
};
use crate::errors::*;
use crate::farm_seed::SeedType;
use crate::*;


#[near_bindgen]
impl Contract {

    #[payable]
    pub fn withdraw_seed(&mut self, seed_id: SeedId, amount: U128) {
        assert_one_yocto();
        let sender_id = env::predecessor_account_id();

        let amount: Balance = amount.into();

        // update inner state
        let seed_type = self.internal_seed_withdraw(&seed_id, &sender_id, amount);

        match seed_type {
            SeedType::FT => {
                ext_fungible_token::ft_transfer(
                    sender_id.clone().try_into().unwrap(),
                    amount.into(),
                    None,
                    &seed_id,
                    1,  // one yocto near
                    GAS_FOR_FT_TRANSFER,
                )
                .then(ext_self::callback_post_withdraw_ft_seed(
                    seed_id,
                    sender_id,
                    amount.into(),
                    &env::current_account_id(),
                    0,
                    GAS_FOR_RESOLVE_WITHDRAW_SEED,
                ));
            }
            SeedType::MFT => {
                let (receiver_id, token_id) = parse_seed_id(&seed_id);
                ext_multi_fungible_token::mft_transfer(
                    wrap_mft_token_id(&token_id),
                    sender_id.clone().try_into().unwrap(),
                    amount.into(),
                    None,
                    &receiver_id,
                    1,  // one yocto near
                    GAS_FOR_FT_TRANSFER,
                )
                .then(ext_self::callback_post_withdraw_mft_seed(
                    seed_id,
                    sender_id,
                    amount.into(),
                    &env::current_account_id(),
                    0,
                    GAS_FOR_RESOLVE_WITHDRAW_SEED,
                ));
            }
        }
        
    }

    #[private]
    pub fn callback_post_withdraw_ft_seed(
        &mut self,
        seed_id: SeedId,
        sender_id: AccountId,
        amount: U128,
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
    pub fn callback_post_withdraw_mft_seed(
        &mut self,
        seed_id: SeedId,
        sender_id: AccountId,
        amount: U128,
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


/// Internal methods implementation.
impl Contract {

    #[inline]
    pub(crate) fn get_seed(&self, seed_id: &String) -> VersionedFarmSeed {
        let orig = self.data().seeds.get(seed_id).expect(&format!("{}", ERR31_SEED_NOT_EXIST));
        if orig.need_upgrade() {
            orig.upgrade()
        } else {
            orig
        } 
    }

    #[inline]
    pub(crate) fn get_seed_wrapped(&self, seed_id: &String) -> Option<VersionedFarmSeed> {
        if let Some(farm_seed) = self.data().seeds.get(seed_id) {
            if farm_seed.need_upgrade() {
                Some(farm_seed.upgrade())
            } else {
                Some(farm_seed)
            }
        } else {
            None
        }
    }

    pub(crate) fn internal_seed_deposit(
        &mut self, 
        seed_id: &String, 
        sender_id: &AccountId, 
        seed_amount: Balance, 
        seed_power: Balance, 
        seed_type: SeedType,
        is_cd_account: bool) {

        // first claim all reward of the user for this seed farms 
        // to update user reward_per_seed in each farm 
        self.internal_claim_user_reward_by_seed_id(sender_id, seed_id);

        // **** update seed (new version)
        let mut farm_seed = self.get_seed(seed_id);
        farm_seed.get_ref_mut().seed_type = seed_type;
        farm_seed.get_ref_mut().add_seed_amount(seed_amount);
        farm_seed.get_ref_mut().add_seed_power(seed_power);
        self.data_mut().seeds.insert(&seed_id, &farm_seed);

        let mut farmer = self.get_farmer(sender_id);
        if !is_cd_account {
            farmer.get_ref_mut().add_seed_amount(&seed_id, seed_amount);
        }
        farmer.get_ref_mut().add_seed_power(&seed_id, seed_power);
        self.data_mut().farmers.insert(sender_id, &farmer);
    }

    fn internal_seed_withdraw(
        &mut self, 
        seed_id: &SeedId, 
        sender_id: &AccountId, 
        amount: Balance) -> SeedType {
        
        // first claim all reward of the user for this seed farms 
        // to update user reward_per_seed in each farm
        self.internal_claim_user_reward_by_seed_id(sender_id, seed_id);

        let mut farm_seed = self.get_seed(seed_id);
        let mut farmer = self.get_farmer(sender_id);

        // Then update user seed and total seed of this LPT
        let _farmer_seed_amount_remain = farmer.get_ref_mut().sub_seed_amount(seed_id, amount);
        let farmer_seed_power_remain = farmer.get_ref_mut().sub_seed_power(seed_id, amount);
        let _seed_amount_remain = farm_seed.get_ref_mut().sub_seed_amount(amount);
        let _seed_power_remain = farm_seed.get_ref_mut().sub_seed_power(amount);

        if farmer_seed_power_remain == 0{
            // remove farmer rps of relative farm
            for farm_id in farm_seed.get_ref().farms.iter() {
                farmer.get_ref_mut().remove_rps(farm_id);
            }
        }
        self.data_mut().farmers.insert(sender_id, &farmer);
        self.data_mut().seeds.insert(seed_id, &farm_seed);
        farm_seed.get_ref().seed_type.clone()
    }
}
