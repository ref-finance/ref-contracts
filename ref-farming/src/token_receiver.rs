use crate::*;
use crate::errors::*;
use near_sdk::PromiseOrValue;
use near_sdk::json_types::{U128};
use crate::utils::parse_farm_id;
use crate::utils::MFT_TAG;
use crate::farm_seed::SeedType;

use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;

#[near_bindgen]
impl FungibleTokenReceiver for Contract {
    /// Callback on receiving tokens by this contract.
    /// transfer reward token with specific msg indicate 
    /// which farm to be deposited to.
    fn ft_on_transfer(
        &mut self,
        sender_id: ValidAccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128> {

        let sender: AccountId = sender_id.into();
        let amount: u128 = amount.into();
        if msg.is_empty() {
            // ****** seed Token deposit in ********
            self.remove_unused_rps(&sender);
            self.internal_seed_deposit(
                &env::predecessor_account_id(), 
                &sender, 
                amount.into(), 
                SeedType::FT
            );
            PromiseOrValue::Value(U128(0))

        } else {  
            // ****** reward Token deposit in ********
            let farm_id = msg.parse::<FarmId>().expect(&format!("{}", ERR42_INVALID_FARM_ID));
            let (seed_id, _) = parse_farm_id(&farm_id);

            let mut farm_seed = self.get_seed(&seed_id);
            let farm = farm_seed.get_ref_mut().farms.get_mut(&farm_id).expect(&format!("{}", ERR41_FARM_NOT_EXIST));

            // update farm
            assert_eq!(
                farm.get_reward_token(), 
                env::predecessor_account_id(), 
                "{}", ERR44_INVALID_FARM_REWARD
            );
            if let Some(cur_remain) = farm.add_reward(&amount) {
                self.data_mut().seeds.insert(&seed_id, &farm_seed);
                let old_balance = self.data().reward_info.get(&env::predecessor_account_id()).unwrap_or(0);
                self.data_mut().reward_info.insert(&env::predecessor_account_id(), &(old_balance + amount));
                env::log(
                    format!(
                        "{} added {} Reward Token, Now has {} left",
                        sender, amount, cur_remain
                    )
                    .as_bytes(),
                );
                PromiseOrValue::Value(U128(0))
            } else {
                env::panic(format!("{}", ERR43_INVALID_FARM_STATUS).as_bytes())
            }
        }
        
    }
}

pub trait MFTTokenReceiver {
    fn mft_on_transfer(
        &mut self,
        token_id: String,
        sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128>;
}

enum TokenOrPool {
    Token(AccountId),
    Pool(u64),
}

fn parse_token_id(token_id: String) -> TokenOrPool {
    if let Ok(pool_id) = str::parse::<u64>(&token_id) {
        TokenOrPool::Pool(pool_id)
    } else {
        TokenOrPool::Token(token_id)
    }
}

/// seed token deposit
#[near_bindgen]
impl MFTTokenReceiver for Contract {
    /// Callback on receiving tokens by this contract.
    fn mft_on_transfer(
        &mut self,
        token_id: String,
        sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128> {

        self.assert_storage_usage(&sender_id);

        self.remove_unused_rps(&sender_id);
 
        let seed_id: String;
        match parse_token_id(token_id.clone()) {
            TokenOrPool::Pool(pool_id) => {
                seed_id = format!("{}{}{}", env::predecessor_account_id(), MFT_TAG, pool_id);
            }
            TokenOrPool::Token(token_id) => {
                seed_id = token_id;
            }
        }


        assert!(msg.is_empty(), "ERR_MSG_INCORRECT");
        
        self.internal_seed_deposit(&seed_id, &sender_id, amount.into(), SeedType::MFT);

        PromiseOrValue::Value(U128(0))
    }
}
