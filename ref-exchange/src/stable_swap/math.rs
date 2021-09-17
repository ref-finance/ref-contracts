///! Calculator to maintain the invariant on adding/removing liquidity and on swapping.
///! Large part of the code was taken from https://github.com/saber-hq/stable-swap/blob/master/stable-swap-math/src/curve.rs
use near_sdk::{Balance, Timestamp};

use crate::fees::SwapFees;
use crate::utils::{FEE_DIVISOR, U256};

/// Number of coins in the pool.
pub const N_COINS: u32 = 2;
/// Minimum ramp duration.
pub const MIN_RAMP_DURATION: Timestamp = 86400;
/// Min amplification coefficient.
pub const MIN_AMP: u128 = 1;
/// Max amplification coefficient.
pub const MAX_AMP: u128 = 1_000_000;
/// Max amplification change.
pub const MAX_AMP_CHANGE: u128 = 10;

/// Stable Swap Fee calculator.
pub struct Fees {
    pub trade_fee: u32,
    pub admin_fee: u32,
}

impl Fees {
    pub fn new(total_fee: u32, fees: &SwapFees) -> Self {
        Self {
            trade_fee: total_fee - fees.exchange_fee,
            admin_fee: fees.exchange_fee,
        }
    }
    pub fn trade_fee(&self, amount: Balance) -> Balance {
        println!(
            "trade fee: {} {}",
            amount * (self.trade_fee as u128) / (FEE_DIVISOR as u128),
            amount
        );
        amount * (self.trade_fee as u128) / (FEE_DIVISOR as u128)
    }

    pub fn admin_trade_fee(&self, amount: Balance) -> Balance {
        amount * (self.admin_fee as u128) / (FEE_DIVISOR as u128)
    }

    pub fn normalized_trade_fee(&self, num_coins: u32, amount: Balance) -> Balance {
        let adjusted_trade_fee = (self.trade_fee * num_coins) / (4 * (num_coins - 1));
        amount * (adjusted_trade_fee as u128) / (FEE_DIVISOR as u128)
    }
}

/// Encodes all results of swapping from a source token to a destination token.
#[derive(Debug)]
pub struct SwapResult {
    /// New amount of source token.
    pub new_source_amount: Balance,
    /// New amount of destination token.
    pub new_destination_amount: Balance,
    /// Amount of destination token swapped.
    pub amount_swapped: Balance,
    /// Admin fee for the swap.
    pub admin_fee: Balance,
    /// Fee for the swap.
    pub fee: Balance,
}

/// The StableSwap invariant calculator.
pub struct StableSwap {
    /// Initial amplification coefficient (A)
    initial_amp_factor: u128,
    /// Target amplification coefficient (A)
    target_amp_factor: u128,
    /// Current unix timestamp
    current_ts: Timestamp,
    /// Ramp A start timestamp
    start_ramp_ts: Timestamp,
    /// Ramp A stop timestamp
    stop_ramp_ts: Timestamp,
}

impl StableSwap {
    pub fn new(
        initial_amp_factor: u128,
        target_amp_factor: u128,
        current_ts: Timestamp,
        start_ramp_ts: Timestamp,
        stop_ramp_ts: Timestamp,
    ) -> Self {
        Self {
            initial_amp_factor,
            target_amp_factor,
            current_ts,
            start_ramp_ts,
            stop_ramp_ts,
        }
    }

    fn compute_next_d(
        &self,
        amp_factor: u128,
        d_init: U256,
        d_prod: U256,
        sum_x: Balance,
    ) -> Option<U256> {
        let ann = amp_factor.checked_mul(N_COINS.into())?;
        let leverage = (sum_x as u128).checked_mul(ann.into())?;
        // d = (ann * sum_x + d_prod * n_coins) * d / ((ann - 1) * d + (n_coins + 1) * d_prod)
        let numerator = d_init.checked_mul(
            d_prod
                .checked_mul(N_COINS.into())?
                .checked_add(leverage.into())?,
        )?;
        let denominator = d_init
            .checked_mul(ann.checked_sub(1)?.into())?
            .checked_add(d_prod.checked_mul((N_COINS + 1).into())?)?;
        numerator.checked_div(denominator)
    }

