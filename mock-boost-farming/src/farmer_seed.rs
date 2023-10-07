use crate::*;

#[derive(BorshSerialize, BorshDeserialize, Serialize, Default)]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug, Deserialize))]
#[serde(crate = "near_sdk::serde")]
pub struct FarmerSeed {
    #[serde(with = "u128_dec_format")]
    pub free_amount: Balance,
    /// The amount of locked token.
    #[serde(with = "u128_dec_format")]
    pub locked_amount: Balance,
    /// The amount of power for those locked amount.
    #[serde(with = "u128_dec_format")]
    pub x_locked_amount: Balance,
    /// When the locking token can be unlocked without slash in nanoseconds.
    #[serde(with = "u64_dec_format")]
    pub unlock_timestamp: u64,
    /// The duration of current locking in seconds.
    pub duration_sec: u32,
    /// <booster_id, booster-ratio>
    pub boost_ratios: HashMap<SeedId, f64>,
    #[serde(skip_serializing)]
    pub user_rps: HashMap<FarmId, BigDecimal>,
}

impl FarmerSeed {
    pub fn get_seed_power(&self) -> Balance {
        let base_power = self.get_basic_seed_power();
        let extras: Vec<u128> = self.boost_ratios.values().map(|ratio|((base_power as f64) * ratio) as u128).collect();
        base_power + extras.iter().sum::<u128>()
    }

    pub fn get_basic_seed_power(&self) -> Balance {
        self.free_amount + self.x_locked_amount
    }

    pub fn is_empty(&self) -> bool {
        self.free_amount + self.locked_amount == 0
    }

    pub fn add_free(&mut self, amount: Balance) -> Balance {
        let prev = self.get_seed_power();
        self.free_amount += amount;
        self.get_seed_power() - prev
    }

    pub fn withdraw_free(&mut self, amount: Balance) -> Balance {
        assert!(amount <= self.free_amount, "{}", E101_INSUFFICIENT_BALANCE);
        let prev = self.get_seed_power();
        self.free_amount -= amount;
        prev - self.get_seed_power()
    }

    pub fn add_lock(&mut self, amount: Balance, duration_sec: u32, config: &Config) -> Balance {
        let prev = self.get_seed_power();

        let timestamp = env::block_timestamp();
        let new_unlock_timestamp = timestamp + to_nano(duration_sec);

        if self.unlock_timestamp > 0 && self.unlock_timestamp > timestamp {
            // exist x locked need relock
            assert!(self.unlock_timestamp <= new_unlock_timestamp, "{}", E304_CAUSE_PRE_UNLOCK);
            let relocked_x = compute_x_amount(&config, self.locked_amount, duration_sec);
            self.x_locked_amount = std::cmp::max(self.x_locked_amount, relocked_x);
            let extra_x = compute_x_amount(config, amount, duration_sec);
            self.x_locked_amount += extra_x;
        } else {
            self.x_locked_amount = compute_x_amount(config, self.locked_amount + amount, duration_sec);
        }
        self.unlock_timestamp = new_unlock_timestamp;
        self.locked_amount += amount;
        self.duration_sec = duration_sec;

        self.get_seed_power() - prev
    }

    pub fn free_to_lock(&mut self, amount: Balance, duration_sec: u32, config: &Config) -> Balance {
        assert!(amount <= self.free_amount, "{}", E101_INSUFFICIENT_BALANCE);
        let prev = self.get_seed_power();
        self.free_amount -= amount;

        assert!(
            duration_sec <= config.maximum_locking_duration_sec,
            "{}", E201_INVALID_DURATION
        );

        let timestamp = env::block_timestamp();
        let new_unlock_timestamp = timestamp + to_nano(duration_sec);

        if self.unlock_timestamp > 0 && self.unlock_timestamp > timestamp {
            // exist x locked need relock
            assert!(self.unlock_timestamp <= new_unlock_timestamp, "{}", E304_CAUSE_PRE_UNLOCK);
            let relocked_x = compute_x_amount(&config, self.locked_amount, duration_sec);
            self.x_locked_amount = std::cmp::max(self.x_locked_amount, relocked_x);
            let extra_x = compute_x_amount(config, amount, duration_sec);
            self.x_locked_amount += extra_x;
        } else {
            self.x_locked_amount = compute_x_amount(config, self.locked_amount + amount, duration_sec);
        }
        self.unlock_timestamp = new_unlock_timestamp;
        self.locked_amount += amount;
        self.duration_sec = duration_sec;
        
        self.get_seed_power() - prev
    }

    pub fn unlock_to_free(&mut self, amount: Balance) -> Balance {
        let prev = self.get_seed_power();

        let timestamp = env::block_timestamp();
        assert!(self.unlock_timestamp < timestamp && self.unlock_timestamp != 0, "{}", E305_STILL_IN_LOCK);
        assert!(amount <= self.locked_amount && amount != 0, "{}", E101_INSUFFICIENT_BALANCE);

        if amount < self.locked_amount {
            let new_x = u128_ratio(self.x_locked_amount, self.locked_amount - amount, self.locked_amount);
            assert!(new_x < self.x_locked_amount, "{}", E306_LOCK_AMOUNT_TOO_SMALL);
            self.x_locked_amount = new_x;
        } else {
            self.x_locked_amount = 0;
            self.unlock_timestamp = 0;
            self.duration_sec = 0;
        }
        self.free_amount += amount;
        self.locked_amount -= amount;

        prev - self.get_seed_power()
    }

    pub fn unlock_to_free_with_slashed(&mut self, amount: Balance, slash_rate: u32) -> (Balance, Balance) {
        let prev = self.get_seed_power();

        let timestamp = env::block_timestamp();
        assert!(self.unlock_timestamp > timestamp, "{}", E309_NO_NEED_FORCE);
        assert!(amount <= self.locked_amount && amount != 0, "{}", E101_INSUFFICIENT_BALANCE);

        let full_slashed = u128_ratio(amount, slash_rate as u128, BP_DENOM);
        let seed_slashed = u128_ratio(full_slashed, (self.unlock_timestamp - timestamp) as u128, to_nano(self.duration_sec) as u128);

        if amount < self.locked_amount {
            let new_x = u128_ratio(self.x_locked_amount, self.locked_amount - amount, self.locked_amount);
            assert!(new_x < self.x_locked_amount, "{}", E306_LOCK_AMOUNT_TOO_SMALL);
            self.x_locked_amount = new_x;
        } else {
            self.x_locked_amount = 0;
            self.unlock_timestamp = 0;
            self.duration_sec = 0;
        }

        self.free_amount += amount - seed_slashed;
        self.locked_amount -= amount;

        (prev - self.get_seed_power(), seed_slashed)
    }
}

fn compute_x_amount(config: &Config, amount: u128, duration_sec: u32) -> u128 {
    amount
        + u128_ratio(
            amount,
            u128::from(config.max_locking_multiplier - MIN_LOCKING_REWARD_RATIO) * u128::from(to_nano(duration_sec)),
            u128::from(to_nano(config.maximum_locking_duration_sec)) * MIN_LOCKING_REWARD_RATIO as u128,
        )
}
