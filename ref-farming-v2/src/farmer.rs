//! Farmer records a farmer's 
//! * all claimed reward tokens, 
//! * all seeds he staked,
//! * user_rps per farm,
//! and the deposited near amount prepaid as storage fee


use std::collections::HashMap;
use near_sdk::collections::{LookupMap, Vector};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{env, AccountId, Balance, Timestamp};
use near_sdk::serde::{Deserialize, Serialize};
use crate::{SeedId, FarmId, RPS};
use crate::farm_seed::SeedType;
use crate::*;
use crate::errors::*;
use crate::utils::{MAX_ACCOUNT_LENGTH, U256};
use crate::StorageKeys;
/// each entry cost MAX_ACCOUNT_LENGTH bytes, 
/// amount: Balance cost 16 bytes
/// each empty hashmap cost 4 bytes
pub const MIN_FARMER_LENGTH: u128 = MAX_ACCOUNT_LENGTH + 16 + 4 * 3;

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct CDAccount {
    pub seed_id: SeedId,
    /// Index of CDStrategy member(locking_time、additional)
    /// Affected only when CDAccount created
    pub cd_strategy: usize,
    /// From ft_on_transfer、ft_on_transfer amount
    pub seed_amount: Balance,
    /// self.seed_amount * CDStrategy.additional[self.cd_strategy] / CDStrategy.denominator
    pub seed_power: Balance,
    /// seed stake begin timestamp: env::block_timestamp()
    pub begin_sec: Timestamp,
    /// seed stake end timestamp: self.begin_sec + CDStrategy.locking_time[self.cd_strategy]
    pub end_sec: Timestamp
}

impl Contract {
    pub(crate) fn generate_cd_account(&self, seed_id: SeedId, cd_strategy: usize, amount: Balance) -> CDAccount {
        let strategy = &self.data().cd_strategy;
        assert!(cd_strategy < strategy.locking_time.len() && cd_strategy < strategy.additional.len(), "{}", ERR62_INVALID_CD_STRATEGY_INDEX);
        let now = env::block_timestamp();
        let power = (U256::from(amount) * U256::from(strategy.additional[cd_strategy]) / U256::from(strategy.denominator)).as_u128();
        CDAccount{
            seed_id,
            cd_strategy,
            seed_amount: amount,
            seed_power: amount + power,
            begin_sec: now,
            end_sec: now + strategy.locking_time[cd_strategy]
        }
    }

    pub(crate) fn append_cd_account(&self, amount: Balance, cd_account: &mut CDAccount) {
        let strategy = &self.data().cd_strategy;
        let total_power = U256::from(amount) * U256::from(strategy.additional[cd_account.cd_strategy]) / U256::from(strategy.denominator);
        let locking_time = cd_account.end_sec - cd_account.begin_sec;
        let passing_time = env::block_timestamp() - cd_account.begin_sec;
        assert!(locking_time > passing_time, "{}", ERR64_EXPIRED_CD_ACCOUNT);
        let residue_time = locking_time - passing_time;
        let residue_power = (total_power * U256::from(residue_time) / U256::from(locking_time)).as_u128();
        cd_account.seed_amount += amount;
        cd_account.seed_power += amount + residue_power;
    }

