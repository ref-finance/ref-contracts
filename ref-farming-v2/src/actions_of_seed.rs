use near_sdk::json_types::U128;
use near_sdk::{AccountId, Balance, Promise};
use std::convert::TryInto;

use crate::errors::*;
use crate::farm_seed::SeedType;
use crate::utils::{
    assert_one_yocto, ext_fungible_token, ext_multi_fungible_token, ext_self, parse_seed_id,
    wrap_mft_token_id, GAS_FOR_FT_TRANSFER, GAS_FOR_RESOLVE_WITHDRAW_SEED, MAX_CDACCOUNT_NUM,
};
use crate::*;

#[near_bindgen]
impl Contract {
    #[payable]
    pub fn withdraw_seed(&mut self, seed_id: SeedId, amount: U128) -> Promise {
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
                    1, // one yocto near
                    GAS_FOR_FT_TRANSFER,
                )
                .then(ext_self::callback_withdraw_seed(
                    seed_id,
                    sender_id,
                    amount.into(),
                    &env::current_account_id(),
                    0,
                    GAS_FOR_RESOLVE_WITHDRAW_SEED,
                ))
            }
            SeedType::MFT => {
                let (receiver_id, token_id) = parse_seed_id(&seed_id);
                ext_multi_fungible_token::mft_transfer(
                    wrap_mft_token_id(&token_id),
                    sender_id.clone().try_into().unwrap(),
                    amount.into(),
                    None,
                    &receiver_id,
                    1, // one yocto near
                    GAS_FOR_FT_TRANSFER,
                )
                .then(ext_self::callback_withdraw_seed(
                    seed_id,
                    sender_id,
                    amount.into(),
                    &env::current_account_id(),
                    0,
                    GAS_FOR_RESOLVE_WITHDRAW_SEED,
                ))
            }
        }
    }

    #[payable]
    pub fn withdraw_seed_from_cd_account(&mut self, index: u64, amount: U128) -> Promise {
        assert_one_yocto();
        let sender_id = env::predecessor_account_id();
        // update inner state
        let (seed_id, amount) =
            self.internal_seed_withdraw_from_cd_account(&sender_id, index, amount.0);
        let (receiver_id, token_id) = parse_seed_id(&seed_id);
        if receiver_id == token_id {
            ext_fungible_token::ft_transfer(
                sender_id.clone().try_into().unwrap(),
                amount.into(),
                None,
                &seed_id,
                1, // one yocto near
                GAS_FOR_FT_TRANSFER,
            )
            .then(ext_self::callback_withdraw_seed(
                seed_id.clone(),
                sender_id,
                amount.into(),
                &env::current_account_id(),
                0,
                GAS_FOR_RESOLVE_WITHDRAW_SEED,
            ))
        } else {
            ext_multi_fungible_token::mft_transfer(
                wrap_mft_token_id(&token_id),
                sender_id.clone().try_into().unwrap(),
                amount.into(),
                None,
                &receiver_id,
                1, // one yocto near
                GAS_FOR_FT_TRANSFER,
            )
            .then(ext_self::callback_withdraw_seed(
                seed_id.clone(),
                sender_id,
                amount.into(),
                &env::current_account_id(),
                0,
                GAS_FOR_RESOLVE_WITHDRAW_SEED,
            ))
        }
    }
}

/// Internal methods implementation.
impl Contract {
    #[inline]
    pub(crate) fn get_seed(&self, seed_id: &String) -> VersionedFarmSeed {
        let orig = self
            .data()
            .seeds
            .get(seed_id)
            .expect(&format!("{}", ERR31_SEED_NOT_EXIST));
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
        seed_type: SeedType,
    ) {
        let mut farm_seed = self.get_seed(&seed_id);
        if seed_amount < farm_seed.get_ref().min_deposit {
            env::panic(
                format!(
                    "{} {}",
                    ERR34_BELOW_MIN_SEED_DEPOSITED,
                    farm_seed.get_ref().min_deposit
                )
                .as_bytes(),
            )
        }
        // 1. claim all reward of the user for this seed farms
        //    to update user reward_per_seed in each farm
        self.internal_claim_user_reward_by_seed_id(sender_id, seed_id);

        // 2. update farmer seed_power
        let mut farmer = self.get_farmer(sender_id);
        farmer.get_ref_mut().add_seed_amount(&seed_id, seed_amount);
        farmer.get_ref_mut().add_seed_power(&seed_id, seed_amount);
        self.data_mut().farmers.insert(sender_id, &farmer);

        // 3. update seed
        farm_seed.get_ref_mut().seed_type = seed_type;
        farm_seed.get_ref_mut().add_seed_amount(seed_amount);
        farm_seed.get_ref_mut().add_seed_power(seed_amount);
        self.data_mut().seeds.insert(&seed_id, &farm_seed);

        // 4. output log/event
        env::log(
            format!(
                "{} deposit seed {} with amount {}.",
                sender_id, seed_id, seed_amount,
            )
            .as_bytes(),
        );
    }

