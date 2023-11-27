
use crate::*;
use near_sdk::{is_promise_success, Timestamp};
use crate::utils::ext_self;

pub const GAS_FOR_DEPOSIT_FREE_SHADOW_SEED: Gas = 50_000_000_000_000;
pub const GAS_FOR_DEPOSIT_FREE_SHADOW_SEED_CALLBACK: Gas = 20_000_000_000_000;

pub const GAS_FOR_WITHDRAW_FREE_SHADOW_SEED: Gas = 50_000_000_000_000;
pub const GAS_FOR_WITHDRAW_FREE_SHADOW_SEED_CALLBACK: Gas = 20_000_000_000_000;

pub const GAS_FOR_BURROWLAND_DEPOSIT_SHADOW_ASSET: Gas = 50_000_000_000_000;
pub const GAS_FOR_BURROWLAND_DEPOSIT_SHADOW_ASSET_CALLBACK: Gas = 20_000_000_000_000;

pub const GAS_FOR_BURROWLAND_WITHDRAW_SHADOW_ASSET: Gas = 50_000_000_000_000;
pub const GAS_FOR_BURROWLAND_WITHDRAW_SHADOW_ASSET_CALLBACK: Gas = 20_000_000_000_000;

#[ext_contract(ext_boost_farm_receiver)]
pub trait BoostFarmActions {
    fn deposit_free_shadow_seed(&mut self, farmer_id: AccountId, seed_id: String, amount: U128);
    fn withdraw_free_shadow_seed(&mut self, farmer_id: AccountId, seed_id: String, amount: U128);
}

#[ext_contract(ext_burrowland_receiver)]
pub trait BurrowlandActions {
    fn deposit_shadow_asset(&mut self, sender_id: AccountId, token_id: AccountId, amount: U128, after_deposit_actions_msg: Option<String>);
    fn withdraw_shadow_asset(&mut self, sender_id: AccountId, token_id: AccountId, amount: U128, before_withdraw_actions_msg: Option<String>);
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

pub fn pool_id_to_burrowland_token_id(pool_id: u64) -> String {
    format!("shadow_ref_v1-{}", pool_id)
}

#[near_bindgen]
impl Contract {

    pub fn sync_lp_infos(&self, pool_ids: Vec<u64>) -> HashMap<String, UnitShareTokens> {
        let mut result = HashMap::new();
        let current_timestamp = env::block_timestamp();
        for pool_id in pool_ids {
            let burrowland_token_id = pool_id_to_burrowland_token_id(pool_id);
            let mut pool = self.pools.get(pool_id).expect(ERR85_NO_POOL);
            let share_decimals = pool.get_share_decimal();
            let amounts: Vec<U128> = pool.remove_liquidity(&String::from("@view"), 10u128.pow(share_decimals as u32), vec![0; pool.tokens().len()], true).into_iter().map(|x| U128(x)).collect();
            let tokens = pool.tokens().iter().zip(amounts.into_iter()).map(|(token_id, amount)| TokenAmount { token_id: token_id.clone(), amount }).collect();
            result.insert(burrowland_token_id, UnitShareTokens{
                timestamp: current_timestamp,
                decimals: share_decimals,
                tokens
            });
        }
        result
    }

    #[payable]
    pub fn shadow_farming(&mut self, pool_id: u64, amount: Option<U128>) -> PromiseOrValue<bool> {
        self.assert_contract_running();
        let prev_storage = env::storage_usage();
        let sender_id = env::predecessor_account_id();
        let mut account = self.internal_unwrap_account(&sender_id);
        let pool = self.pools.get(pool_id).expect(ERR85_NO_POOL);

        let total_shares = pool.share_balances(&sender_id);
        let available_shares = if let Some(record) = account.get_shadow_record(pool_id) {
            record.available_farming_shares(total_shares)
        } else {
            total_shares
        };

        let amount = amount.unwrap_or(U128(available_shares)).0;
        assert!(amount > 0, "amount must be greater than zero");
        assert!(amount <= available_shares, "amount must be less than or equal to {}", available_shares);

        account.update_shadow_record(pool_id, ShadowActions::ToFarming, amount);
        self.internal_save_account(&sender_id, account);

        let storage_cost = self.internal_check_storage(prev_storage);

        let seed_id = format!("{}@{}", env::current_account_id(), pool_id);

        ext_boost_farm_receiver::deposit_free_shadow_seed(
            sender_id.clone(),
            seed_id,
            U128(amount),
            &self.boost_farm_id,
            0,
            GAS_FOR_DEPOSIT_FREE_SHADOW_SEED,
        )
        .then(ext_self::callback_deposit_free_shadow_seed(
            sender_id,
            pool_id,
            U128(amount),
            U128(storage_cost),
            &env::current_account_id(),
            0,
            GAS_FOR_DEPOSIT_FREE_SHADOW_SEED_CALLBACK,
        )).into()
    }

