use std::collections::HashMap;
use near_sdk::json_types::{ValidAccountId, U128};
use near_sdk::{assert_one_yocto, env, near_bindgen, 
    AccountId, Balance, Promise, PromiseResult};

use crate::utils::{ext_fungible_token, ext_self, GAS_FOR_FT_TRANSFER, GAS_FOR_RESOLVE_TRANSFER, parse_farm_id};
use crate::errors::*;
use crate::*;
use uint::construct_uint;

construct_uint! {
    /// 256-bit unsigned integer.
    pub struct U256(4);
}

#[near_bindgen]
impl Contract {

    /// Clean invalid rps,
    /// return false if the rps is still valid.
    pub fn remove_user_rps_by_farm(&mut self, farm_id: FarmId) -> bool {
        let sender_id = env::predecessor_account_id();
        let mut farmer = self.get_farmer(&sender_id);
        let (seed_id, _) = parse_farm_id(&farm_id);
        let farm_seed = self.get_seed(&seed_id);
        if !farm_seed.get_ref().farms.contains(&farm_id) {
            farmer.get_ref_mut().remove_rps(&farm_id);
            self.data_mut().farmers.insert(&sender_id, &farmer);
            true
        } else {
            false
        }
    }

    pub fn claim_reward_by_farm(&mut self, farm_id: FarmId) {
        let sender_id = env::predecessor_account_id();
        self.internal_claim_user_reward_by_farm_id(&sender_id, &farm_id);
        self.assert_storage_usage(&sender_id);
    }

    pub fn claim_reward_by_seed(&mut self, seed_id: SeedId) {
        let sender_id = env::predecessor_account_id();
        self.internal_claim_user_reward_by_seed_id(&sender_id, &seed_id);
        self.assert_storage_usage(&sender_id);
    }

    /// if withdraw_all_tokens is true, withdraw all tokens, 
    /// or just withdraw the reward token of this farm.
    pub fn claim_and_withdraw_by_farm(&mut self, farm_id: FarmId, withdraw_all_tokens: bool) {
        let sender_id = env::predecessor_account_id();
        self.claim_reward_by_farm(farm_id.clone());
        if withdraw_all_tokens {
            self.internal_withdraw_all_rewards(&sender_id, None);
        } else {
            if let Some(farm) = self.data().farms.get(&farm_id) {
                let token_id = farm.get_reward_token();
                self.internal_withdraw_all_rewards(&sender_id, Some(&token_id));
            }
        }
    }

    pub fn claim_and_withdraw_by_seed(&mut self, seed_id: SeedId) {
        let sender_id = env::predecessor_account_id();
        self.claim_reward_by_seed(seed_id.clone());
        self.internal_withdraw_all_rewards(&sender_id, None);
    }

    /// Withdraws given reward token of given user.
    #[payable]
    pub fn withdraw_reward(&mut self, token_id: ValidAccountId, amount: Option<U128>) -> Promise {
        assert_one_yocto();

        let token_id: AccountId = token_id.into();
        let amount: u128 = amount.unwrap_or(U128(0)).into(); 

        let sender_id = env::predecessor_account_id();

        let mut farmer = self.get_farmer(&sender_id);

        // Note: subtraction, will be reverted if the promise fails.
        let amount = farmer.get_ref_mut().sub_reward(&token_id, amount);
        self.data_mut().farmers.insert(&sender_id, &farmer);

        self.internal_send_tokens(&sender_id, &token_id, amount)

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
            PromiseResult::Successful(_) => {
                env::log(
                    format!(
                        "{} withdraw reward {} amount {}, Succeed.",
                        sender_id, token_id, amount.0,
                    )
                    .as_bytes(),
                );
            }
            PromiseResult::Failed => {
                env::log(
                    format!(
                        "{} withdraw reward {} amount {}, Callback Failed.",
                        sender_id, token_id, amount.0,
                    )
                    .as_bytes(),
                );
                // This reverts the changes from withdraw function.
                let mut farmer = self.get_farmer(&sender_id);
                farmer.get_ref_mut().add_reward(&token_id, amount.0);
                self.data_mut().farmers.insert(&sender_id, &farmer);
            }
        };
    }
}

fn claim_user_reward_from_farm(
    farm: &mut Farm, 
    farmer: &mut Farmer, 
    total_seeds: &Balance,
    silent: bool,
) {
    let user_seeds = farmer.seeds.get(&farm.get_seed_id()).unwrap_or(&0_u128);
    let user_rps = farmer.get_rps(&farm.get_farm_id());
    let (new_user_rps, reward_amount) = farm.claim_user_reward(&user_rps, user_seeds, total_seeds, silent);
    if !silent {
        env::log(
            format!(
                "user_rps@{} increased to {}",
                farm.get_farm_id(), U256::from_little_endian(&new_user_rps),
            )
            .as_bytes(),
        );
    }
        
    farmer.set_rps(&farm.get_farm_id(), new_user_rps);
    if reward_amount > 0 {
        farmer.add_reward(&farm.get_reward_token(), reward_amount);
        if !silent {
            env::log(
                format!(
                    "claimed {} {} as reward from {}",
                    reward_amount, farm.get_reward_token() , farm.get_farm_id(),
                )
                .as_bytes(),
            );
        }
    }
}

