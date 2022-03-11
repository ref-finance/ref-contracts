//! Farmer records a farmer's 
//! * all claimed reward tokens, 
//! * all seeds he staked,
//! * all cd account he add,
//! * user_rps per farm,

use std::collections::HashMap;
use near_sdk::collections::{LookupMap, Vector};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{env, AccountId, Balance};
use near_sdk::serde::{Deserialize, Serialize};
use crate::{SeedId, FarmId, RPS};
use crate::*;
use crate::errors::*;
use crate::utils::*;
use crate::StorageKeys;

/// If seed_amount == 0, this CDAccount is empty and can be occupied.
/// When add/remove seed to/from a non-empty CDAccount, 
/// the delta power is calculate based on delta seed, current timestamp, begin_sec and end_sec.
/// When remove seed before end_sec, a slash on seed amount would happen, 
/// based on remove amount, seed_slash_rate, current timestamp, begin_sec and end_sec.
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct CDAccount {
    pub seed_id: SeedId,
    /// actual seed balance user staked
    pub seed_amount: Balance,
    /// implied power_reward_rate when occupied
    pub original_power_reward_rate: u32,
    /// shares used in reward distribution
    pub seed_power: Balance,
    /// the begin timestamp
    pub begin_sec: TimestampSec,
    /// promise not unstake before this timestamp
    pub end_sec: TimestampSec,
}

impl Default for CDAccount {
    fn default() -> CDAccount {
        CDAccount {
            seed_id: "".to_string(),
            seed_amount: 0,
            original_power_reward_rate: 0,
            seed_power: 0,
            begin_sec: 0,
            end_sec: 0,
        }
    }
}

impl CDAccount {
    /// return power
    pub(crate) fn occupy(&mut self, seed_id: &SeedId, amount: Balance, power_reward_rate: u32, lasts_sec: u32) -> Balance {
        assert_eq!(self.seed_amount, 0, "{}", ERR65_NON_EMPTY_CD_ACCOUNT);
        assert!(lasts_sec > 0, "{}", ERR68_INVALID_CD_STRATEGY);

        self.seed_id = seed_id.clone();
        self.seed_amount = amount;
        self.original_power_reward_rate = power_reward_rate;
        self.seed_power = amount + (U256::from(amount) * U256::from(power_reward_rate) / U256::from(DENOM)).as_u128();
        self.begin_sec = to_sec(env::block_timestamp());
        self.end_sec = self.begin_sec + lasts_sec;
        self.seed_power
    }

    /// return power added
    pub(crate) fn append(&mut self, seed_id: &SeedId, amount: Balance) -> Balance {
        assert!(self.seed_amount > 0, "{}", ERR66_EMPTY_CD_ACCOUNT);
        assert_eq!(self.seed_id, seed_id.clone(), "{}", ERR67_UNMATCHED_SEED_ID);

        self.seed_amount += amount;

        let now = to_sec(env::block_timestamp());
        let power_reward = if now < self.end_sec && now > self.begin_sec {
            let full_reward = U256::from(amount) * U256::from(self.original_power_reward_rate) / U256::from(DENOM);
            (full_reward * U256::from(self.end_sec - now) / U256::from(self.end_sec - self.begin_sec)).as_u128()
        } else {
            0
        };
        self.seed_power += amount + power_reward;

        amount + power_reward
    }

    /// return power removed and seed slashed
    pub(crate) fn remove(&mut self, seed_id: &SeedId, amount: Balance, slash_rate: u32) -> (Balance, Balance) {
        assert!(self.seed_amount > 0, "{}", ERR66_EMPTY_CD_ACCOUNT);
        assert_eq!(self.seed_id, seed_id.clone(), "{}", ERR67_UNMATCHED_SEED_ID);
        assert!(self.seed_amount >= amount, "{}", ERR32_NOT_ENOUGH_SEED);

        let now = to_sec(env::block_timestamp());
        let seed_slashed = if now < self.end_sec && now >= self.begin_sec {
            let full_slashed = U256::from(amount) * U256::from(slash_rate) / U256::from(DENOM);
            (full_slashed * U256::from(self.end_sec - now) / U256::from(self.end_sec - self.begin_sec)).as_u128()
        } else {
            0
        };
        
        let power_removed = (U256::from(self.seed_power) * U256::from(amount) / U256::from(self.seed_amount)).as_u128();

        self.seed_amount -= amount;
        self.seed_power -= power_removed;

        (power_removed, seed_slashed)
    }
}


/// Account deposits information and storage cost.
#[derive(BorshSerialize, BorshDeserialize)]
#[cfg_attr(feature = "test", derive(Clone))]
pub struct Farmer {
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

    pub fn new(farmer_id: AccountId) -> Self {
        VersionedFarmer::V101(Farmer {
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