    pub fn shadow_cancel_farming(&mut self, pool_id: u64, amount: Option<U128>) -> PromiseOrValue<bool> {
        self.assert_contract_running();
        let prev_storage = env::storage_usage();
        let sender_id = env::predecessor_account_id();
        let mut account = self.internal_unwrap_account(&sender_id);

        let to_farming_shares = if let Some(record) = account.get_shadow_record(pool_id) {
            record.to_farming_amount
        } else {
            0
        };

        let amount = amount.unwrap_or(U128(to_farming_shares)).0;
        assert!(amount > 0, "amount must be greater than zero");
        assert!(amount <= to_farming_shares, "amount must be less than or equal to {}", to_farming_shares);

        account.update_shadow_record(pool_id, ShadowActions::FromFarming, amount);
        self.internal_save_account(&sender_id, account);

        let storage_refund = if prev_storage > env::storage_usage() {
           (prev_storage - env::storage_usage()) as Balance * env::storage_byte_cost()
        } else {
            0
        };

        let seed_id = format!("{}@{}", env::current_account_id(), pool_id);

        ext_boost_farm_receiver::withdraw_free_shadow_seed(
            sender_id.clone(),
            seed_id,
            U128(amount),
            &self.boost_farm_id,
            0,
            GAS_FOR_WITHDRAW_FREE_SHADOW_SEED,
        )
        .then(ext_self::callback_withdraw_free_shadow_seed(
            sender_id,
            pool_id,
            U128(amount),
            U128(storage_refund),
            &env::current_account_id(),
            0,
            GAS_FOR_WITHDRAW_FREE_SHADOW_SEED_CALLBACK,
        )).into()
    }

    #[payable]
    pub fn shadow_burrowland_deposit(&mut self, pool_id: u64, amount: Option<U128>, after_deposit_actions_msg: Option<String>) -> PromiseOrValue<bool> {
        self.assert_contract_running();
        let prev_storage = env::storage_usage();
        let sender_id = env::predecessor_account_id();
        let mut account = self.internal_unwrap_account(&sender_id);
        let pool = self.pools.get(pool_id).expect(ERR85_NO_POOL);
        self.assert_no_frozen_tokens(pool.tokens());

        let total_shares = pool.share_balances(&sender_id);
        let available_shares = if let Some(record) = account.get_shadow_record(pool_id) {
            record.available_burrowland_shares(total_shares)
        } else {
            total_shares
        };
        let amount = amount.unwrap_or(U128(available_shares)).0;

        assert!(amount > 0, "amount must be greater than zero");
        assert!(amount <= available_shares, "amount must be less than {}", available_shares);

        account.update_shadow_record(pool_id, ShadowActions::ToBurrowland, amount);
        self.internal_save_account(&sender_id, account);

        let storage_cost = self.internal_check_storage(prev_storage);
        let token_id = pool_id_to_burrowland_token_id(pool_id);

        ext_burrowland_receiver::deposit_shadow_asset(
            sender_id.clone(),
            token_id,
            U128(amount),
            after_deposit_actions_msg,
            &self.burrowland_id,
            0,
            GAS_FOR_BURROWLAND_DEPOSIT_SHADOW_ASSET,
        )
        .then(ext_self::callback_deposit_shadow_asset(
            sender_id,
            pool_id,
            U128(amount),
            U128(storage_cost),
            &env::current_account_id(),
            0,
            GAS_FOR_BURROWLAND_DEPOSIT_SHADOW_ASSET_CALLBACK,
        )).into()
    }

