//!   The SimpleFarm provide a way to gain farming rewards periodically and 
//! proportionally.
//!   The creator first wrap his reward distribution schema with 
//! `SimpleFarmRewardTerms`, and create the farm with it, attached enough near 
//! for storage fee.
//!   But to enable farming, the creator or someone else should deposit reward 
//! token to the farm, after it was created.

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::{U64, U128, ValidAccountId};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{env, AccountId, Balance, BlockHeight};

use crate::{SeedId, FarmId};
use crate::errors::*;
use uint::construct_uint;

construct_uint! {
    /// 256-bit unsigned integer.
    pub struct U256(4);
}

pub type RPS = [u8; 32];

// to ensure precision, all reward_per_seed would be multiplied by this DENOM
const DENOM: u128 = 1_000_000_000_000_000_000;

///   The terms defines how the farm works.
///   In this version, we distribute reward token with a start height, a reward 
/// session interval, and reward amount per session.  
///   In this way, the farm will take the amount from undistributed reward to  
/// unclaimed reward each session. And all farmers would got reward token pro  
/// rata of their seeds.
#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub struct SimpleFarmTerms {
    pub seed_id: SeedId,
    pub reward_token: AccountId,
    pub start_at: BlockHeight,
    pub reward_per_session: Balance,
    pub session_interval: BlockHeight,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct HRSimpleFarmTerms {
    pub seed_id: SeedId,
    pub reward_token: ValidAccountId,
    pub start_at: U64,
    pub reward_per_session: U128,
    pub session_interval: U64, 
}

impl From<&HRSimpleFarmTerms> for SimpleFarmTerms {
    fn from(terms: &HRSimpleFarmTerms) -> Self {
        SimpleFarmTerms {
            seed_id: terms.seed_id.clone(),
            reward_token: terms.reward_token.clone().into(),
            start_at: terms.start_at.into(),
            reward_per_session: terms.reward_per_session.into(),
            session_interval: terms.session_interval.into(),
        }
    }
}

#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub enum SimpleFarmStatus {
    Created, Running, Ended, Cleared
}

impl From<&SimpleFarmStatus> for String {
    fn from(status: &SimpleFarmStatus) -> Self {
        match *status {
            SimpleFarmStatus::Created => { String::from("Created") },
            SimpleFarmStatus::Running => { String::from("Running") },
            SimpleFarmStatus::Ended => { String::from("Ended") },
            SimpleFarmStatus::Cleared => { String::from("Cleared") },
        }
    }
}

/// Reward Distribution Record
#[derive(BorshSerialize, BorshDeserialize, Clone, Default)]
pub struct SimpleFarmRewardDistribution {
    /// unreleased reward
    pub undistributed: Balance,
    /// the total rewards distributed but not yet claimed by farmers.
    pub unclaimed: Balance,
    /// Reward_Per_Seed
    /// rps(cur) = rps(prev) + distributing_reward / total_seed_staked
    pub rps: RPS,
    /// Reward_Round
    /// rr = (cur_block_height - start_at) / session_interval
    pub rr: u64,
}

///   Implementation of simple farm, Similar to the design of "berry farm".
///   Farmer stake their seed to farming on multiple farm accept that seed.
#[derive(BorshSerialize, BorshDeserialize)]
pub struct SimpleFarm {

    pub farm_id: FarmId,
    
    pub terms: SimpleFarmTerms,

    pub status: SimpleFarmStatus,

    pub last_distribution: SimpleFarmRewardDistribution,

    /// total reward send into this farm by far, 
    /// every time reward deposited in, add to this field
    pub amount_of_reward: Balance,
    /// reward token has been claimed by farmer by far
    pub amount_of_claimed: Balance,

}

impl SimpleFarm {
    pub(crate) fn new(
        id: FarmId,
        terms: SimpleFarmTerms,
    ) -> Self {
        Self {
            farm_id: id.clone(),
            amount_of_reward: 0,
            amount_of_claimed: 0,

            status: SimpleFarmStatus::Created,
            last_distribution: SimpleFarmRewardDistribution::default(),
            terms,
        }
    }

