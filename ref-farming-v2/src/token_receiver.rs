use crate::*;
use crate::errors::*;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{serde_json, PromiseOrValue, Timestamp};
use near_sdk::json_types::{U128};
use crate::utils::{MFT_TAG, MAX_CDACCOUNT_NUM};
use crate::farm_seed::SeedType;


use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;

/// Message parameters to receive via token function call.
#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[serde(untagged)]
enum TokenReceiverMessage {
    CDAccount {
        index: u64,
        seed_id: SeedId,
        cd_strategy: usize,
    },
    Reward {
        farm_id: FarmId,
    }
}

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

            // if seed not exist, it will panic
            let seed_farm = self.get_seed(&env::predecessor_account_id());
            if amount < seed_farm.get_ref().min_deposit {
                env::panic(
                    format!(
                        "{} {}", 
                        ERR34_BELOW_MIN_SEED_DEPOSITED, 
                        seed_farm.get_ref().min_deposit
                    )
                    .as_bytes()
                )
            }

            self.internal_seed_deposit(
                &env::predecessor_account_id(), 
                &sender, 
                amount.into(), 
                amount.into(), 
                SeedType::FT,
                false
            );
            
            self.assert_storage_usage(&sender);

            env::log(
                format!(
                    "{} deposit FT seed {} with amount {}.",
                    sender, env::predecessor_account_id(), amount,
                )
                .as_bytes(),
            );
            PromiseOrValue::Value(U128(0))

        } else {  
            let message =
                serde_json::from_str::<TokenReceiverMessage>(&msg).expect(ERR51_WRONG_MSG_FORMAT);
            match message {
                TokenReceiverMessage::CDAccount {
                    index,
                    seed_id,
                    cd_strategy,
                } => {
                    // ****** create/append CD account ********
                    if let Some(seed_farm) = self.get_seed_wrapped(&seed_id) {
                        if amount < seed_farm.get_ref().min_deposit {
                            env::panic(
                                format!(
                                    "{} {}", 
                                    ERR34_BELOW_MIN_SEED_DEPOSITED, 
                                    seed_farm.get_ref().min_deposit
                                )
                                .as_bytes()
                            )
                        }

                        let mut seed_power = 0_u128;
                        let mut farmer = self.get_farmer(&sender);
                        let latest_index = farmer.get_ref().cd_accounts.len();
                        assert!(latest_index < MAX_CDACCOUNT_NUM, "{}", ERR61_CDACCOUNT_NUM_HAS_REACHED_LIMIT);
                        let is_create = index >= latest_index;

                        if is_create {
                            let cd_account = self.generate_cd_account(seed_id, cd_strategy, amount.into());
                            farmer.get_ref_mut().cd_accounts.push(&cd_account);
                            seed_power = cd_account.seed_power;
                            // farming_amount = farmer.get_ref_mut().create_cd_account(seed_id, cd_strategy, amount.into(), &self.data().cd_strategy);
                        } else {
                            let mut cd_account = farmer.get_ref().cd_accounts.get(index).unwrap();
                            self.append_cd_account(amount.into(), &mut cd_account);
                            farmer.get_ref_mut().cd_accounts.replace(index, &cd_account);
                            seed_power = cd_account.seed_power;
                            // farming_amount = farmer.get_ref_mut().append_cd_account(index, seed_id, cd_strategy, amount.into(), &self.data().cd_strategy);
                        }

                        self.data_mut().farmers.insert(&sender, &farmer);

                        self.internal_seed_deposit(
                            &env::predecessor_account_id(), 
                            &sender, 
                            amount,
                            seed_power, 
                            SeedType::FT,
                            true
                        );

                        env::log(
                            format!(
                                "{} {} CD account {} with amount {}. FT seed increase {}",
                                sender, 
                                if is_create { "create" } else { "append" }, 
                                if is_create { latest_index } else { index }, 
                                amount,
                                seed_power
                            )
                            .as_bytes(),
                        );

                        PromiseOrValue::Value(U128(0))
                    }else{
                        env::panic(format!("{}", ERR31_SEED_NOT_EXIST).as_bytes())
                    }
                },
                TokenReceiverMessage::Reward {
                    farm_id
                } => {
                    // ****** reward Token deposit in ********
                    // let farm_id = msg.parse::<FarmId>().expect(&format!("{}", ERR42_INVALID_FARM_ID));
                    let mut farm = self.data().farms.get(&farm_id).expect(ERR41_FARM_NOT_EXIST);

                    // update farm
                    assert_eq!(farm.get_reward_token(), env::predecessor_account_id(), "{}", ERR44_INVALID_FARM_REWARD);
                    if let Some(cur_remain) = farm.add_reward(&amount) {
                        self.data_mut().farms.insert(&farm_id, &farm);
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

/// a sub token would use a format ":<u64>"
fn try_identify_sub_token_id(token_id: &String) ->Result<u64, &'static str> {
    if token_id.starts_with(":") {
        if let Ok(pool_id) = str::parse::<u64>(&token_id[1..token_id.len()]) {
            Ok(pool_id)
        } else {
            Err("Illegal pool id")
        }
    } else {
        Err("Illegal pool id")
    }
}

fn parse_token_id(token_id: String) -> TokenOrPool {
    if let Ok(pool_id) = try_identify_sub_token_id(&token_id) {
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
        let amount: u128 = amount.into();
        if msg.is_empty() {
            let seed_id: String;
            match parse_token_id(token_id.clone()) {
                TokenOrPool::Pool(pool_id) => {
                    seed_id = format!("{}{}{}", env::predecessor_account_id(), MFT_TAG, pool_id);
                }
                TokenOrPool::Token(_) => {
                    // for seed deposit, using mft to transfer 'root' token is not supported.
                    env::panic(ERR35_ILLEGAL_TOKEN_ID.as_bytes());
                }
            }

            // if seed not exist, it will panic
            let seed_farm = self.get_seed(&seed_id);
            if amount < seed_farm.get_ref().min_deposit {
                env::panic(
                    format!(
                        "{} {}", 
                        ERR34_BELOW_MIN_SEED_DEPOSITED, 
                        seed_farm.get_ref().min_deposit
                    )
                    .as_bytes()
                )
            }
            
            self.internal_seed_deposit(&seed_id, &sender_id, amount, amount, SeedType::MFT, false);

            self.assert_storage_usage(&sender_id);

            env::log(
                format!(
                    "{} deposit MFT seed {} with amount {}.",
                    sender_id, seed_id, amount,
                )
                .as_bytes(),
            );

            PromiseOrValue::Value(U128(0))
        }else{
            let message =
                serde_json::from_str::<TokenReceiverMessage>(&msg).expect(ERR51_WRONG_MSG_FORMAT);
            match message {
                TokenReceiverMessage::CDAccount {
                    index,
                    seed_id,
                    cd_strategy,
                } => {
                    // ****** add/update CD account ********
                    let seed_farm = self.get_seed(&seed_id);
                    if amount < seed_farm.get_ref().min_deposit {
                        env::panic(
                            format!(
                                "{} {}", 
                                ERR34_BELOW_MIN_SEED_DEPOSITED, 
                                seed_farm.get_ref().min_deposit
                            )
                            .as_bytes()
                        )
                    }
                    
                    let mut seed_power = 0_u128;
                    let mut farmer = self.get_farmer(&sender_id);
                    let latest_index = farmer.get_ref().cd_accounts.len();
                    assert!(latest_index < MAX_CDACCOUNT_NUM, "{}", ERR61_CDACCOUNT_NUM_HAS_REACHED_LIMIT);
                    let is_create = index >= latest_index;

                    if is_create {
                        let cd_account = self.generate_cd_account(seed_id.clone(), cd_strategy, amount.into());
                        farmer.get_ref_mut().cd_accounts.push(&cd_account);
                        seed_power = cd_account.seed_power;
                        // farming_amount = farmer.get_ref_mut().create_cd_account(seed_id, cd_strategy, amount.into(), &self.data().cd_strategy);
                    } else {
                        let mut cd_account = farmer.get_ref().cd_accounts.get(index).unwrap();
                        self.append_cd_account(amount.into(), &mut cd_account);
                        farmer.get_ref_mut().cd_accounts.replace(index, &cd_account);
                        seed_power = cd_account.seed_power;
                        // farming_amount = farmer.get_ref_mut().append_cd_account(index, seed_id, cd_strategy, amount.into(), &self.data().cd_strategy);
                    }

                    self.data_mut().farmers.insert(&sender_id, &farmer);

                    self.internal_seed_deposit(
                        &seed_id, 
                        &sender_id, 
                        amount, 
                        seed_power, 
                        SeedType::MFT,
                        true
                    );

                    env::log(
                        format!(
                            "{} {} CD account {} with amount {}. MFT seed increase {}",
                            sender_id, 
                            if is_create { "create" } else { "append" }, 
                            if is_create { latest_index } else { index }, 
                            amount,
                            seed_power
                        )
                        .as_bytes(),
                    );

                    PromiseOrValue::Value(U128(0))
                },
                _ => {
                    // ****** not support other msg format ********
                    env::panic(format!("{}", ERR52_MSG_NOT_SUPPORT).as_bytes())
                }
            }
        }
    }
}