    pub fn shadow_burrowland_withdraw(&mut self, pool_id: u64, amount: Option<U128>, before_withdraw_actions_msg: Option<String>) -> PromiseOrValue<bool> {
        self.assert_contract_running();
        let prev_storage = env::storage_usage();
        let sender_id = env::predecessor_account_id();
        let mut account = self.internal_unwrap_account(&sender_id);

        let to_burrowland_shares = if let Some(record) = account.get_shadow_record(pool_id) {
            record.to_burrowland_amount
        } else {
            0
        };

        let amount = amount.unwrap_or(U128(to_burrowland_shares)).0;
        assert!(amount > 0, "amount must be greater than zero");
        assert!(amount <= to_burrowland_shares, "amount must be less than or equal to {}", to_burrowland_shares);

        account.update_shadow_record(pool_id, ShadowActions::FromBurrowland, amount);
        self.internal_save_account(&sender_id, account);

        let storage_refund = if prev_storage > env::storage_usage() {
            (prev_storage - env::storage_usage()) as Balance * env::storage_byte_cost()
        } else {
            0
        };

        let token_id = pool_id_to_burrowland_token_id(pool_id);

        ext_burrowland_receiver::withdraw_shadow_asset(
            sender_id.clone(),
            token_id,
            U128(amount),
            before_withdraw_actions_msg,
            &self.burrowland_id,
            0,
            GAS_FOR_BURROWLAND_WITHDRAW_SHADOW_ASSET,
        )
        .then(ext_self::callback_withdraw_shadow_asset(
            sender_id,
            pool_id,
            U128(amount),
            U128(storage_refund),
            &env::current_account_id(),
            0,
            GAS_FOR_BURROWLAND_WITHDRAW_SHADOW_ASSET_CALLBACK,
        )).into()
    }

    pub fn process_burrowland_liquidate_result(&mut self, sender_id: AccountId, liquidation_account_id: AccountId, pool_id: u64, liquidate_share_amount: U128, min_token_amounts: Vec<U128>) {
        assert!(self.burrowland_id == env::predecessor_account_id());
        let mut liquidation_account = self.internal_unwrap_account(&liquidation_account_id);
        liquidation_account.update_shadow_record(pool_id, ShadowActions::FromBurrowland, liquidate_share_amount.0);
        let mut pool = self.pools.get(pool_id).expect(ERR85_NO_POOL);
        self.assert_no_frozen_tokens(pool.tokens());

        let total_shares = pool.share_balances(&liquidation_account_id);
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

        let storage_refund = if withdraw_seed_amount > 0 {
            let prev_storage = env::storage_usage();
            liquidation_account.update_shadow_record(pool_id, ShadowActions::FromFarming, withdraw_seed_amount);
            if prev_storage > env::storage_usage() {
                (prev_storage - env::storage_usage()) as Balance * env::storage_byte_cost()
            } else {
                0
            }
        } else {
            0
        };

        let mut sender_account = self.internal_unwrap_account(&sender_id);
        
        let prev_storage = env::storage_usage();
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
            sender_account.deposit(&tokens[i], amounts[i]);
        }
        if prev_storage > env::storage_usage() {
            sender_account.near_amount +=
                (prev_storage - env::storage_usage()) as Balance * env::storage_byte_cost();
        }
        self.internal_save_account(&sender_id, sender_account);

        if withdraw_seed_amount > 0 {
            
            let seed_id = format!("{}@{}", env::current_account_id(), pool_id);

            ext_boost_farm_receiver::withdraw_free_shadow_seed(
                liquidation_account_id.clone(),
                seed_id,
                U128(withdraw_seed_amount),
                &self.boost_farm_id,
                0,
                GAS_FOR_WITHDRAW_FREE_SHADOW_SEED,
            )
            .then(ext_self::callback_withdraw_free_shadow_seed(
                liquidation_account_id.clone(),
                pool_id,
                U128(withdraw_seed_amount),
                U128(storage_refund),
                &env::current_account_id(),
                0,
                GAS_FOR_WITHDRAW_FREE_SHADOW_SEED_CALLBACK,
            ));
        }
        self.internal_save_account(&liquidation_account_id, liquidation_account);
    }