    /// return None if the farm can not accept reward anymore
    /// else return amount of undistributed reward 
    pub(crate) fn add_reward(&mut self, amount: &Balance) -> Option<Balance> {

        match self.status {
            SimpleFarmStatus::Created => {
                self.status = SimpleFarmStatus::Running;
                if self.terms.start_at == 0 {
                    self.terms.start_at = env::block_index();
                }
                self.amount_of_reward += amount;
                self.last_distribution.undistributed += amount;
                Some(self.last_distribution.undistributed)
            },
            SimpleFarmStatus::Running => {
                self.amount_of_reward += amount;
                self.last_distribution.undistributed += amount;
                Some(self.last_distribution.undistributed)
            },
            _ => {None},
        }
        
    }


    pub(crate) fn try_distribute(&self, total_seeds: &Balance) -> Option<SimpleFarmRewardDistribution> {

        assert!(
            total_seeds != &0_u128,
            "{}", ERR500
        );

        if let SimpleFarmStatus::Running = self.status {
            let mut dis = self.last_distribution.clone();
            // calculate rr according to cur_height
            dis.rr = (env::block_index() - self.terms.start_at) / self.terms.session_interval;
            let mut reward_added = (dis.rr - self.last_distribution.rr) as u128 
                * self.terms.reward_per_session;
            if self.last_distribution.undistributed < reward_added {
                // recalculate rr according to undistributed
                let increased_rr = (self.last_distribution.undistributed 
                    / self.terms.reward_per_session) as u64;
                dis.rr = self.last_distribution.rr + increased_rr;
                reward_added = increased_rr as u128 * self.terms.reward_per_session;
                env::log(
                    format!(
                        "Farm ends at Round #{}, unclaimed reward: {}.",
                        dis.rr, reward_added + dis.unclaimed
                    )
                    .as_bytes(),
                );
            }
            dis.unclaimed += reward_added;
            dis.undistributed -= reward_added;

            // calculate rps
            (
                U256::from_little_endian(&self.last_distribution.rps) + 
                U256::from(reward_added) 
                * U256::from(DENOM) 
                / U256::from(*total_seeds)
            ).to_little_endian(&mut dis.rps);

            Some(dis)
        } else {
            None
        }

    }

    /// Return how many reward token that the user hasn't claimed yet.
    /// return (cur_rps - last_user_rps) * user_seeds / DENOM
    pub(crate) fn view_farmer_unclaimed_reward(
        &self,
        user_rps: &RPS,
        user_seeds: &Balance,
        total_seeds: &Balance,
    ) -> Balance {
        if total_seeds == &0 {
            return 0;
        }
        if let Some(dis) = self.try_distribute(total_seeds) {
            (U256::from(*user_seeds) 
            * (U256::from_little_endian(&dis.rps) - U256::from_little_endian(user_rps))
            / U256::from(DENOM)).as_u128()
        } else {
            0
        }
    }

    pub(crate) fn distribute(&mut self, total_seeds: &Balance) {
        if total_seeds == &0 {
            return;
        }
    
        if let Some(dis) = self.try_distribute(total_seeds) {
            if self.last_distribution.rr != dis.rr {
                self.last_distribution = dis.clone();
                env::log(
                    format!(
                        "{} RPS increased to {} and RR update to #{}",
                        self.farm_id, U256::from_little_endian(&dis.rps), dis.rr,
                    )
                    .as_bytes(),
                );
            }
            if self.last_distribution.undistributed < self.terms.reward_per_session {
                self.status = SimpleFarmStatus::Ended;
            }
        } 
    }

    /// return the new user reward per seed 
    /// and amount of reward as (user_rps, reward_amount) 
    pub(crate) fn claim_user_reward(
        &mut self, 
        user_rps: &RPS,
        user_seeds: &Balance, 
        total_seeds: &Balance
    ) -> Option<(RPS, Balance)> {

        if total_seeds == &0 {
            return None;
        }

        self.distribute(total_seeds);

        let claimed = (
            U256::from(*user_seeds) 
            * (U256::from_little_endian(&self.last_distribution.rps) - U256::from_little_endian(user_rps))
            / U256::from(DENOM)
        ).as_u128();

        if claimed > 0 {
            assert!(self.last_distribution.unclaimed >= claimed, "{}", ERR500);
            self.last_distribution.unclaimed -= claimed;
        }

        self.amount_of_claimed += claimed;
        
        Some((self.last_distribution.rps, claimed))
    }

    pub fn can_be_removed(&self) -> bool {
        if let SimpleFarmStatus::Ended = self.status {
            if self.amount_of_claimed == self.amount_of_reward {
                true
            } else {
                false
            }
        } else {
            false
        }
    }

}