    pub(crate) fn internal_remove_cd_account(&mut self, sender_id: &AccountId, index: u64) -> (SeedType, CDAccount, Balance)  {
        let farmer = self.get_farmer(sender_id);
        let cd_accounts = &farmer.get_ref().cd_accounts;
        assert!(cd_accounts.len() > index, "{}", ERR63_INVALID_CD_ACCOUNT_INDEX);
        let cd_account = cd_accounts.get(index).unwrap();
        let seed_id = &cd_account.seed_id;

        // first claim all reward of the user for this seed farms 
        // to update user reward_per_seed in each farm
        self.internal_claim_user_reward_by_seed_id(sender_id, seed_id);

        let mut farmer = self.get_farmer(sender_id);
        let mut farm_seed = self.get_seed(seed_id);

        // Then update user seed and total seed of this LPT
        let farmer_seed_power_remain = farmer.get_ref_mut().sub_seed_power(seed_id, cd_account.seed_power);
        let _seed_amount_remain = farm_seed.get_ref_mut().sub_seed_amount(cd_account.seed_amount);
        let _seed_power_remain = farm_seed.get_ref_mut().sub_seed_power(cd_account.seed_power);
        farmer.get_ref_mut().cd_accounts.swap_remove(index);

        if farmer_seed_power_remain == 0 {
            // remove farmer rps of relative farm
            for farm_id in farm_seed.get_ref().farms.iter() {
                farmer.get_ref_mut().remove_rps(farm_id);
            }
        }

        self.data_mut().farmers.insert(sender_id, &farmer);
        self.data_mut().seeds.insert(seed_id, &farm_seed);

        let strategy = &self.data().cd_strategy;
        let liquidated_damages = if env::block_timestamp() >= cd_account.end_sec {
            0_u128
        } else {
            let total = U256::from(cd_account.seed_amount) * U256::from(strategy.damage) / U256::from(strategy.denominator);
            let locking_time = cd_account.end_sec - cd_account.begin_sec;
            let passing_time = env::block_timestamp() - cd_account.begin_sec;
            let residue_time = locking_time - passing_time;
            (total * U256::from(residue_time) / U256::from(locking_time)).as_u128()
        };
        let withdraw_seed = cd_account.seed_amount - liquidated_damages;
        (farm_seed.get_ref().seed_type.clone(), cd_account.clone(), withdraw_seed)
    }
}

/// Account deposits information and storage cost.
#[derive(BorshSerialize, BorshDeserialize)]
#[cfg_attr(feature = "test", derive(Clone))]
pub struct Farmer {
    /// Native NEAR amount sent to this contract.
    /// Used for storage.
    pub amount: Balance,
    /// Amounts of various reward tokens the farmer claimed.
    pub rewards: HashMap<AccountId, Balance>,
    /// Amounts of various seed tokens the farmer staked.
    pub seed_amounts: HashMap<SeedId, Balance>,
    /// Powers of various seed tokens the farmer staked.
    pub seed_powers: HashMap<SeedId, Balance>,
    /// Record user_last_rps of farms
    pub user_rps: LookupMap<FarmId, RPS>,
    pub rps_count: u32,
    /// Farmer can create up to 16 CD accounts
    pub cd_accounts: Vector<CDAccount>,
}

impl Farmer {

    /// Adds amount to the balance of given token
    pub(crate) fn add_reward(&mut self, token: &AccountId, amount: Balance) {
        if let Some(x) = self.rewards.get_mut(token) {
            *x = *x + amount;
        } else {
            self.rewards.insert(token.clone(), amount);
        }
    }

    /// Subtract from `reward` balance.
    /// if amount == 0, subtract all reward balance.
    /// Panics if `amount` is bigger than the current balance.
    /// return actual subtract amount
    pub(crate) fn sub_reward(&mut self, token: &AccountId, amount: Balance) -> Balance {
        let value = *self.rewards.get(token).expect(ERR21_TOKEN_NOT_REG);
        assert!(value >= amount, "{}", ERR22_NOT_ENOUGH_TOKENS);
        if amount == 0 {
            self.rewards.remove(&token.clone());
            value
        } else {
            self.rewards.insert(token.clone(), value - amount);
            amount
        }
    }

    pub fn add_seed_amount(&mut self, seed_id: &SeedId, amount: Balance) {
        if amount > 0 {
            self.seed_amounts.insert(
                seed_id.clone(), 
                amount + self.seed_amounts.get(seed_id).unwrap_or(&0_u128)
            );
        }
        
    }

    /// return seed remained.
    pub fn sub_seed_amount(&mut self, seed_id: &SeedId, amount: Balance) -> Balance {
        let prev_balance = self.seed_amounts.get(seed_id).expect(&format!("{}", ERR31_SEED_NOT_EXIST));
        assert!(prev_balance >= &amount, "{}", ERR32_NOT_ENOUGH_SEED);
        let cur_balance = prev_balance - amount;
        if cur_balance > 0 {
            self.seed_amounts.insert(seed_id.clone(), cur_balance);
        } else {
            self.seed_amounts.remove(seed_id);
        }
        cur_balance
    }

    pub fn add_seed_power(&mut self, seed_id: &SeedId, amount: Balance) {
        if amount > 0 {
            self.seed_powers.insert(
                seed_id.clone(), 
                amount + self.seed_powers.get(seed_id).unwrap_or(&0_u128)
            );
        }
        
    }