    pub fn process_burrowland_force_close_result(&mut self, liquidation_account_id: AccountId, pool_id: u64, liquidate_share_amount: U128, min_token_amounts: Vec<U128>) {
        assert!(self.burrowland_id == env::predecessor_account_id());
        let mut liquidation_account = self.internal_unwrap_account(&liquidation_account_id);
        liquidation_account.update_shadow_record(pool_id, ShadowActions::FromBurrowland, liquidate_share_amount.0);
        let mut pool = self.pools.get(pool_id).expect(ERR85_NO_POOL);
        self.assert_no_frozen_tokens(pool.tokens());

        let total_shares = pool.share_balances(&liquidation_account_id);
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

        let storage_refund = if withdraw_seed_amount > 0 {
            let prev_storage = env::storage_usage();
            liquidation_account.update_shadow_record(pool_id, ShadowActions::FromFarming, withdraw_seed_amount);
            if prev_storage > env::storage_usage() {
                (prev_storage - env::storage_usage()) as Balance * env::storage_byte_cost()
            } else {
                0
            }
        } else {
            0
        };

        let owner_id = self.owner_id.clone();
        let mut owner_account = self.internal_unwrap_account(&owner_id);
        let prev_storage = env::storage_usage();
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
            owner_account.deposit(&tokens[i], amounts[i]);
        }
        if prev_storage > env::storage_usage() {
            owner_account.near_amount +=
                (prev_storage - env::storage_usage()) as Balance * env::storage_byte_cost();
        }
        self.internal_save_account(&owner_id, owner_account);

        if withdraw_seed_amount > 0 {
            let seed_id = format!("{}@{}", env::current_account_id(), pool_id);

            ext_boost_farm_receiver::withdraw_free_shadow_seed(
                liquidation_account_id.clone(),
                seed_id,
                U128(withdraw_seed_amount),
                &self.boost_farm_id,
                0,
                GAS_FOR_WITHDRAW_FREE_SHADOW_SEED,
            )
            .then(ext_self::callback_withdraw_free_shadow_seed(
                liquidation_account_id.clone(),
                pool_id,
                U128(withdraw_seed_amount),
                U128(storage_refund),
                &env::current_account_id(),
                0,
                GAS_FOR_WITHDRAW_FREE_SHADOW_SEED_CALLBACK,
            ));
        }
        self.internal_save_account(&liquidation_account_id, liquidation_account);
    }
}

#[near_bindgen]
impl Contract {
    #[private]
    pub fn callback_deposit_free_shadow_seed(
        &mut self,
        sender_id: AccountId,
        pool_id: u64,
        amount: U128,
        storage_cost: U128
    ) -> bool {
        if !is_promise_success() {
            let mut account = self.internal_unwrap_account(&sender_id); 
            account.update_shadow_record(pool_id, ShadowActions::FromFarming, amount.0);
            self.internal_save_account(&sender_id, account);
            if storage_cost.0 > 0 {
                Promise::new(sender_id).transfer(storage_cost.0);
            }
            return false
        }
        true
    }

    #[private]
    pub fn callback_withdraw_free_shadow_seed(
        &mut self,
        sender_id: AccountId,
        pool_id: u64,
        amount: U128,
        storage_refund: U128
    ) -> bool {
        if !is_promise_success() {
            let mut account = self.internal_unwrap_account(&sender_id); 
            account.update_shadow_record(pool_id, ShadowActions::ToFarming, amount.0);
            self.internal_save_account(&sender_id, account);
            false
        } else {
            if storage_refund.0 > 0 {
                let mut account = self.internal_unwrap_account(&sender_id); 
                account.near_amount += storage_refund.0;
                self.internal_save_account(&sender_id, account);
            }
            true
        }
    }

    #[private]
    pub fn callback_deposit_shadow_asset(
        &mut self,
        sender_id: AccountId,
        pool_id: u64,
        amount: U128,
        storage_cost: U128
    ) -> bool {
        if !is_promise_success() {
            let mut account = self.internal_unwrap_account(&sender_id); 
            account.update_shadow_record(pool_id, ShadowActions::FromBurrowland, amount.0);
            self.internal_save_account(&sender_id, account);
            if storage_cost.0 > 0 {
                Promise::new(sender_id).transfer(storage_cost.0);
            }
            return false
        }
        true
    }

    #[private]
    pub fn callback_withdraw_shadow_asset(
        &mut self,
        sender_id: AccountId,
        pool_id: u64,
        amount: U128,
        storage_refund: U128
    ) -> bool {
        if !is_promise_success() {
            let mut account = self.internal_unwrap_account(&sender_id); 
            account.update_shadow_record(pool_id, ShadowActions::ToBurrowland, amount.0);
            self.internal_save_account(&sender_id, account);
            false
        } else {
            if storage_refund.0 > 0 {
                let mut account = self.internal_unwrap_account(&sender_id); 
                account.near_amount += storage_refund.0;
                self.internal_save_account(&sender_id, account);
            }
            true
        }
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