impl Contract {
    
    /// if token was given, then just withdraw that token, or withdraw all tokens
    pub(crate) fn internal_withdraw_all_rewards(&mut self, farmer_id: &AccountId, token: Option<&AccountId>) {
        
        let view_farmer = self.get_farmer(&farmer_id);
        let tokens = {
            if let Some(token) = token {
                vec![token]
            } else {
                view_farmer.get_ref().rewards.keys().collect::<Vec<_>>()
            }
        };
        
        let mut farmer = self.get_farmer(&farmer_id);
        let mut withdraw_tasks = HashMap::<AccountId, u128>::new();
        for token_id in tokens {
            let amount = farmer.get_ref_mut().sub_reward(&token_id, 0);
            withdraw_tasks.insert(token_id.clone(), amount);
        }
        self.data_mut().farmers.insert(farmer_id, &farmer);

        for (token_id, amount) in withdraw_tasks {
            self.internal_send_tokens(farmer_id, &token_id, amount);
        }
    }
    
    /// Sends given amount to given user and if it fails, returns it back to user's balance.
    /// Tokens must already be subtracted from internal balance.
    pub(crate) fn internal_send_tokens(
        &self,
        sender_id: &AccountId,
        token_id: &AccountId,
        amount: Balance,
    ) -> Promise {
        ext_fungible_token::ft_transfer(
            sender_id.clone(),
            U128(amount),
            None,
            token_id,
            1,
            GAS_FOR_FT_TRANSFER,
        )
        .then(ext_self::callback_post_withdraw_reward(
            token_id.clone(),
            sender_id.clone(),
            U128(amount),
            &env::current_account_id(),
            0,
            GAS_FOR_RESOLVE_TRANSFER,
        ))
    }

    pub(crate) fn internal_claim_user_reward_by_seed_id(
        &mut self, 
        sender_id: &AccountId,
        seed_id: &SeedId) {
        let mut farmer = self.get_farmer(sender_id);
        if let Some(mut farm_seed) = self.get_seed_wrapped(seed_id) {
            let amount = farm_seed.get_ref().amount;
            for farm_id in &mut farm_seed.get_ref_mut().farms.iter() {
                let mut farm = self.data().farms.get(farm_id).unwrap();
                claim_user_reward_from_farm(
                    &mut farm, 
                    farmer.get_ref_mut(),  
                    &amount,
                    true,
                );
                self.data_mut().farms.insert(farm_id, &farm);
            }
            self.data_mut().seeds.insert(seed_id, &farm_seed);
            self.data_mut().farmers.insert(sender_id, &farmer);
        }
    }

    pub(crate) fn internal_claim_user_reward_by_farm_id(
        &mut self, 
        sender_id: &AccountId, 
        farm_id: &FarmId) {
        let mut farmer = self.get_farmer(sender_id);

        let (seed_id, _) = parse_farm_id(farm_id);

        if let Some(farm_seed) = self.get_seed_wrapped(&seed_id) {
            let amount = farm_seed.get_ref().amount;
            if let Some(mut farm) = self.data().farms.get(farm_id) {
                claim_user_reward_from_farm(
                    &mut farm, 
                    farmer.get_ref_mut(), 
                    &amount,
                    false,
                );
                self.data_mut().farms.insert(farm_id, &farm);
                self.data_mut().farmers.insert(sender_id, &farmer);
            }
        }
    }


    #[inline]
    pub(crate) fn get_farmer(&self, from: &AccountId) -> VersionedFarmer {
        let orig = self.data().farmers
            .get(from)
            .expect(ERR10_ACC_NOT_REGISTERED);
        if orig.need_upgrade() {
                orig.upgrade()
            } else {
                orig
            }
    }

    #[inline]
    pub(crate) fn get_farmer_default(&self, from: &AccountId) -> VersionedFarmer {
        let orig = self.data().farmers.get(from).unwrap_or(VersionedFarmer::new(from.clone(), 0));
        if orig.need_upgrade() {
            orig.upgrade()
        } else {
            orig
        }
    }

    #[inline]
    pub(crate) fn get_farmer_wrapped(&self, from: &AccountId) -> Option<VersionedFarmer> {
        if let Some(farmer) = self.data().farmers.get(from) {
            if farmer.need_upgrade() {
                Some(farmer.upgrade())
            } else {
                Some(farmer)
            }
        } else {
            None
        }
    }

    /// Returns current balance of given token for given user. 
    /// If there is nothing recorded, returns 0.
    pub(crate) fn internal_get_reward(
        &self,
        sender_id: &AccountId,
        token_id: &AccountId,
    ) -> Balance {
        self.get_farmer_default(sender_id)
            .get_ref().rewards.get(token_id).cloned()
            .unwrap_or_default()
    }
}