    pub(crate) fn internal_seed_deposit_to_new_cd_account(
        &mut self,
        sender: &AccountId,
        seed_id: &SeedId,
        index: u64,
        cd_strategy: usize,
        amount: Balance,
        seed_type: SeedType,
    ) {
        let mut farm_seed = self.get_seed(seed_id);
        if amount < farm_seed.get_ref().min_deposit {
            env::panic(
                format!(
                    "{} {}",
                    ERR34_BELOW_MIN_SEED_DEPOSITED,
                    farm_seed.get_ref().min_deposit
                )
                .as_bytes(),
            )
        }
        // 1. claim all reward of the user for this seed farms
        //    to update user reward_per_seed in each farm
        self.internal_claim_user_reward_by_seed_id(sender, seed_id);

        // 2. update CD Account and farmer seed_power
        let mut farmer = self.get_farmer(sender);
        assert!(
            index < MAX_CDACCOUNT_NUM,
            "{}",
            ERR63_INVALID_CD_ACCOUNT_INDEX
        );
        assert!(
            index <= farmer.get_ref().cd_accounts.len(),
            "{}",
            ERR63_INVALID_CD_ACCOUNT_INDEX
        );
        assert!(
            cd_strategy < STRATEGY_LIMIT,
            "{}",
            ERR62_INVALID_CD_STRATEGY_INDEX
        );
        let strategy = &self.data().cd_strategy.stake_strategy[cd_strategy];
        assert!(strategy.enable, "{}", ERR62_INVALID_CD_STRATEGY_INDEX);
        let mut cd_account = farmer.get_ref().cd_accounts.get(index).unwrap_or_default();
        let seed_power = cd_account.occupy(
            &seed_id,
            amount,
            strategy.power_reward_rate,
            strategy.lock_sec,
        );
        if index < farmer.get_ref().cd_accounts.len() {
            farmer.get_ref_mut().cd_accounts.replace(index, &cd_account);
        } else {
            farmer.get_ref_mut().cd_accounts.push(&cd_account);
        }
        farmer.get_ref_mut().add_seed_power(seed_id, seed_power);
        self.data_mut().farmers.insert(sender, &farmer);

        // 3. update seed
        farm_seed.get_ref_mut().seed_type = seed_type;
        farm_seed.get_ref_mut().add_seed_amount(amount);
        farm_seed.get_ref_mut().add_seed_power(seed_power);
        self.data_mut().seeds.insert(seed_id, &farm_seed);

        // 4. output log/event
        env::log(
            format!(
                "{} create CD account with seed amount {}, seed power {}",
                sender, amount, seed_power
            )
            .as_bytes(),
        );
    }

