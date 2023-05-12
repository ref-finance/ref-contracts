use crate::*;


#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct FarmTerms {
    pub reward_token: AccountId,
    pub start_at: u32,
    #[serde(with = "u128_dec_format")]
    pub daily_reward: Balance,
}


#[derive(BorshSerialize, BorshDeserialize, Clone, Serialize, Deserialize, Debug)]
#[serde(crate = "near_sdk::serde")]
pub enum VSeedFarm {
    Current(SeedFarm),
}

impl From<VSeedFarm> for SeedFarm {
    fn from(v: VSeedFarm) -> Self {
        match v {
            VSeedFarm::Current(c) => c,
        }
    }
}

impl From<SeedFarm> for VSeedFarm {
    fn from(c: SeedFarm) -> Self {
        VSeedFarm::Current(c)
    }
}

#[derive(Serialize, Clone, Debug, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub enum FarmStatus {
    /// Just create and waiting for the start time
    Created,
    /// Past the start time but no reward deposited
    Pending,
    /// Past the start time and have reward to distribute
    Running,
    /// Past the start time and reward has been dry out
    Ended,
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Clone, Debug, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct SeedFarm {
    pub farm_id: FarmId,

    pub terms: FarmTerms,

    /// total reward send into this farm by far,
    /// every time reward deposited in, add to this field
    #[serde(with = "u128_dec_format")]
    pub total_reward: Balance,

    #[serde(with = "u64_dec_format")]
    pub distributed_at: Timestamp,

    /// The amount of rewards has been distributed.
    /// remaining_reward = total_reward - distributed_reward
    #[serde(with = "u128_dec_format")]
    pub distributed_reward: Balance,

    /// reward token has been claimed by farmer
    #[serde(with = "u128_dec_format")]
    pub claimed_reward: Balance,

    /// when there is no seed token staked, reward goes to beneficiary
    #[serde(with = "u128_dec_format")]
    pub amount_of_beneficiary: Balance,

    #[serde(skip)]
    pub rps: BigDecimal,

    #[borsh_skip]
    pub status: Option<FarmStatus>,
}

impl SeedFarm {
    pub fn new(id: FarmId, term: &FarmTerms) -> Self {
        Self {
            farm_id: id.clone(),
            terms: term.clone(),
            total_reward: 0,
            distributed_at: to_nano(term.start_at),
            distributed_reward: 0,
            claimed_reward: 0,
            amount_of_beneficiary: 0,
            rps: BigDecimal::from(0_u32),
            status: Some(FarmStatus::Created),
        }
    }

    pub fn has_ended(&self) -> bool {
        match self.status.as_ref().unwrap() {
            FarmStatus::Ended => true,
            _ => false,
        }
    }

    pub fn internal_update_status(&mut self, block_ts: u64) {
        if self.terms.start_at == 0 || to_nano(self.terms.start_at) >= block_ts {
            self.status = Some(FarmStatus::Created);
        } else if self.total_reward == 0 && to_nano(self.terms.start_at) < block_ts {
            self.status = Some(FarmStatus::Pending);
        } else if self.total_reward > 0 && self.distributed_reward >= self.total_reward {
            self.status = Some(FarmStatus::Ended);
        } else {
            self.status = Some(FarmStatus::Running);
        }
    }

    pub fn update(&mut self, seed_power: Balance) {
        let block_ts = env::block_timestamp();

        self.internal_update_status(block_ts);

        if block_ts <= self.distributed_at {
            // already updated, skip
            return;
        }

        match self.status.as_ref().unwrap() {
            FarmStatus::Ended => {
                self.distributed_at = block_ts;
            },
            FarmStatus::Running => {
                let reward = std::cmp::min(
                    self.total_reward - self.distributed_reward,
                    u128_ratio(
                        self.terms.daily_reward,
                        u128::from(block_ts - self.distributed_at),
                        u128::from(NANOS_PER_DAY),
                    ),
                );
                self.distributed_reward += reward;
                if seed_power > 0 {
                    self.rps = self.rps + BigDecimal::from(reward).div_u128(seed_power);
                } else {
                    self.amount_of_beneficiary += reward;
                }
                self.distributed_at = block_ts;
                self.internal_update_status(block_ts);
            },
            _ => {},
        }
    }

    pub fn add_reward(&mut self, reward_token: &AccountId, amount: Balance) -> (Balance, u32) {
        assert!(self.terms.reward_token == reward_token.clone(), "{}", E404_UNMATCHED_REWARD_TOKEN);
        if self.terms.start_at == 0 {
            self.terms.start_at = nano_to_sec(env::block_timestamp());
            self.distributed_at = env::block_timestamp();
        }
        self.total_reward += amount;
        (self.total_reward, self.terms.start_at)
    }

    pub fn finalize(&mut self) {
        assert!(self.has_ended(), "{}", E405_FARM_NOT_ENDED);
        // remaining unclaimed rewards belongs to beneficiary
        self.amount_of_beneficiary = 
            self.distributed_reward - self.claimed_reward;
    }

}

impl Contract {
    pub fn internal_get_outdated_farm(&self, farm_id: &FarmId) -> Option<SeedFarm> {
        self.data().outdated_farms.get(farm_id).map(|o| o.into())
    }

    pub fn internal_unwrap_outdated_farm(&self, farm_id: &FarmId) -> SeedFarm {
        self.internal_get_outdated_farm(farm_id)
            .expect(E401_FARM_NOT_EXIST)
    }

    pub fn internal_set_outdated_farm(&mut self, farm_id: &FarmId, farm: SeedFarm) {
        self.data_mut().outdated_farms.insert(farm_id, &farm.into());
    }
}