    /// Compute the amplification coefficient (A)
    pub fn compute_amp_factor(&self) -> Option<Balance> {
        if self.current_ts < self.stop_ramp_ts {
            let time_range = self.stop_ramp_ts.checked_sub(self.start_ramp_ts)?;
            let time_delta = self.current_ts.checked_sub(self.start_ramp_ts)?;

            // Compute amp factor based on ramp time
            if self.target_amp_factor >= self.initial_amp_factor {
                // Ramp up
                let amp_range = self
                    .target_amp_factor
                    .checked_sub(self.initial_amp_factor)?;
                let amp_delta = (amp_range as u128)
                    .checked_mul(time_delta as u128)?
                    .checked_div(time_range as u128)?;
                self.initial_amp_factor
                    .checked_add(amp_delta)
                    .map(|x| x as u128)
            } else {
                // Ramp down
                let amp_range = self
                    .initial_amp_factor
                    .checked_sub(self.target_amp_factor)?;
                let amp_delta = (amp_range as u128)
                    .checked_mul(time_delta as u128)?
                    .checked_div(time_range as u128)?;
                self.initial_amp_factor
                    .checked_sub(amp_delta)
                    .map(|x| x as u128)
            }
        } else {
            // when stop_ramp_ts == 0 or current_ts >= stop_ramp_ts
            Some(self.target_amp_factor as u128)
        }
    }

    /// Compute stable swap invariant (D)
    /// Equation:
    /// A * sum(x_i) * n**n + D = A * D * n**n + D**(n+1) / (n**n * prod(x_i))
    pub fn compute_d(&self, amount_a: Balance, amount_b: Balance) -> Option<U256> {
        let sum_x = amount_a.checked_add(amount_b)?; // sum(x_i), a.k.a S
        if sum_x == 0 {
            Some(0.into())
        } else {
            let amp_factor = self.compute_amp_factor()?;
            let amount_a_times_coins = amount_a.checked_mul(N_COINS.into())?;
            let amount_b_times_coins = amount_b.checked_mul(N_COINS.into())?;

            // Newton's method to approximate D
            let mut d_prev: U256;
            let mut d: U256 = sum_x.into();
            for _ in 0..256 {
                let mut d_prod = d;
                d_prod = d_prod
                    .checked_mul(d)?
                    .checked_div(amount_a_times_coins.into())?;
                d_prod = d_prod
                    .checked_mul(d)?
                    .checked_div(amount_b_times_coins.into())?;
                d_prev = d;
                d = self.compute_next_d(amp_factor, d, d_prod, sum_x)?;
                // Equality with the precision of 1
                if d > d_prev {
                    if d.checked_sub(d_prev)? <= 1.into() {
                        break;
                    }
                } else if d_prev.checked_sub(d)? <= 1.into() {
                    break;
                }
            }

            Some(d)
        }
    }

    /// Compute the amount of LP tokens to mint after a deposit
    pub fn compute_lp_amount_for_deposit(
        &self,
        deposit_amount_a: Balance,
        deposit_amount_b: Balance,
        swap_amount_a: Balance,
        swap_amount_b: Balance,
        pool_token_supply: Balance,
        fees: &Fees,
    ) -> Option<Balance> {
        // Initial invariant
        let d_0 = self.compute_d(swap_amount_a, swap_amount_b)?;
        let old_balances = [swap_amount_a, swap_amount_b];
        let mut new_balances = [
            swap_amount_a.checked_add(deposit_amount_a)?,
            swap_amount_b.checked_add(deposit_amount_b)?,
        ];
        // Invariant after change
        let d_1 = self.compute_d(new_balances[0], new_balances[1])?;
        if d_1 <= d_0 {
            None
        } else {
            // Recalculate the invariant accounting for fees
            for i in 0..new_balances.len() {
                let ideal_balance = d_1
                    .checked_mul(old_balances[i].into())?
                    .checked_div(d_0)?
                    .as_u128();
                let difference = if ideal_balance > new_balances[i] {
                    ideal_balance.checked_sub(new_balances[i])?
                } else {
                    new_balances[i].checked_sub(ideal_balance)?
                };
                let fee = fees.normalized_trade_fee(N_COINS, difference);
                new_balances[i] = new_balances[i].checked_sub(fee)?;
            }

            let d_2 = self.compute_d(new_balances[0], new_balances[1])?;
            Some(
                U256::from(pool_token_supply)
                    .checked_mul(d_2.checked_sub(d_0)?)?
                    .checked_div(d_0)?
                    .as_u128(),
            )
        }
    }

