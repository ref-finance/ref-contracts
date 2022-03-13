use crate::errors::*;
use crate::farm_seed::SeedType;
use crate::utils::MFT_TAG;
use crate::*;
use near_sdk::json_types::U128;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{serde_json, PromiseOrValue};

use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;

/// Message parameters to receive via token function call.
#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
enum TokenReceiverMessage {
    NewCDAccount { index: u32, cd_strategy: u32 },
    AppendCDAccount { index: u32 },
    Reward { farm_id: FarmId },
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
            self.internal_seed_deposit(
                &env::predecessor_account_id(),
                &sender,
                amount.into(),
                SeedType::FT,
            );
            PromiseOrValue::Value(U128(0))
        } else {
            let message =
                serde_json::from_str::<TokenReceiverMessage>(&msg).expect(ERR51_WRONG_MSG_FORMAT);
            match message {
                TokenReceiverMessage::NewCDAccount { index, cd_strategy } => {
                    let seed_id = env::predecessor_account_id();
                    self.internal_seed_deposit_to_new_cd_account(
                        &sender,
                        &seed_id,
                        index.into(),
                        cd_strategy as usize,
                        amount,
                        SeedType::FT,
                    );
                    PromiseOrValue::Value(U128(0))
                }
                TokenReceiverMessage::AppendCDAccount { index } => {
                    let seed_id = env::predecessor_account_id();
                    self.internal_seed_deposit_to_exist_cd_account(
                        &sender,
                        &seed_id,
                        index.into(),
                        amount,
                    );
                    PromiseOrValue::Value(U128(0))
                }
                TokenReceiverMessage::Reward { farm_id } => {
                    // ****** reward Token deposit in ********
                    let mut farm = self.data().farms.get(&farm_id).expect(ERR41_FARM_NOT_EXIST);

                    // update farm
                    assert_eq!(
                        farm.get_reward_token(),
                        env::predecessor_account_id(),
                        "{}",
                        ERR44_INVALID_FARM_REWARD
                    );
                    if let Some(cur_remain) = farm.add_reward(&amount) {
                        self.data_mut().farms.insert(&farm_id, &farm);
                        let old_balance = self
                            .data()
                            .reward_info
                            .get(&env::predecessor_account_id())
                            .unwrap_or(0);
                        self.data_mut()
                            .reward_info
                            .insert(&env::predecessor_account_id(), &(old_balance + amount));

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
fn try_identify_sub_token_id(token_id: &String) -> Result<u64, &'static str> {
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
        if msg.is_empty() {
            self.internal_seed_deposit(&seed_id, &sender_id, amount, SeedType::MFT);
            PromiseOrValue::Value(U128(0))
        } else {
            let message =
                serde_json::from_str::<TokenReceiverMessage>(&msg).expect(ERR51_WRONG_MSG_FORMAT);
            match message {
                TokenReceiverMessage::NewCDAccount { index, cd_strategy } => {
                    self.internal_seed_deposit_to_new_cd_account(
                        &sender_id,
                        &seed_id,
                        index.into(),
                        cd_strategy as usize,
                        amount,
                        SeedType::MFT,
                    );
                    PromiseOrValue::Value(U128(0))
                }
                TokenReceiverMessage::AppendCDAccount { index } => {
                    self.internal_seed_deposit_to_exist_cd_account(
                        &sender_id,
                        &seed_id,
                        index.into(),
                        amount,
                    );
                    PromiseOrValue::Value(U128(0))
                }
                _ => {
                    // ****** not support other msg format ********
                    env::panic(format!("{}", ERR52_MSG_NOT_SUPPORT).as_bytes())
                }
            }
        }
    }
}
