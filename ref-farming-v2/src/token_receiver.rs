use crate::*;
use crate::errors::*;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{serde_json, PromiseOrValue};
use near_sdk::json_types::{U128};
use crate::utils::{MFT_TAG};
use crate::farm_seed::SeedType;


use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;

/// Message parameters to receive via token function call.
#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[serde(untagged)]
enum TokenReceiverMessage {
    NewCDAccount {
        index: u64,
        seed_id: SeedId,
        cd_strategy: usize,
    },
    AppendCDAccount {
        index: u64,
        seed_id: SeedId,
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
                TokenReceiverMessage::NewCDAccount {
                    index,
                    seed_id,
                    cd_strategy,
                } => {
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

                        let seed_power = self.generate_cd_account(&sender, seed_id, index, cd_strategy, amount.into());

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
                                "{} create CD account with seed amount {}, seed power {}",
                                sender, 
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
                TokenReceiverMessage::AppendCDAccount {
                    index,
                    seed_id,
                } => {
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

                        let seed_power = self.append_cd_account(&sender, seed_id, index, amount.into());

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
                                "{} append CD account {} with seed amount {}, seed power {}",
                                sender, 
                                index, 
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
                TokenReceiverMessage::NewCDAccount {
                    index,
                    seed_id,
                    cd_strategy,
                } => {
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
                    
                    let seed_power = self.generate_cd_account(&sender_id, seed_id.clone(), index, cd_strategy, amount.into());
                    
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
                            "{} create CD account with seed amount {}, seed power {}",
                            sender_id, 
                            amount,
                            seed_power
                        )
                        .as_bytes(),
                    );

                    PromiseOrValue::Value(U128(0))
                },
                TokenReceiverMessage::AppendCDAccount {
                    index,
                    seed_id,
                } => {
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
                    
                    let seed_power = self.append_cd_account(&sender_id, seed_id.clone(), index, amount.into());
                    
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
                            "{} append CD account {} with seed amount {}, seed power {}",
                            sender_id, 
                            index,
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