    pub(crate) fn internal_seed_deposit_to_exist_cd_account(
        &mut self,
        sender: &AccountId,
        seed_id: &SeedId,
        index: u64,
        amount: Balance,
    ) {
        let mut farm_seed = self.get_seed(&seed_id);
        if amount < farm_seed.get_ref().min_deposit {
            env::panic(
                format!(
                    "{} {}",
                    ERR34_BELOW_MIN_SEED_DEPOSITED,
                    farm_seed.get_ref().min_deposit
                )
                .as_bytes(),
            )
        }
        // 1. claim all reward of the user for this seed farms
        //    to update user reward_per_seed in each farm
        self.internal_claim_user_reward_by_seed_id(sender, seed_id);

        // 2. update CD Account and farmer seed_power
        let mut farmer = self.get_farmer(sender);
        assert!(
            index < farmer.get_ref().cd_accounts.len(),
            "{}",
            ERR63_INVALID_CD_ACCOUNT_INDEX
        );
        let mut cd_account = farmer.get_ref().cd_accounts.get(index).unwrap();
        let power_added = cd_account.append(seed_id, amount);
        farmer.get_ref_mut().cd_accounts.replace(index, &cd_account);
        farmer.get_ref_mut().add_seed_power(seed_id, power_added);
        self.data_mut().farmers.insert(sender, &farmer);

        // 3. update seed
        farm_seed.get_ref_mut().add_seed_amount(amount);
        farm_seed.get_ref_mut().add_seed_power(power_added);
        self.data_mut().seeds.insert(seed_id, &farm_seed);

        // 4. output log/event
        env::log(
            format!(
                "{} append CD account {} with seed amount {}, seed power {}",
                sender, index, amount, power_added
            )
            .as_bytes(),
        );
    }

    fn internal_seed_withdraw(
        &mut self,
        seed_id: &SeedId,
        sender_id: &AccountId,
        amount: Balance,
    ) -> SeedType {
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

        if farmer_seed_power_remain == 0 {
            // remove farmer rps of relative farm
            for farm_id in farm_seed.get_ref().farms.iter() {
                farmer.get_ref_mut().remove_rps(farm_id);
            }
        }
        self.data_mut().farmers.insert(sender_id, &farmer);
        self.data_mut().seeds.insert(seed_id, &farm_seed);
        farm_seed.get_ref().seed_type.clone()
    }

    fn internal_seed_withdraw_from_cd_account(
        &mut self,
        sender_id: &AccountId,
        index: u64,
        amount: Balance,
    ) -> (SeedId, Balance) {
        let farmer = self.get_farmer(sender_id);
        assert!(
            index < farmer.get_ref().cd_accounts.len(),
            "{}",
            ERR63_INVALID_CD_ACCOUNT_INDEX
        );
        let seed_id = &farmer.get_ref().cd_accounts.get(index).unwrap().seed_id;
        // 1. claim all reward of the user for this seed farms
        //    to update user reward_per_seed in each farm
        self.internal_claim_user_reward_by_seed_id(sender_id, seed_id);

        // 2. remove seed from cd account
        let mut farmer = self.get_farmer(sender_id);
        let mut cd_account = farmer.get_ref().cd_accounts.get(index).unwrap();
        let mut farm_seed = self.get_seed(seed_id);

        let (power_removed, seed_slashed) =
            cd_account.remove(seed_id, amount, farm_seed.get_ref().slash_rate);

        // 3. update user seed and total seed of this LPT
        let farmer_seed_power_remain = farmer.get_ref_mut().sub_seed_power(seed_id, power_removed);
        let _ = farm_seed.get_ref_mut().sub_seed_amount(amount);
        let _ = farm_seed.get_ref_mut().sub_seed_power(power_removed);

        // 4. collect seed_slashed
        if seed_slashed > 0 {
            env::log(
                format!(
                    "{} got slashed of {} seed with amount {}.",
                    sender_id, seed_id, seed_slashed,
                )
                .as_bytes(),
            );
            // all seed amount go to seed_slashed
            let seed_amount = self.data().seeds_slashed.get(&seed_id).unwrap_or(0);
            self.data_mut()
                .seeds_slashed
                .insert(&seed_id, &(seed_amount + seed_slashed));
        }

        // 5. remove user_rps if needed
        if farmer_seed_power_remain == 0 {
            // remove farmer rps of relative farm
            for farm_id in farm_seed.get_ref().farms.iter() {
                farmer.get_ref_mut().remove_rps(farm_id);
            }
        }

        // 6. save back to storage
        farmer.get_ref_mut().cd_accounts.replace(index, &cd_account);
        self.data_mut().farmers.insert(sender_id, &farmer);
        self.data_mut().seeds.insert(seed_id, &farm_seed);

        (seed_id.clone(), amount - seed_slashed)
    }
}
