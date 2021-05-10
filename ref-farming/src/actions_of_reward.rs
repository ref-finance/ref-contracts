
use std::convert::TryInto;
use near_sdk::json_types::{ValidAccountId, U128};
use near_sdk::{assert_one_yocto, env, near_bindgen, AccountId, Balance, PromiseResult};

use crate::utils::{ext_fungible_token, ext_self, GAS_FOR_FT_TRANSFER, parse_farm_id};
use crate::errors::*;
use crate::*;
use uint::construct_uint;

construct_uint! {
    /// 256-bit unsigned integer.
    pub struct U256(4);
}

#[near_bindgen]
impl Contract {

    #[payable]
    pub fn claim_reward_by_farm(&mut self, farm_id: FarmId) {
        assert_one_yocto();
        let sender_id = env::predecessor_account_id();
        self.assert_storage_usage(&sender_id);
        self.internal_claim_user_reward_by_farm_id(&sender_id, &farm_id);
    }

    #[payable]
    pub fn claim_reward_by_seed(&mut self, seed_id: SeedId) {
        assert_one_yocto();
        let sender_id = env::predecessor_account_id();
        self.assert_storage_usage(&sender_id);
        self.internal_claim_user_reward_by_seed_id(&sender_id, &seed_id);
    }

    /// Withdraws given reward token of given user.
    #[payable]
    pub fn withdraw_reward(&mut self, token_id: ValidAccountId, amount: Option<U128>) {
        assert_one_yocto();

        let token_id: AccountId = token_id.into();
        let amount: u128 = amount.unwrap_or(U128(0)).into(); 

        let sender_id = env::predecessor_account_id();
        self.assert_storage_usage(&sender_id);

        let mut rewards = self.get_farmer(&sender_id);

        // Note: subtraction, will be reverted if the promise fails.
        let amount = rewards.sub_reward(&token_id, amount);
        self.farmers.insert(&sender_id, &rewards);
        ext_fungible_token::ft_transfer(
            sender_id.clone().try_into().unwrap(),
            amount.into(),
            None,
            &token_id,
            1,
            GAS_FOR_FT_TRANSFER,
        )
        .then(ext_self::callback_post_withdraw_reward(
            token_id,
            sender_id,
            amount.into(),
            &env::current_account_id(),
            0,
            GAS_FOR_FT_TRANSFER,
        ));
    }

    #[private]
    pub fn callback_post_withdraw_reward(
        &mut self,
        token_id: AccountId,
        sender_id: AccountId,
        amount: U128,
    ) {
        assert_eq!(
            env::promise_results_count(),
            1,
            "{}",
            ERR25_CALLBACK_POST_WITHDRAW_INVALID
        );
        match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Successful(_) => {}
            PromiseResult::Failed => {
                // This reverts the changes from withdraw function.
                let mut rewards = self.get_farmer(&sender_id);
                rewards.add_reward(&token_id, amount.0);
                self.farmers.insert(&sender_id, &rewards);
            }
        };
    }
}

fn claim_user_reward_from_farm(
    farm: &mut Farm, 
    farmer: &mut Farmer, 
    total_seeds: &Balance,
) -> bool {
    let user_seeds = farmer.seeds.get(&farm.get_seed_id()).unwrap_or(&0_u128);
    let user_rps = farmer.get_rps(&farm.get_farm_id());
    if let Some((new_user_rps, reward_amount)) = 
        farm.claim_user_reward(&user_rps, user_seeds, total_seeds) {
        env::log(
            format!(
                "user_rps@{} increased to {}",
                farm.get_farm_id(), U256::from_little_endian(&new_user_rps),
            )
            .as_bytes(),
        );

        farmer.set_rps(&farm.get_farm_id(), new_user_rps);
        if reward_amount > 0 {
            farmer.add_reward(&farm.get_reward_token(), reward_amount);
            env::log(
                format!(
                    "claimed {} {} as reward from {}",
                    reward_amount, farm.get_reward_token() , farm.get_farm_id(),
                )
                .as_bytes(),
            );
        }
        true
    } else {
        false
    }
}

impl Contract {

    pub(crate) fn internal_claim_user_reward_by_seed_id(
        &mut self, 
        sender_id: &AccountId,
        seed_id: &SeedId) {
        let mut farmer = self.get_farmer(sender_id);
        if let Some(mut farm_seed) = self.seeds.get(seed_id) {
            for farm in &mut farm_seed.farms {
                claim_user_reward_from_farm(
                    farm, 
                    &mut farmer,  
                    &farm_seed.amount);
            }
            
            self.seeds.insert(seed_id, &farm_seed);
            self.farmers.insert(sender_id, &farmer);
        }
    }

    pub(crate) fn internal_claim_user_reward_by_farm_id(
        &mut self, 
        sender_id: &AccountId, 
        farm_id: &FarmId) {
        let mut farmer = self.get_farmer(sender_id);

        let (seed_id, index) = parse_farm_id(farm_id);

        if let Some(mut farm_seed) = self.seeds.get(&seed_id) {
            if let Some(farm) = farm_seed.farms.get_mut(index) {
                claim_user_reward_from_farm(
                    farm, 
                    &mut farmer, 
                    &farm_seed.amount,
                );
                self.seeds.insert(&seed_id, &farm_seed);
                self.farmers.insert(sender_id, &farmer);
            }
        }
    }

    // Returns `from` AccountDeposit.
    #[inline]
    pub(crate) fn get_farmer(&self, from: &AccountId) -> Farmer {
        self.farmers
            .get(from)
            .expect(ERR10_ACC_NOT_REGISTERED)
    }

    /// Returns current balance of given token for given user. 
    /// If there is nothing recorded, returns 0.
    pub(crate) fn internal_get_reward(
        &self,
        sender_id: &AccountId,
        token_id: &AccountId,
    ) -> Balance {
        self.farmers
            .get(sender_id)
            .and_then(|d| d.rewards.get(token_id).cloned())
            .unwrap_or_default()
    }
}