    pub fn sub_seed_power(&mut self, seed_id: &SeedId, amount: Balance) -> Balance {
        let prev_balance = self.seed_powers.get(seed_id).expect(&format!("{}", ERR31_SEED_NOT_EXIST));
        assert!(prev_balance >= &amount, "{}", ERR32_NOT_ENOUGH_SEED);
        let cur_balance = prev_balance - amount;
        if cur_balance > 0 {
            self.seed_powers.insert(seed_id.clone(), cur_balance);
        } else {
            self.seed_powers.remove(seed_id);
        }
        cur_balance
    }

    pub fn get_rps(&self, farm_id: &FarmId) -> RPS {
        self.user_rps.get(farm_id).unwrap_or(RPS::default()).clone()
    }

    pub fn set_rps(&mut self, farm_id: &FarmId, rps: RPS) {
        if !self.user_rps.contains_key(farm_id) {
            self.rps_count += 1;
        } 
        self.user_rps.insert(farm_id, &rps);
    }

    pub fn remove_rps(&mut self, farm_id: &FarmId) {
        if self.user_rps.contains_key(farm_id) {
            self.user_rps.remove(farm_id);
            self.rps_count -= 1;
        }
    }

    pub fn create_cd_account(&mut self, seed_id: SeedId, cd_strategy_index: usize, amount: Balance, cd_strategy: &CDStrategy) -> Balance {
        // let farming_amount = U256::from(amount) * 
        0
    }

    pub fn append_cd_account(&mut self, index: usize, seed_id: SeedId, cd_strategy_index: usize, amount: Balance, cd_strategy: &CDStrategy) -> Balance{
        0
    }

    /// Returns amount of yocto near necessary to cover storage used by this data structure.
    pub fn storage_usage(&self) -> Balance {
        (
            MIN_FARMER_LENGTH 
            + self.rewards.len() as u128 * (4 + MAX_ACCOUNT_LENGTH + 16)
            + self.seed_amounts.len() as u128 * (4 + MAX_ACCOUNT_LENGTH + 16)
            + self.rps_count as u128 * (4 + 1 + 2 * MAX_ACCOUNT_LENGTH + 32)
        )
        * env::storage_byte_cost()
    }
}


/// Versioned Farmer, used for lazy upgrade.
/// Which means this structure would upgrade automatically when used.
/// To achieve that, each time the new version comes in, 
/// each function of this enum should be carefully re-code!
#[derive(BorshSerialize, BorshDeserialize)]
pub enum VersionedFarmer {
    V101(Farmer),
}

impl VersionedFarmer {

    pub fn new(farmer_id: AccountId, amount: Balance) -> Self {
        VersionedFarmer::V101(Farmer {
            amount: amount,
            rewards: HashMap::new(),
            seed_amounts: HashMap::new(),
            seed_powers: HashMap::new(),
            user_rps: LookupMap::new(StorageKeys::UserRps {
                account_id: farmer_id.clone(),
            }),
            rps_count: 0,
            cd_accounts: Vector::new(StorageKeys::CDAccount {
                account_id: farmer_id.clone(),
            })
        })
    }

    /// Upgrades from other versions to the currently used version.
    pub fn upgrade(self) -> Self {
        match self {
            VersionedFarmer::V101(farmer) => VersionedFarmer::V101(farmer),
        }
    }

    #[inline]
    #[allow(unreachable_patterns)]
    pub fn need_upgrade(&self) -> bool {
        match self {
            VersionedFarmer::V101(_) => false,
            _ => true,
        }
    }

    #[inline]
    #[allow(unreachable_patterns)]
    pub fn get_ref(&self) -> &Farmer {
        match self {
            VersionedFarmer::V101(farmer) => farmer,
            _ => unimplemented!(),
        }
    }

    #[inline]
    #[allow(unreachable_patterns)]
    pub fn get(self) -> Farmer {
        match self {
            VersionedFarmer::V101(farmer) => farmer,
            _ => unimplemented!(),
        }
    }

    #[inline]
    #[allow(unreachable_patterns)]
    pub fn get_ref_mut(&mut self) -> &mut Farmer {
        match self {
            VersionedFarmer::V101(farmer) => farmer,
            _ => unimplemented!(),
        }
    }
}
