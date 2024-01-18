
use crate::*;
use near_sdk::{is_promise_success, Timestamp};
use crate::utils::ext_self;

pub const GAS_FOR_ON_CAST_SHADOW: Gas = 200_000_000_000_000;
pub const GAS_FOR_ON_CAST_SHADOW_CALLBACK: Gas = 20_000_000_000_000;
pub const GAS_FOR_ON_BURROW_LIQUIDATION: Gas = 40_000_000_000_000;
pub const GAS_FOR_ON_BURROW_LIQUIDATION_CALLBACK: Gas = 5_000_000_000_000;

#[ext_contract(ext_shadow_receiver)]
pub trait ShadowReceiver {
    fn on_cast_shadow(&mut self, account_id: AccountId, shadow_id: String, amount: U128, msg: String);
    fn on_remove_shadow(&mut self, account_id: AccountId, shadow_id: String, amount: U128, msg: String);
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct TokenAmount {
    pub token_id: AccountId,
    pub amount: U128,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct UnitShareTokens {
    #[serde(with = "u64_dec_format")]
    pub timestamp: Timestamp,
    pub decimals: u8,
    pub tokens: Vec<TokenAmount>,
}

pub fn pool_id_to_shadow_id(pool_id: u64) -> String {
    format!("shadow_ref_v1-{}", pool_id)
}

pub fn shadow_id_to_pool_id(shadow_id: &String) -> u64 {
    shadow_id.split("-").collect::<Vec<&str>>()[1].parse().expect("Invalid shadow_id")
}

#[near_bindgen]
impl Contract {

    pub fn get_unit_lpt_assets(&self, pool_ids: Vec<u64>) -> HashMap<String, UnitShareTokens> {
        let mut result = HashMap::new();
        let current_timestamp = env::block_timestamp();
        for pool_id in pool_ids {
            let shadow_id = pool_id_to_shadow_id(pool_id);
            if let Some(amounts) = self.get_unit_share_twap_token_amounts(pool_id) {
                let pool = self.pools.get(pool_id).expect(ERR85_NO_POOL);
                let share_decimals = pool.get_share_decimal();
                let tokens = pool.tokens().iter().zip(amounts.into_iter()).map(|(token_id, amount)| TokenAmount { token_id: token_id.clone(), amount }).collect();
                result.insert(shadow_id, UnitShareTokens{
                    timestamp: current_timestamp,
                    decimals: share_decimals,
                    tokens
                });
            }
        }
        result
    }

    #[payable]
    pub fn shadow_action(&mut self, action: ShadowActions, pool_id: u64, amount: Option<U128>, msg: String) -> PromiseOrValue<bool> {
        self.assert_contract_running();
        let shadow_id = pool_id_to_shadow_id(pool_id);
        let prev_storage = env::storage_usage();
        let sender_id = env::predecessor_account_id();
        let mut account = self.internal_unwrap_account(&sender_id);
        let pool = self.pools.get(pool_id).expect(ERR85_NO_POOL);
        let total_shares = pool.share_balances(&sender_id);
        let (amount, max_amount) = match action {
            ShadowActions::ToFarming => {
                let available_amount = if let Some(record) = account.get_shadow_record(pool_id) {
                    record.available_farming_shares(total_shares)
                } else {
                    total_shares
                };
                (amount.unwrap_or(U128(available_amount)).0, available_amount)
            }
            ShadowActions::ToBurrowland => {
                let available_amount = if let Some(record) = account.get_shadow_record(pool_id) {
                    record.available_burrowland_shares(total_shares)
                } else {
                    total_shares
                };
                (amount.unwrap_or(U128(available_amount)).0, available_amount)
            }
            ShadowActions::FromFarming => {
                let in_farming_amount = if let Some(record) = account.get_shadow_record(pool_id) {
                    record.shadow_in_farm
                } else {
                    0
                };
                (amount.unwrap_or(U128(in_farming_amount)).0, in_farming_amount)
            }
            ShadowActions::FromBurrowland => {
                let in_burrowland_amount = if let Some(record) = account.get_shadow_record(pool_id) {
                    record.shadow_in_burrow
                } else {
                    0
                };
                (amount.unwrap_or(U128(in_burrowland_amount)).0, in_burrowland_amount)
            }
        };
        assert!(amount > 0, "amount must be greater than zero");
        assert!(amount <= max_amount, "amount must be less than or equal to {}", max_amount);

        let contract_id = match action {
            ShadowActions::FromBurrowland | ShadowActions::ToBurrowland => {
                self.burrowland_id.clone()
            }
            ShadowActions::FromFarming | ShadowActions::ToFarming => {
                self.boost_farm_id.clone()
            }
        };

        match action {
            ShadowActions::ToFarming | ShadowActions::ToBurrowland => {
                account.update_shadow_record(pool_id, &action, amount);
                self.internal_save_account(&sender_id, account);
                let storage_fee = self.internal_check_storage(prev_storage);
                ext_shadow_receiver::on_cast_shadow(
                        sender_id.clone(),
                        shadow_id,
                        U128(amount),
                        msg,
                        &contract_id,
                        0,
                        GAS_FOR_ON_CAST_SHADOW
                    )
                    .then(ext_self::callback_on_shadow(
                            action,
                            sender_id,
                            pool_id,
                            U128(amount),
                            U128(storage_fee),
                            &env::current_account_id(),
                            0,
                            GAS_FOR_ON_CAST_SHADOW_CALLBACK
                        )
                    )
                    .into()
            }
            ShadowActions::FromFarming | ShadowActions::FromBurrowland => {
                ext_shadow_receiver::on_remove_shadow(
                        sender_id.clone(),
                        shadow_id,
                        U128(amount),
                        msg,
                        &contract_id,
                        0,
                        GAS_FOR_ON_CAST_SHADOW
                    )
                    .then(ext_self::callback_on_shadow(
                            action,
                            sender_id,
                            pool_id,
                            U128(amount),
                            U128(0),
                            &env::current_account_id(),
                            0,
                            GAS_FOR_ON_CAST_SHADOW_CALLBACK
                        )
                    )
                    .into()
            }
        }
    }

    pub fn on_burrow_liquidation(&mut self, liquidator_account_id: AccountId, liquidation_account_id: AccountId, shadow_id: String, liquidate_share_amount: U128, min_token_amounts: Vec<U128>) {
        assert!(self.burrowland_id == env::predecessor_account_id());
        let pool_id = shadow_id_to_pool_id(&shadow_id);
        
        let mut pool = self.pools.get(pool_id).expect(ERR85_NO_POOL);
        self.assert_no_frozen_tokens(pool.tokens());
        let total_shares = pool.share_balances(&liquidation_account_id);

        let mut liquidation_account = self.internal_unwrap_account(&liquidation_account_id);
        let prev_storage = env::storage_usage();
        liquidation_account.update_shadow_record(pool_id, &ShadowActions::FromBurrowland, liquidate_share_amount.0);
        let available_shares = if let Some(record) = liquidation_account.get_shadow_record(pool_id) {
            record.free_shares(total_shares)
        } else {
            total_shares
        };
        let withdraw_seed_amount = if available_shares > 0 {
            if available_shares > liquidate_share_amount.0 { 0 } else { liquidate_share_amount.0 - available_shares }
        } else {
            liquidate_share_amount.0
        };
        if withdraw_seed_amount > 0 {
            liquidation_account.update_shadow_record(pool_id, &ShadowActions::FromFarming, withdraw_seed_amount);
        }
        if prev_storage > env::storage_usage() {
            liquidation_account.near_amount +=
                (prev_storage - env::storage_usage()) as Balance * env::storage_byte_cost()
        }
        self.internal_save_account(&liquidation_account_id, liquidation_account);
        
        let mut liquidator_account = self.internal_unwrap_account(&liquidator_account_id);
        let amounts = pool.remove_liquidity(
            &liquidation_account_id,
            liquidate_share_amount.0,
            min_token_amounts
                .into_iter()
                .map(|amount| amount.into())
                .collect(),
            false
        );
        self.pools.replace(pool_id, &pool);
        let tokens = pool.tokens();
        for i in 0..tokens.len() {
            liquidator_account.deposit(&tokens[i], amounts[i]);
        }
        self.internal_save_account(&liquidator_account_id, liquidator_account);

        if withdraw_seed_amount > 0 {
            ext_shadow_receiver::on_remove_shadow(
                liquidation_account_id.clone(),
                shadow_id,
                U128(withdraw_seed_amount),
                "".to_string(),
                &self.boost_farm_id,
                0,
                GAS_FOR_ON_BURROW_LIQUIDATION
            )
            .then(ext_self::callback_on_burrow_liquidation(
                    liquidation_account_id,
                    pool_id,
                    U128(withdraw_seed_amount),
                    &env::current_account_id(),
                    0,
                    GAS_FOR_ON_BURROW_LIQUIDATION_CALLBACK
                )
            );
        }
    }
}

#[near_bindgen]
impl Contract {
    #[private]
    pub fn callback_on_shadow(
        &mut self,
        action: ShadowActions,
        sender_id: AccountId,
        pool_id: u64,
        amount: U128,
        storage_fee: U128
    ) -> bool {
        if !is_promise_success() {
            let mut account = self.internal_unwrap_account(&sender_id); 
            match action {
                ShadowActions::ToFarming => {
                    account.update_shadow_record(pool_id, &ShadowActions::FromFarming, amount.0);
                    if storage_fee.0 > 0 {
                        Promise::new(sender_id.clone()).transfer(storage_fee.0);
                    }
                }
                ShadowActions::ToBurrowland => {
                    account.update_shadow_record(pool_id, &ShadowActions::FromBurrowland, amount.0);
                    if storage_fee.0 > 0 {
                        Promise::new(sender_id.clone()).transfer(storage_fee.0);
                    }
                }
                _ => {}
            }
            self.internal_save_account(&sender_id, account);
            false
        } else {
            match action {
                ShadowActions::FromFarming | ShadowActions::FromBurrowland => {
                    let prev_storage = env::storage_usage();
                    let mut account = self.internal_unwrap_account(&sender_id); 
                    account.update_shadow_record(pool_id, &action, amount.0);
                    if prev_storage > env::storage_usage() {
                        account.near_amount += (prev_storage - env::storage_usage()) as Balance * env::storage_byte_cost()
                    }
                    self.internal_save_account(&sender_id, account);
                }
                _ => {}
            }
            true
        }
    }

    #[private]
    pub fn callback_on_burrow_liquidation(
        &mut self,
        sender_id: AccountId,
        pool_id: u64,
        amount: U128,
    ) {
        log!("pool_id {}, {} remove {} farming seed {}", pool_id, sender_id, amount.0, 
            if is_promise_success() { "successful" } else { "failed" });
    }
}

pub mod u64_dec_format {
    use near_sdk::serde::de;
    use near_sdk::serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(num: &u64, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&num.to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<u64, D::Error>
    where
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .parse()
            .map_err(de::Error::custom)
    }
}