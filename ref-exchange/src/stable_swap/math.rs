///! Calculator to maintain the invariant on adding/removing liquidity and on swapping.
///! Large part of the code was taken from https://github.com/saber-hq/stable-swap/blob/master/stable-swap-math/src/curve.rs
use near_sdk::{Balance, Timestamp};

use crate::fees::SwapFees;
use crate::utils::{FEE_DIVISOR, U256};

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
    pub fn compute_d_ex(&self, c_amounts: &Vec<Balance>) -> Option<U256> {
        let n_coins = c_amounts.len() as u128;
        let sum_x = c_amounts.iter().fold(0, |sum, i| sum + i);
        if sum_x == 0 {
            Some(0.into())
        } else {
            let amp_factor = self.compute_amp_factor()?;
            let mut d_prev: U256;
            let mut d: U256 = sum_x.into();
            for _ in 0..256 {
                // $ D_{k,prod} = \frac{D_k^{n+1}}{n^n \prod x_{i}} = \frac{D^3}{4xy} $
                let mut d_prod = d;
                for c_amount in c_amounts {
                    d_prod = d_prod.checked_mul(d)?
                    .checked_div((c_amount * n_coins + 1).into())?; // +1 to prevent divided by zero
                }
                d_prev = d;

                // let ann = amp_factor.checked_mul(N_COINS.checked_pow(N_COINS)?.into())?;
                let ann = amp_factor.checked_mul(n_coins.into())?;
                let leverage = (sum_x as u128).checked_mul(ann.into())?;
                // d = (ann * sum_x + d_prod * n_coins) * d_prev / ((ann - 1) * d_prev + (n_coins + 1) * d_prod)
                let numerator = d_prev.checked_mul(
                    d_prod
                        .checked_mul(n_coins.into())?
                        .checked_add(leverage.into())?,
                )?;
                let denominator = d_prev
                    .checked_mul(ann.checked_sub(1)?.into())?
                    .checked_add(d_prod.checked_mul((n_coins + 1).into())?)?;
                d = numerator.checked_div(denominator)?;

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
    pub fn compute_lp_amount_for_deposit_ex(
        &self,
        deposit_c_amounts: Vec<Balance>,
        current_c_amounts: Vec<Balance>,
        pool_token_supply: Balance,
        fees: &Fees,
    ) -> Option<Balance> {
        let n_coins = current_c_amounts.len();
        // Initial invariant
        let d_0 = self.compute_d_ex(&current_c_amounts)?;
        let old_balances = current_c_amounts;

        let mut new_balances = vec![0_u128; n_coins];
        for (index, value) in deposit_c_amounts.iter().enumerate() {
            new_balances[index].checked_add(*value)?;
        }

        // Invariant after change
        let d_1 = self.compute_d_ex(&new_balances)?;
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
                let fee = fees.normalized_trade_fee(n_coins as u32, difference);
                new_balances[i] = new_balances[i].checked_sub(fee)?;
            }

            let d_2 = self.compute_d_ex(&new_balances)?;

            Some(
                U256::from(pool_token_supply)
                    .checked_mul(d_2.checked_sub(d_0)?)?
                    .checked_div(d_0)?
                    .as_u128(),
            )
        }
    }

    /// Compute amount of swap out token 'y' with token 'x' amount change to x_c_amount
    pub fn compute_y_ex(
        &self, 
        x_c_amount: Balance, 
        current_c_amounts: Vec<Balance>, 
        index_x: usize, 
        index_y: usize, 
    ) -> Option<U256> {
        let n_coins = current_c_amounts.len();
        let amp_factor = self.compute_amp_factor()?;
        let ann = amp_factor.checked_mul(n_coins as u128)?;
        // invariant
        let d = self.compute_d_ex(&current_c_amounts)?;
        let mut s_ = x_c_amount;
        let mut c = d.checked_mul(d)?.checked_div(x_c_amount.into())?;
        for (idx, c_amount) in current_c_amounts.iter().enumerate() {
            if idx != index_x && idx != index_y {
                s_ += *c_amount;
                c = c.checked_mul(d)?
                    .checked_div((*c_amount).into())?;
            }
        }
        c = c
            .checked_mul(d)?
            .checked_div(ann.checked_mul((n_coins as u128).checked_pow(n_coins as u32)?.into())?.into())?;

        let b = d.checked_div(ann.into())?.checked_add(s_.into())?; // d will be subtracted later

        // Solve for y by approximating: y**2 + b*y = c
        let mut y_prev: U256;
        let mut y = d;
        for _ in 0..256 {
            y_prev = y;
            // $ y_{k+1} = \frac{y_k^2 + c}{2y_k + b - D} $
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


    /// given token_out user want get and total tokens in pool and lp token supply,
    /// return <lp_amount_to_burn, admin_fees_of_each_token_in_c_amount>
    /// all amounts are in c_amount (comparable amount)
    pub fn compute_lp_amount_for_withdraw(
        &self,
        withdraw_c_amounts: Vec<Balance>,
        current_c_amounts: Vec<Balance>,
        pool_token_supply: Balance,
        fees: &Fees,
    ) -> Option<(Balance, Vec<Balance>)> {
        let n_coins = current_c_amounts.len();
        let mut admin_fees = vec![0_u128; n_coins];
        // Initial invariant, D0
        let d_0 = self.compute_d_ex(&current_c_amounts)?;
        let old_balances = current_c_amounts;

        // invariant after withdraw, D1
        let mut new_balances = vec![0_u128; n_coins];
        for (index, value) in withdraw_c_amounts.iter().enumerate() {
            new_balances[index].checked_sub(*value)?;
        }
        let d_1 = self.compute_d_ex(&new_balances)?;

        // compare ideal token portions from D1 with withdraws, to calculate diff fee.
        if d_1 >= d_0 {
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
                let fee = fees.normalized_trade_fee(n_coins as u32, difference);
                admin_fees[i] = fees.admin_trade_fee(fee);
                // new_balance is for calculation D2, the one with fee charged
                new_balances[i] = new_balances[i].checked_sub(fee)?;
            }

            let d_2 = self.compute_d_ex(&new_balances)?;

            let shares_out = U256::from(pool_token_supply)
                .checked_mul(d_0.checked_sub(d_2)?)?
                .checked_div(d_0)?
                .as_u128();

            Some((shares_out, admin_fees))
        }

    }


    /// Compute SwapResult after an exchange
    pub fn swap_to_ex(
        &self,
        index_x: usize,
        x_c_amount: Balance,
        index_y: usize,
        current_c_amounts: Vec<Balance>,
        fees: &Fees,
    ) -> Option<SwapResult> {
        let y = self.compute_y_ex(
            x_c_amount, 
            current_c_amounts.clone(),
            index_x,
            index_y,
        ).unwrap().as_u128();

        let dy = current_c_amounts[index_y].checked_sub(y)?;
        let dy_fee = fees.trade_fee(dy);
        let admin_fee = fees.admin_trade_fee(dy_fee);

        let amount_swapped = dy.checked_sub(dy_fee)?;
        let new_destination_amount = current_c_amounts[index_y]
            .checked_sub(amount_swapped)?
            .checked_sub(admin_fee)?;
        let new_source_amount = current_c_amounts[index_x].checked_add(x_c_amount)?;

        Some(SwapResult {
            new_source_amount,
            new_destination_amount,
            amount_swapped,
            admin_fee,
            fee: dy_fee,
        })
    }
}