    /// Compute swap amount `y` in proportion to `x`
    /// Solve for y:
    /// y**2 + y * (sum' - (A*n**n - 1) * D / (A * n**n)) = D ** (n + 1) / (n ** (2 * n) * prod' * A)
    /// y**2 + b*y = c
    pub fn compute_y_raw(&self, x: Balance, d: U256) -> Option<U256> {
        let amp_factor = self.compute_amp_factor()?;
        let ann = amp_factor.checked_mul(N_COINS.into())?; // A * n ** n

        // sum' = prod' = x
        // c =  D ** (n + 1) / (n ** (2 * n) * prod' * A)
        let mut c = d
            .checked_mul(d)?
            .checked_div(x.checked_mul(N_COINS.into())?.into())?;
        c = c
            .checked_mul(d)?
            .checked_div(ann.checked_mul(N_COINS.into())?.into())?;
        // b = sum' - (A*n**n - 1) * D / (A * n**n)
        let b = d.checked_div(ann.into())?.checked_add(x.into())?; // d is subtracted on line 147

        // Solve for y by approximating: y**2 + b*y = c
        let mut y_prev: U256;
        let mut y = d;
        for _ in 0..256 {
            y_prev = y;
            // y = (y * y + c) / (2 * y + b - d);
            let y_numerator = y.checked_pow(2.into())?.checked_add(c)?;
            let y_denominator = y.checked_mul(2.into())?.checked_add(b)?.checked_sub(d)?;
            y = y_numerator.checked_div(y_denominator)?;
            if y > y_prev {
                if y.checked_sub(y_prev)? <= 1.into() {
                    break;
                }
            } else if y_prev.checked_sub(y)? <= 1.into() {
                break;
            }
        }
        Some(y)
    }

    /// Compute swap amount `y` in proportion to `x`
    pub fn compute_y(&self, x: Balance, d: U256) -> u128 {
        self.compute_y_raw(x, d).unwrap().as_u128()
    }

    /// Calculate withdrawal amount when withdrawing only one type of token
    /// Calculation:
    /// 1. Get current D
    /// 2. Solve Eqn against y_i for D - _token_amount
    pub fn compute_withdraw_one(
        &self,
        pool_token_amount: Balance,
        pool_token_supply: Balance,
        swap_base_amount: Balance, // Same denomination of token to be withdrawn
        swap_quote_amount: Balance, // Counter denomination of token to be withdrawn
        fees: &Fees,
    ) -> Option<(Balance, Balance)> {
        let d_0 = self.compute_d(swap_base_amount, swap_quote_amount)?;
        let d_1 = d_0.checked_sub(
            U256::from(pool_token_amount)
                .checked_mul(d_0)?
                .checked_div(pool_token_supply.into())?,
        )?;
        let new_y = self.compute_y(swap_quote_amount, d_1);

        // expected_base_amount = swap_base_amount * d_1 / d_0 - new_y;
        let expected_base_amount = U256::from(swap_base_amount)
            .checked_mul(d_1)?
            .checked_div(d_0)?
            .as_u128()
            .checked_sub(new_y)?;
        // expected_quote_amount = swap_quote_amount - swap_quote_amount * d_1 / d_0;
        let expected_quote_amount = swap_quote_amount.checked_sub(
            U256::from(swap_quote_amount)
                .checked_mul(d_1)?
                .checked_div(d_0)?
                .as_u128(),
        )?;
        // new_base_amount = swap_base_amount - expected_base_amount * fee / fee_denominator;
        let new_base_amount = swap_base_amount
            .checked_sub(fees.normalized_trade_fee(N_COINS, expected_base_amount))?;
        // new_quote_amount = swap_quote_amount - expected_quote_amount * fee / fee_denominator;
        let new_quote_amount = swap_quote_amount
            .checked_sub(fees.normalized_trade_fee(N_COINS, expected_quote_amount))?;
        let dy = new_base_amount
            .checked_sub(self.compute_y(new_quote_amount, d_1))?
            .checked_sub(1)?; // Withdraw less to account for rounding errors
        let dy_0 = swap_base_amount.checked_sub(new_y)?;

        Some((dy, dy_0 - dy))
    }

    /// Compute SwapResult after an exchange
    pub fn swap_to(
        &self,
        source_amount: Balance,
        swap_source_amount: Balance,
        swap_destination_amount: Balance,
        fees: &Fees,
    ) -> Option<SwapResult> {
        let y = self.compute_y(
            swap_source_amount.checked_add(source_amount)?,
            self.compute_d(swap_source_amount, swap_destination_amount)?,
        );
        let dy = swap_destination_amount.checked_sub(y)?;
        let dy_fee = fees.trade_fee(dy);
        let admin_fee = fees.admin_trade_fee(dy_fee);

        let amount_swapped = dy.checked_sub(dy_fee)?;
        let new_destination_amount = swap_destination_amount
            .checked_sub(amount_swapped)?
            .checked_sub(admin_fee)?;
        let new_source_amount = swap_source_amount.checked_add(source_amount)?;

        Some(SwapResult {
            new_source_amount,
            new_destination_amount,
            amount_swapped,
            admin_fee,
            fee: dy_fee,
        })
    }
}
