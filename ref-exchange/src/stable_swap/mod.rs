use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::LookupMap;
use near_sdk::json_types::ValidAccountId;
use near_sdk::{env, AccountId, Balance, Timestamp};

use crate::admin_fee::AdminFees;
use crate::errors::*;
use crate::stable_swap::math::{
    Fees, StableSwap, SwapResult, MAX_AMP, MAX_AMP_CHANGE, MIN_AMP, MIN_RAMP_DURATION,
};
use crate::utils::{add_to_collection, SwapVolume, FEE_DIVISOR, U256, u128_ratio};
use crate::StorageKey;

mod math;

pub const MIN_DECIMAL: u8 = 1;
pub const MAX_DECIMAL: u8 = 24;
pub const TARGET_DECIMAL: u8 = 18;
pub const MIN_RESERVE: u128 = 1 * 10u128.pow(TARGET_DECIMAL as u32) / 1_000_u128;

#[derive(BorshSerialize, BorshDeserialize)]
pub struct StableSwapPool {
    /// List of tokens in the pool.
    pub token_account_ids: Vec<AccountId>,
    /// Each decimals for tokens in the pool
    pub token_decimals: Vec<u8>,
    /// token amounts in comparable decimal.
    pub c_amounts: Vec<Balance>,
    /// Volumes accumulated by this pool.
    pub volumes: Vec<SwapVolume>,
    /// Fee charged for swap (gets divided by FEE_DIVISOR).
    pub total_fee: u32,
    /// Shares of the pool by liquidity providers.
    pub shares: LookupMap<AccountId, Balance>,
    /// Total number of shares.
    pub shares_total_supply: Balance,
    /// Initial amplification coefficient.
    pub init_amp_factor: u128,
    /// Target for ramping up amplification coefficient.
    pub target_amp_factor: u128,
    /// Initial amplification time.
    pub init_amp_time: Timestamp,
    /// Stop ramp up amplification time.
    pub stop_amp_time: Timestamp,
}

impl StableSwapPool {
    pub fn new(
        id: u32,
        token_account_ids: Vec<ValidAccountId>,
        token_decimals: Vec<u8>,
        amp_factor: u128,
        total_fee: u32,
    ) -> Self {
        for decimal in token_decimals.clone().into_iter() {
            assert!(decimal <= MAX_DECIMAL, "{}", ERR60_DECIMAL_ILLEGAL);
            assert!(decimal >= MIN_DECIMAL, "{}", ERR60_DECIMAL_ILLEGAL);
        }
        assert!(
            amp_factor >= MIN_AMP && amp_factor <= MAX_AMP,
            "{}",
            ERR61_AMP_ILLEGAL
        );
        assert!(total_fee < FEE_DIVISOR, "{}", ERR62_FEE_ILLEGAL);
        Self {
            token_account_ids: token_account_ids.iter().map(|a| a.clone().into()).collect(),
            token_decimals,
            c_amounts: vec![0u128; token_account_ids.len()],
            volumes: vec![SwapVolume::default(); token_account_ids.len()],
            total_fee,
            shares: LookupMap::new(StorageKey::Shares { pool_id: id }),
            shares_total_supply: 0,
            init_amp_factor: amp_factor,
            target_amp_factor: amp_factor,
            init_amp_time: 0,
            stop_amp_time: 0,
        }
    }

    pub fn modify_total_fee(&mut self, total_fee: u32) {
        self.total_fee = total_fee;
    }

    pub fn get_amounts(&self) ->Vec<u128> {
        let mut amounts = self.c_amounts.clone();
        for (index, value) in self.token_decimals.iter().enumerate() {
            if *value <= TARGET_DECIMAL {
                let factor = 10_u128
                    .checked_pow((TARGET_DECIMAL - value) as u32)
                    .unwrap();
                amounts[index] = amounts[index].checked_div(factor).unwrap();
            } else {
                let factor = 10_u128
                    .checked_pow((value - TARGET_DECIMAL) as u32)
                    .unwrap();
                amounts[index] = amounts[index].checked_mul(factor).unwrap();
            }
        }
        amounts
    }

    fn amounts_to_c_amounts(&self, amounts: &Vec<u128>) ->Vec<u128> {
        let mut c_amounts = amounts.clone();
        for (index, value) in self.token_decimals.iter().enumerate() {
            if *value <= TARGET_DECIMAL {
                let factor = 10_u128
                    .checked_pow((TARGET_DECIMAL - value) as u32)
                    .unwrap();
                c_amounts[index] = c_amounts[index].checked_mul(factor).unwrap();
            } else {
                let factor = 10_u128
                    .checked_pow((value - TARGET_DECIMAL) as u32)
                    .unwrap();
                c_amounts[index] = c_amounts[index].checked_div(factor).unwrap();
            }
        }
        c_amounts
    }

    fn amount_to_c_amount(&self, amount: u128, index: usize) -> u128 {
        let value = self.token_decimals.get(index).unwrap();
        if *value <= TARGET_DECIMAL {
            let factor = 10_u128
                .checked_pow((TARGET_DECIMAL - value) as u32)
                .unwrap();
            amount.checked_mul(factor).unwrap()
        } else {
            let factor = 10_u128
                .checked_pow((value - TARGET_DECIMAL) as u32)
                .unwrap();
            amount.checked_div(factor).unwrap()
        }
    }

    fn c_amount_to_amount(&self, c_amount: u128, index: usize) -> u128 {
        let value = self.token_decimals.get(index).unwrap();
        if *value <= TARGET_DECIMAL {
            let factor = 10_u128
                .checked_pow((TARGET_DECIMAL - value) as u32)
                .unwrap();
            c_amount.checked_div(factor).unwrap()
        } else {
            let factor = 10_u128
                .checked_pow((value - TARGET_DECIMAL) as u32)
                .unwrap();
            c_amount.checked_mul(factor).unwrap()
        }
    }

    fn assert_min_reserve(&self, balance: u128) {
        assert!(
            balance >= MIN_RESERVE,
            "{}",
            ERR69_MIN_RESERVE
        );
    }

    pub fn get_amp(&self) -> u64 {
        if let Some(amp) = self.get_invariant().compute_amp_factor() {
            amp as u64
        } else {
            0
        }
    }

    fn get_invariant(&self) -> StableSwap {
        StableSwap::new(
            self.init_amp_factor,
            self.target_amp_factor,
            env::block_timestamp(),
            self.init_amp_time,
            self.stop_amp_time,
        )
    }

    /// Returns token index for given token account_id.
    fn token_index(&self, token_id: &AccountId) -> usize {
        self.token_account_ids
            .iter()
            .position(|id| id == token_id)
            .expect(ERR63_MISSING_TOKEN)
    }

    /// Returns given pool's total fee.
    pub fn get_fee(&self) -> u32 {
        self.total_fee
    }

    /// Returns volumes of the given pool.
    pub fn get_volumes(&self) -> Vec<SwapVolume> {
        self.volumes.clone()
    }

    /// Get per lp token price, with 1e8 precision
    pub fn get_share_price(&self) -> u128 {

        let sum_token = self.c_amounts.iter().sum::<u128>();

        U256::from(sum_token)
            .checked_mul(100000000.into())
            .unwrap()
            .checked_div(self.shares_total_supply.into())
            .unwrap_or(100000000.into())
            .as_u128()
    }

    /// caculate mint share and related fee for adding liquidity
    /// return (share, fee_part)
    fn calc_add_liquidity(
        &self, 
        amounts: &Vec<Balance>, 
        fees: &AdminFees,
    ) -> (Balance, Balance) {
        let invariant = self.get_invariant();

        // make amounts into comparable-amounts
        let c_amounts = self.amounts_to_c_amounts(amounts);

        if self.shares_total_supply == 0 {
            // Bootstrapping the pool, request providing all non-zero balances,
            // and all fee free.
            for c_amount in &c_amounts {
                assert!(*c_amount > 0, "{}", ERR65_INIT_TOKEN_BALANCE);
            }
            (
                invariant
                    .compute_d(&c_amounts)
                    .expect(ERR66_INVARIANT_CALC_ERR)
                    .as_u128(),
                0,
            )
        } else {
            // Subsequent add liquidity will charge fee according to difference with ideal balance portions
            invariant
                .compute_lp_amount_for_deposit(
                    &c_amounts,
                    &self.c_amounts,
                    self.shares_total_supply,
                    &Fees::new(self.total_fee, &fees),
                )
                .expect(ERR67_LPSHARE_CALC_ERR)
        }
    }

    /// Add liquidity into the pool.
    /// Allows to add liquidity of a subset of tokens,
    /// by set other tokens balance into 0.
    pub fn add_liquidity(
        &mut self,
        sender_id: &AccountId,
        amounts: &Vec<Balance>,
        min_shares: Balance,
        fees: &AdminFees,
        is_view: bool
    ) -> Balance {
        let n_coins = self.token_account_ids.len();
        assert_eq!(amounts.len(), n_coins, "{}", ERR64_TOKENS_COUNT_ILLEGAL);

        let (new_shares, fee_part) = self.calc_add_liquidity(amounts, fees);
        //slippage check on the LP tokens.
        assert!(new_shares >= min_shares, "{}", ERR68_SLIPPAGE);

        for i in 0..n_coins {
            self.c_amounts[i] = self.c_amounts[i].checked_add(self.amount_to_c_amount(amounts[i], i)).unwrap();
        }
        self.mint_shares(sender_id, new_shares, is_view);
        if !is_view {
            env::log(
                format!(
                    "Mint {} shares for {}, fee is {} shares",
                    new_shares, sender_id, fee_part,
                )
                .as_bytes(),
            );
        }

        if fee_part > 0 {
            let admin_share = u128_ratio(fee_part, fees.admin_fee_bps as u128, FEE_DIVISOR as u128);
            let (mut referral_share, referral) = fees.calc_referral_share(admin_share);

            if referral_share > 0 && self.shares.get(&referral).is_none() {
                referral_share = 0;
            }
            self.mint_shares(&referral, referral_share, is_view);
            self.mint_shares(&fees.exchange_id, admin_share - referral_share, is_view);

            if !is_view {
                if referral_share > 0 {
                    env::log(
                        format!(
                            "Exchange {} got {} shares, Referral {} got {} shares, from add_liquidity", 
                            &fees.exchange_id, admin_share - referral_share, referral, referral_share
                        ).as_bytes(),
                    );
                } else {
                    env::log(
                        format!(
                            "Exchange {} got {} shares, No referral fee, from add_liquidity", 
                            &fees.exchange_id, admin_share
                        ).as_bytes(),
                    );
                }
            }
        }

        new_shares
    }

    /// balanced removal of liquidity would be free of charge.
    pub fn remove_liquidity_by_shares(
        &mut self,
        sender_id: &AccountId,
        shares: Balance,
        min_amounts: Vec<Balance>,
        is_view: bool
    ) -> Vec<Balance> {
        let n_coins = self.token_account_ids.len();
        assert_eq!(min_amounts.len(), n_coins, "{}", ERR64_TOKENS_COUNT_ILLEGAL);
        if !is_view {
            let prev_shares_amount = self.shares.get(&sender_id).expect(ERR13_LP_NOT_REGISTERED);
            assert!(
                prev_shares_amount >= shares,
                "{}",
                ERR34_INSUFFICIENT_LP_SHARES
            );
            self.burn_shares(&sender_id, prev_shares_amount, shares);
        }
        let mut result = vec![0u128; n_coins];

        for i in 0..n_coins {
            result[i] = U256::from(self.c_amounts[i])
                .checked_mul(shares.into())
                .unwrap()
                .checked_div(self.shares_total_supply.into())
                .unwrap()
                .as_u128();
            self.c_amounts[i] = self.c_amounts[i].checked_sub(result[i]).unwrap();
            self.assert_min_reserve(self.c_amounts[i]);
            result[i] = self.c_amount_to_amount(result[i], i);
            assert!(result[i] >= min_amounts[i], "{}", ERR68_SLIPPAGE);
        }

        self.shares_total_supply -= shares;

        if !is_view {
            env::log(
                format!(
                    "LP {} remove {} shares to gain tokens {:?}",
                    sender_id, shares, result
                )
                .as_bytes(),
            );
        }

        result
    }

    /// Remove liquidity from the pool by fixed tokens-out,
    /// allows to remove liquidity of a subset of tokens, by providing 0 in `amounts`.
    /// Fee will be charged according to diff between ideal token portions.
    pub fn remove_liquidity_by_tokens(
        &mut self,
        sender_id: &AccountId,
        amounts: Vec<Balance>,
        max_burn_shares: Balance,
        fees: &AdminFees,
        is_view: bool
    ) -> Balance {
        let n_coins = self.token_account_ids.len();
        assert_eq!(amounts.len(), n_coins, "{}", ERR64_TOKENS_COUNT_ILLEGAL);

        // make amounts into comparable-amounts
        let c_amounts = self.amounts_to_c_amounts(&amounts);
        for i in 0..n_coins {
            self.assert_min_reserve(self.c_amounts[i].checked_sub(c_amounts[i]).unwrap_or(0));
        }

        let invariant = self.get_invariant();
        let trade_fee = Fees::new(self.total_fee, &fees);

        let (burn_shares, fee_part) = invariant
            .compute_lp_amount_for_withdraw(
                &c_amounts,
                &self.c_amounts,
                self.shares_total_supply,
                &trade_fee,
            )
            .expect(ERR67_LPSHARE_CALC_ERR);

        if !is_view {
            let prev_shares_amount = self.shares.get(&sender_id).expect(ERR13_LP_NOT_REGISTERED);
            assert!(
                burn_shares <= prev_shares_amount,
                "{}",
                ERR34_INSUFFICIENT_LP_SHARES
            );
            assert!(burn_shares <= max_burn_shares, "{}", ERR68_SLIPPAGE);
            self.burn_shares(&sender_id, prev_shares_amount, burn_shares);
        }

        for i in 0..n_coins {
            self.c_amounts[i] = self.c_amounts[i].checked_sub(c_amounts[i]).unwrap();
            self.assert_min_reserve(self.c_amounts[i]);
        }
        self.shares_total_supply -= burn_shares;

        if !is_view {
            env::log(
                format!(
                    "LP {} removed {} shares by given tokens, and fee is {} shares",
                    sender_id, burn_shares, fee_part
                )
                .as_bytes(),
            );
        }

        if fee_part > 0 {
            let admin_share = u128_ratio(fee_part, fees.admin_fee_bps as u128, FEE_DIVISOR as u128);
            let (mut referral_share, referral) = fees.calc_referral_share(admin_share);

            if referral_share > 0 && self.shares.get(&referral).is_none() {
                referral_share = 0;
            }
            self.mint_shares(&referral, referral_share, is_view);
            self.mint_shares(&fees.exchange_id, admin_share - referral_share, is_view);

            if !is_view {
                if referral_share > 0 {
                    env::log(
                        format!(
                            "Exchange {} got {} shares, Referral {} got {} shares, from remove_liquidity_by_tokens", 
                            &fees.exchange_id, admin_share - referral_share, referral, referral_share
                        ).as_bytes(),
                    );
                } else {
                    env::log(
                        format!(
                            "Exchange {} got {} shares, No referral fee, from remove_liquidity_by_tokens", 
                            &fees.exchange_id, admin_share
                        ).as_bytes(),
                    );
                }
            }
        }

        burn_shares
    }

    /// Returns number of tokens in outcome, given amount.
    /// Tokens are provided as indexes into token list for given pool.
    /// All tokens are comparable tokens
    fn internal_get_return(
        &self,
        token_in: usize,
        amount_in: Balance,
        token_out: usize,
        fees: &AdminFees,
    ) -> SwapResult {
        // make amounts into comparable-amounts
        let c_amount_in = self.amount_to_c_amount(amount_in, token_in);

        self.get_invariant()
            .swap_to(
                token_in,
                c_amount_in,
                token_out,
                &self.c_amounts,
                &Fees::new(self.total_fee, &fees),
            )
            .expect(ERR70_SWAP_OUT_CALC_ERR)

    }

    /// Swap `token_amount_in` of `token_in` token into `token_out` and return how much was received.
    /// Assuming that `token_amount_in` was already received from `sender_id`.
    pub fn swap(
        &mut self,
        token_in: &AccountId,
        amount_in: Balance,
        token_out: &AccountId,
        min_amount_out: Balance,
        fees: &AdminFees,
        is_view: bool
    ) -> Balance {
        assert_ne!(token_in, token_out, "{}", ERR71_SWAP_DUP_TOKENS);
        let in_idx = self.token_index(token_in);
        let out_idx = self.token_index(token_out);
        let result = self.internal_get_return(in_idx, amount_in, out_idx, &fees);
        let amount_swapped = self.c_amount_to_amount(result.amount_swapped, out_idx);
        assert!(
            amount_swapped >= min_amount_out,
            "{}",
            ERR68_SLIPPAGE
        );
        if !is_view {
            env::log(
                format!(
                    "Swapped {} {} for {} {}, total fee {}, admin fee {}",
                    amount_in, token_in, amount_swapped, token_out, 
                    self.c_amount_to_amount(result.fee, out_idx), 
                    self.c_amount_to_amount(result.admin_fee, out_idx)
                )
                .as_bytes(),
            );
        }

        self.c_amounts[in_idx] = result.new_source_amount;
        self.c_amounts[out_idx] = result.new_destination_amount;
        self.assert_min_reserve(self.c_amounts[out_idx]);

        // Due to the u128 overflow issue in volumes, we have switched to using SwapVolumeU256 for recording.
        // Keeping track of volume per each input traded separately.
        // self.volumes[in_idx].input.0 = self.volumes[in_idx].input.0.wrapping_add(amount_in);
        // self.volumes[out_idx].output.0 = self.volumes[out_idx].output.0.wrapping_add(amount_swapped);

        // handle admin fee.
        if fees.admin_fee_bps > 0 && result.admin_fee > 0 {
            let (exchange_share, referral_share) = if let Some((referral_id, referral_fee)) = &fees.referral_info {
                if self.shares.contains_key(referral_id)
                {
                    self.distribute_admin_fee(&fees.exchange_id, referral_id, *referral_fee, out_idx, result.admin_fee, is_view)
                } else {
                    self.distribute_admin_fee(&fees.exchange_id, referral_id, 0, out_idx, result.admin_fee, is_view)
                }
            } else {
                self.distribute_admin_fee(&fees.exchange_id, &fees.exchange_id, 0, out_idx, result.admin_fee, is_view)
            };
            if !is_view {
                if referral_share > 0 {
                    env::log(
                        format!(
                            "Exchange {} got {} shares, Referral {} got {} shares",
                            &fees.exchange_id, exchange_share, &fees.referral_info.as_ref().unwrap().0, referral_share,
                        )
                        .as_bytes(),
                    );
                } else {
                    env::log(
                        format!(
                            "Exchange {} got {} shares, No referral fee",
                            &fees.exchange_id, exchange_share,
                        )
                        .as_bytes(),
                    );
                }
            }
        }

        amount_swapped
    }

    /// convert admin_fee into shares without any fee.
    /// return share minted this time for the admin/referrer.
    fn distribute_admin_fee(
        &mut self,
        exchange_id: &AccountId,
        referral_id: &AccountId,
        referral_fee_bps: u32,
        token_id: usize,
        c_amount: Balance,
        is_view: bool
    ) -> (Balance, Balance) {
        let invariant = self.get_invariant();

        let mut c_amounts = vec![0_u128; self.c_amounts.len()];
        c_amounts[token_id] = c_amount;

        let (new_shares, _) = invariant
            .compute_lp_amount_for_deposit(
                &c_amounts,
                &self.c_amounts,
                self.shares_total_supply,
                &Fees::zero(),
            )
            .expect(ERR67_LPSHARE_CALC_ERR);
        self.c_amounts[token_id] += c_amount;

        let referral_share = if referral_fee_bps > 0 {
            u128_ratio(new_shares, referral_fee_bps as u128, FEE_DIVISOR as u128)
        } else {
            0
        };

        self.mint_shares(referral_id, referral_share, is_view);
        self.mint_shares(exchange_id, new_shares - referral_share, is_view);

        (new_shares - referral_share, referral_share)
    }

    /// Mint new shares for given user.
    fn mint_shares(&mut self, account_id: &AccountId, shares: Balance, is_view: bool) {
        if shares == 0 {
            return;
        }
        self.shares_total_supply += shares;
        if !is_view {
            add_to_collection(&mut self.shares, &account_id, shares);
        }
    }

    /// Burn shares from given user's balance.
    fn burn_shares(
        &mut self,
        account_id: &AccountId,
        prev_shares_amount: Balance,
        shares: Balance,
    ) {
        if shares == 0 {
            return;
        }
        // Never remove shares from storage to allow to bring it back without extra storage deposit.
        self.shares.insert(&account_id, &(prev_shares_amount - shares));
    }

    /// See if the given account has been registered as a LP
    pub fn share_has_registered(&self, account_id: &AccountId) -> bool {
        self.shares.contains_key(account_id)
    }

    /// Register given account with 0 balance in shares.
    /// Storage payment should be checked by caller.
    pub fn share_register(&mut self, account_id: &AccountId) {
        if self.shares.contains_key(account_id) {
            env::panic(ERR14_LP_ALREADY_REGISTERED.as_bytes());
        }
        self.shares.insert(account_id, &0);
    }

    /// Unregister account with shares balance of 0.
    /// The storage should be refunded to the user.
    pub fn share_unregister(&mut self, account_id: &AccountId) {
        let shares = self.shares.remove(account_id);
        assert!(shares.expect(ERR13_LP_NOT_REGISTERED) == 0, "{}", ERR19_NONZERO_LP_SHARES);
    }

    /// Transfers shares from predecessor to receiver.
    pub fn share_transfer(&mut self, sender_id: &AccountId, receiver_id: &AccountId, amount: u128) {
        let balance = self.shares.get(&sender_id).expect(ERR13_LP_NOT_REGISTERED);
        if let Some(new_balance) = balance.checked_sub(amount) {
            self.shares.insert(&sender_id, &new_balance);
        } else {
            env::panic(ERR34_INSUFFICIENT_LP_SHARES.as_bytes());
        }
        let balance_out = self
            .shares
            .get(&receiver_id)
            .expect(ERR13_LP_NOT_REGISTERED);
        self.shares.insert(&receiver_id, &(balance_out + amount));
    }

    /// Returns balance of shares for given user.
    pub fn share_balance_of(&self, account_id: &AccountId) -> Balance {
        self.shares.get(account_id).unwrap_or_default()
    }

    /// Returns total number of shares in this pool.
    pub fn share_total_balance(&self) -> Balance {
        self.shares_total_supply
    }

    /// Returns list of tokens in this pool.
    pub fn tokens(&self) -> &[AccountId] {
        &self.token_account_ids
    }

    /// [Admin function] increase the amplification factor.
    pub fn ramp_amplification(&mut self, future_amp_factor: u128, future_amp_time: Timestamp) {
        let current_time = env::block_timestamp();
        assert!(
            current_time >= self.init_amp_time + MIN_RAMP_DURATION,
            "{}",
            ERR81_AMP_IN_LOCK
        );
        assert!(
            future_amp_time >= current_time + MIN_RAMP_DURATION,
            "{}",
            ERR82_INSUFFICIENT_RAMP_TIME
        );
        let invariant = StableSwap::new(
            self.init_amp_factor,
            self.target_amp_factor,
            current_time,
            self.init_amp_time,
            self.stop_amp_time,
        );
        let amp_factor = invariant
            .compute_amp_factor()
            .expect(ERR66_INVARIANT_CALC_ERR);
        assert!(
            future_amp_factor > 0 && future_amp_factor < MAX_AMP,
            "{}",
            ERR83_INVALID_AMP_FACTOR
        );
        assert!(
            (future_amp_factor >= amp_factor && future_amp_factor <= amp_factor * MAX_AMP_CHANGE)
                || (future_amp_factor < amp_factor
                    && future_amp_factor * MAX_AMP_CHANGE >= amp_factor),
            "{}",
            ERR84_AMP_LARGE_CHANGE
        );
        self.init_amp_factor = amp_factor;
        self.init_amp_time = current_time;
        self.target_amp_factor = future_amp_factor;
        self.stop_amp_time = future_amp_time;
    }

    /// [Admin function] Stop increase of amplification factor.
    pub fn stop_ramp_amplification(&mut self) {
        let current_time = env::block_timestamp();
        let invariant = StableSwap::new(
            self.init_amp_factor,
            self.target_amp_factor,
            current_time,
            self.init_amp_time,
            self.stop_amp_time,
        );
        let amp_factor = invariant
            .compute_amp_factor()
            .expect(ERR65_INIT_TOKEN_BALANCE);
        self.init_amp_factor = amp_factor;
        self.target_amp_factor = amp_factor;
        self.init_amp_time = current_time;
        self.stop_amp_time = current_time;
    }
}

#[cfg(test)]
mod tests {
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::{testing_env, MockedBlockchain};
    use std::convert::TryInto;

    use super::*;

    fn swap(
        pool: &mut StableSwapPool,
        token_in: usize,
        amount_in: Balance,
        token_out: usize,
    ) -> Balance {
        pool.swap(
            accounts(token_in).as_ref(),
            amount_in,
            accounts(token_out).as_ref(),
            0,
            &AdminFees::zero(),
            false
        )
    }

    #[test]
    fn test_stable_julia_01() {
        let mut context = VMContextBuilder::new();
        testing_env!(context.predecessor_account_id(accounts(0)).build());
        let fees = AdminFees::zero();
        let mut pool = StableSwapPool::new(0, vec![accounts(1), accounts(2)], vec![6, 6], 1000, 0);
        assert_eq!(
            pool.tokens(),
            vec![accounts(1).to_string(), accounts(2).to_string()]
        );

        let mut amounts = vec![100000000000, 100000000000];
        let _ = pool.add_liquidity(accounts(0).as_ref(), &mut amounts, 1, &fees, false);

        let out = swap(&mut pool, 1, 10000000000, 2);
        assert_eq!(out, 9999495232);
        assert_eq!(pool.c_amounts, vec![110000000000000000000000, 90000504767247802010005]);
    }

    #[test]
    fn test_stable_romeo_01() {
        let mut context = VMContextBuilder::new();
        testing_env!(context.predecessor_account_id(accounts(0)).build());
        let fees = AdminFees::zero();
        let mut pool = StableSwapPool::new(0, vec![accounts(1), accounts(2)], vec![18, 24], 1000, 0);
        assert_eq!(
            pool.tokens(),
            vec![accounts(1).to_string(), accounts(2).to_string()]
        );

        let mut amounts = vec![100000000000000000000000, 100000000000000000000000000000];
        let _ = pool.add_liquidity(accounts(0).as_ref(), &mut amounts, 1, &fees, false);

        let out = swap(&mut pool, 1, 10000000000000000000000, 2);
        assert_eq!(out, 9999495232752197989995000000);
        assert_eq!(pool.c_amounts, vec![110000000000000000000000, 90000504767247802010005]);
    }

    #[test]
    fn test_stable_julia_02() {
        let mut context = VMContextBuilder::new();
        testing_env!(context.predecessor_account_id(accounts(0)).build());
        let fees = AdminFees::zero();
        let mut pool = StableSwapPool::new(0, vec![accounts(1), accounts(2)], vec![6, 6], 1000, 0);
        assert_eq!(
            pool.tokens(),
            vec![accounts(1).to_string(), accounts(2).to_string()]
        );

        let mut amounts = vec![100000000000, 100000000000];
        let _ = pool.add_liquidity(accounts(0).as_ref(), &mut amounts, 1, &fees, false);

        let out = swap(&mut pool, 1, 0, 2);
        assert_eq!(out, 0);
        assert_eq!(pool.c_amounts, vec![100000000000000000000000, 100000000000000000000000]);
    }

    #[test]
    fn test_stable_romeo_02() {
        let mut context = VMContextBuilder::new();
        testing_env!(context.predecessor_account_id(accounts(0)).build());
        let fees = AdminFees::zero();
        let mut pool = StableSwapPool::new(0, vec![accounts(1), accounts(2)], vec![18, 24], 1000, 0);
        assert_eq!(
            pool.tokens(),
            vec![accounts(1).to_string(), accounts(2).to_string()]
        );

        let mut amounts = vec![100000000000000000000000, 100000000000000000000000000000];
        let _ = pool.add_liquidity(accounts(0).as_ref(), &mut amounts, 1, &fees, false);

        let out = swap(&mut pool, 1, 0, 2);
        assert_eq!(out, 0);
        assert_eq!(pool.c_amounts, vec![100000000000000000000000, 100000000000000000000000]);
    }

    #[test]
    fn test_stable_julia_03() {
        let mut context = VMContextBuilder::new();
        testing_env!(context.predecessor_account_id(accounts(0)).build());
        let fees = AdminFees::zero();
        let mut pool = StableSwapPool::new(0, vec![accounts(1), accounts(2)], vec![6, 6], 1000, 0);
        assert_eq!(
            pool.tokens(),
            vec![accounts(1).to_string(), accounts(2).to_string()]
        );

        let mut amounts = vec![100000000000, 100000000000];
        let _ = pool.add_liquidity(accounts(0).as_ref(), &mut amounts, 1, &fees, false);

        let out = swap(&mut pool, 1, 1, 2);
        assert_eq!(out, 0);
        assert_eq!(pool.c_amounts, vec![100000000001000000000000, 99999999999000000000001]);
    }

    #[test]
    fn test_stable_romeo_03() {
        let mut context = VMContextBuilder::new();
        testing_env!(context.predecessor_account_id(accounts(0)).build());
        let fees = AdminFees::zero();
        let mut pool = StableSwapPool::new(0, vec![accounts(1), accounts(2)], vec![18, 24], 1000, 0);
        assert_eq!(
            pool.tokens(),
            vec![accounts(1).to_string(), accounts(2).to_string()]
        );

        let mut amounts = vec![100000000000000000000000, 100000000000000000000000000000];
        let _ = pool.add_liquidity(accounts(0).as_ref(), &mut amounts, 1, &fees, false);

        let out = swap(&mut pool, 1, 1, 2);
        assert_eq!(out, 0);
        assert_eq!(pool.c_amounts, vec![100000000000000000000001, 100000000000000000000000]);
    }

    #[test]
    fn test_stable_julia_04() {
        let mut context = VMContextBuilder::new();
        testing_env!(context.predecessor_account_id(accounts(0)).build());
        let fees = AdminFees::zero();
        let mut pool = StableSwapPool::new(0, vec![accounts(1), accounts(2)], vec![6, 6], 1000, 0);
        assert_eq!(
            pool.tokens(),
            vec![accounts(1).to_string(), accounts(2).to_string()]
        );

        let mut amounts = vec![100000000000, 100000000000];
        let _ = pool.add_liquidity(accounts(0).as_ref(), &mut amounts, 1, &fees, false);

        let out = swap(&mut pool, 1, 100000000000, 2);
        assert_eq!(out, 98443663539);
        assert_eq!(pool.c_amounts, vec![200000000000000000000000, 1556336460086846919344]);
    }

    #[test]
    fn test_stable_romeo_04() {
        let mut context = VMContextBuilder::new();
        testing_env!(context.predecessor_account_id(accounts(0)).build());
        let fees = AdminFees::zero();
        let mut pool = StableSwapPool::new(0, vec![accounts(1), accounts(2)], vec![18, 24], 1000, 0);
        assert_eq!(
            pool.tokens(),
            vec![accounts(1).to_string(), accounts(2).to_string()]
        );

        let mut amounts = vec![100000000000000000000000, 100000000000000000000000000000];
        let _ = pool.add_liquidity(accounts(0).as_ref(), &mut amounts, 1, &fees, false);

        let out = swap(&mut pool, 1, 100000000000000000000000, 2);
        assert_eq!(out, 98443663539913153080656000000);
        assert_eq!(pool.c_amounts, vec![200000000000000000000000, 1556336460086846919344]);
    }

    #[test]
    fn test_stable_julia_05() {
        let mut context = VMContextBuilder::new();
        testing_env!(context.predecessor_account_id(accounts(0)).build());
        let fees = AdminFees::zero();
        let mut pool = StableSwapPool::new(0, vec![accounts(1), accounts(2)], vec![6, 6], 1000, 0);
        assert_eq!(
            pool.tokens(),
            vec![accounts(1).to_string(), accounts(2).to_string()]
        );

        let mut amounts = vec![100000000000, 100000000000];
        let _ = pool.add_liquidity(accounts(0).as_ref(), &mut amounts, 1, &fees, false);

        let out = swap(&mut pool, 1, 99999000000, 2);
        assert_eq!(out, 98443167413);
        assert_eq!(pool.c_amounts, vec![199999000000000000000000, 1556832586795864493704]);
    }

    #[test]
    fn test_stable_romeo_05() {
        let mut context = VMContextBuilder::new();
        testing_env!(context.predecessor_account_id(accounts(0)).build());
        let fees = AdminFees::zero();
        let mut pool = StableSwapPool::new(0, vec![accounts(1), accounts(2)], vec![18, 24], 1000, 0);
        assert_eq!(
            pool.tokens(),
            vec![accounts(1).to_string(), accounts(2).to_string()]
        );

        let mut amounts = vec![100000000000000000000000, 100000000000000000000000000000];
        let _ = pool.add_liquidity(accounts(0).as_ref(), &mut amounts, 1, &fees, false);

        let out = swap(&mut pool, 1, 99999000000000000000000, 2);
        assert_eq!(out, 98443167413204135506296000000);
        assert_eq!(pool.c_amounts, vec![199999000000000000000000, 1556832586795864493704]);
    }

    #[test]
    fn test_min_reserve_by_shares() {
        let mut context = VMContextBuilder::new();
        testing_env!(context.predecessor_account_id(accounts(0)).build());
        let fees = AdminFees::zero();
        let mut pool = StableSwapPool::new(0, vec![accounts(1), accounts(2)], vec![18, 18], 1000, 0);
        assert_eq!(
            pool.tokens(),
            vec![accounts(1).to_string(), accounts(2).to_string()]
        );

        let mut amounts = vec![1000000000000000000, 1000000000000000000];
        let shares = pool.add_liquidity(accounts(0).as_ref(), &mut amounts, 1, &fees, false);
        pool.remove_liquidity_by_shares(accounts(0).as_ref(), shares - 2 * MIN_RESERVE, vec![0, 0], false);
        assert_eq!(vec![MIN_RESERVE, MIN_RESERVE], pool.c_amounts);
    }

    #[test]
    fn test_min_reserve_by_tokens() {
        let mut context = VMContextBuilder::new();
        testing_env!(context.predecessor_account_id(accounts(0)).build());
        let fees = AdminFees::zero();
        let mut pool = StableSwapPool::new(0, vec![accounts(1), accounts(2)], vec![18, 18], 1000, 0);
        assert_eq!(
            pool.tokens(),
            vec![accounts(1).to_string(), accounts(2).to_string()]
        );

        let mut amounts = vec![100000000000000000000000, 100000000000000000000000000000];
        let shares = pool.add_liquidity(accounts(0).as_ref(), &mut amounts, 1, &fees, false);
        pool.remove_liquidity_by_tokens(accounts(0).as_ref(), vec![amounts[0] - MIN_RESERVE, amounts[1] - MIN_RESERVE], shares, &AdminFees::zero(), false);
        assert_eq!(vec![MIN_RESERVE, MIN_RESERVE], pool.c_amounts);
    }

    #[test]
    fn test_stable_max() {
        let mut context = VMContextBuilder::new();
        testing_env!(context.predecessor_account_id(accounts(0)).build());
        let fees = AdminFees::zero();
        let mut pool = StableSwapPool::new(
            0, 
            vec![
                "aone.near".try_into().unwrap(),
                "atwo.near".try_into().unwrap(),
                "athree.near".try_into().unwrap(),
                "afour.near".try_into().unwrap(),
                "afive.near".try_into().unwrap(),
                "asix.near".try_into().unwrap(),
                "aseven.near".try_into().unwrap(),
                "aeight.near".try_into().unwrap(),
                "anine.near".try_into().unwrap(), 
            ], 
            vec![
                6, 
                6, 
                6, 
                6, 
                6, 
                6, 
                6, 
                6, 
                6,
            ], 
            1000, 
            0
        );
        assert_eq!(
            pool.tokens(),
            vec![
                "aone.near".to_string(),
                "atwo.near".to_string(),
                "athree.near".to_string(),
                "afour.near".to_string(),
                "afive.near".to_string(),
                "asix.near".to_string(),
                "aseven.near".to_string(),
                "aeight.near".to_string(),
                "anine.near".to_string(), 
            ]
        );

        let mut amounts = vec![
            100000000000_000000, 
            100000000000_000000, 
            100000000000_000000, 
            100000000000_000000, 
            100000000000_000000,
            100000000000_000000, 
            100000000000_000000, 
            100000000000_000000, 
            100000000000_000000,
        ];
        let share = pool.add_liquidity(accounts(0).as_ref(), &mut amounts, 1, &fees, false);
        assert_eq!(share, 900000000000_000000000000000000);
        let out = pool.swap(
            &String::from("aone.near"),
            99999000000,
            &String::from("atwo.near"),
            0,
            &AdminFees::zero(),
            false
        );
        assert_eq!(out, 99998999999);
    }

    #[test]
    fn test_stable_basics() {
        let mut context = VMContextBuilder::new();
        testing_env!(context.predecessor_account_id(accounts(0)).build());
        let fees = AdminFees::zero();
        let mut pool = StableSwapPool::new(0, vec![accounts(1), accounts(2)], vec![6, 6], 10000, 0);
        assert_eq!(
            pool.tokens(),
            vec![accounts(1).to_string(), accounts(2).to_string()]
        );

        let mut amounts = vec![5000000, 10000000];
        let _ = pool.add_liquidity(accounts(0).as_ref(), &mut amounts, 1, &fees, false);

        let out = swap(&mut pool, 1, 1000000, 2);
        assert_eq!(out, 1000031);
        assert_eq!(pool.c_amounts, vec![6000000000000000000, 8999968751649207661]);
        let out2 = swap(&mut pool, 2, out, 1);
        assert_eq!(out2, 999999); // due to precision difference.
        assert_eq!(pool.c_amounts, vec![5000000248340316023, 9999999751649207661]);

        // Add only one side of the capital.
        let mut amounts2 = vec![5000000, 0];
        let num_shares = pool.add_liquidity(accounts(0).as_ref(), &mut amounts2, 1, &fees, false);

        // Withdraw on same side of the capital.
        let shares_burned = pool.remove_liquidity_by_tokens(
            accounts(0).as_ref(),
            vec![5000000, 0],
            num_shares,
            &fees,
            false
        );
        assert_eq!(shares_burned, num_shares);

        // Add only one side of the capital, and withdraw by share
        let mut amounts2 = vec![5000000, 0];
        let num_shares = pool.add_liquidity(accounts(0).as_ref(), &mut amounts2, 1, &fees, false);

        let tokens = pool.remove_liquidity_by_shares(accounts(0).as_ref(), num_shares, vec![1, 1], false);
        assert_eq!(tokens[0], 2500023);
        assert_eq!(tokens[1], 2500023);

        // Add only one side of the capital, and withdraw from another side
        let mut amounts2 = vec![5000000, 0];
        let num_shares = pool.add_liquidity(accounts(0).as_ref(), &mut amounts2, 1, &fees, false);
        let shares_burned = pool.remove_liquidity_by_tokens(
            accounts(0).as_ref(),
            vec![0, 5000000 - 1200],
            num_shares,
            &fees,
            false
        );
        // as imbalance withdraw, will lose a little amount token
        assert!(shares_burned < num_shares);
    }

    /// Test everything with fees.
    #[test]
    fn test_stable_with_fees() {
        let mut context = VMContextBuilder::new();
        testing_env!(context.predecessor_account_id(accounts(0)).build());
        let mut pool =
            StableSwapPool::new(0, vec![accounts(1), accounts(2)], vec![6, 6], 10000, 2000);
        let mut amounts = vec![5000000, 10000000];
        let fees = AdminFees::new(1000); // 10% exchange fee

        let num_shares = pool.add_liquidity(accounts(0).as_ref(), &mut amounts, 1, &fees, false);

        let amount_out = pool.swap(
            accounts(1).as_ref(),
            1000000,
            accounts(2).as_ref(),
            1,
            &fees,
            false
        );
        println!("swap out: {}", amount_out);
        let tokens = pool.remove_liquidity_by_shares(accounts(0).as_ref(), num_shares/2, vec![1, 1], false);
        assert_eq!(tokens[0], 2996052);
        assert_eq!(tokens[1], 4593934);
    }

    /// Test that adding and then removing all of the liquidity leaves the pool empty and with no shares.
    #[test]
    #[should_panic(expected = "E69: pool reserved token balance less than MIN_RESERVE")]
    fn test_stable_add_transfer_remove_liquidity() {
        let mut context = VMContextBuilder::new();
        testing_env!(context.predecessor_account_id(accounts(0)).build());
        let mut pool = StableSwapPool::new(0, vec![accounts(1), accounts(2)], vec![6, 6], 10000, 0);
        let mut amounts = vec![5000000, 10000000];
        let fees = AdminFees::zero();
        let num_shares = pool.add_liquidity(accounts(0).as_ref(), &mut amounts, 1, &fees, false);
        assert_eq!(amounts, vec![5000000, 10000000]);
        assert!(num_shares > 1);
        assert_eq!(num_shares, pool.share_balance_of(accounts(0).as_ref()));
        assert_eq!(pool.share_total_balance(), num_shares);

        // Move shares to another account.
        pool.share_register(accounts(3).as_ref());
        pool.share_transfer(accounts(0).as_ref(), accounts(3).as_ref(), num_shares);
        assert_eq!(pool.share_balance_of(accounts(0).as_ref()), 0);
        assert_eq!(pool.share_balance_of(accounts(3).as_ref()), num_shares);
        assert_eq!(pool.share_total_balance(), num_shares);

        // Remove all liquidity.
        testing_env!(context.predecessor_account_id(accounts(3)).build());
        pool.remove_liquidity_by_shares(accounts(3).as_ref(), num_shares, vec![1, 1], false);
    }

    /// Test ramping up amplification factor, ramping it even more and then stopping.
    #[test]
    fn test_stable_ramp_amp() {
        let mut context = VMContextBuilder::new();
        testing_env!(context.predecessor_account_id(accounts(0)).build());
        let mut pool = StableSwapPool::new(0, vec![accounts(1), accounts(2)], vec![6, 6], 10000, 0);

        let start_ts = MIN_RAMP_DURATION + 1_000_000_000;
        testing_env!(context.block_timestamp(start_ts).build());
        pool.ramp_amplification(50000, start_ts + MIN_RAMP_DURATION * 10);
        testing_env!(context
            .block_timestamp(start_ts + MIN_RAMP_DURATION * 3)
            .build());
        pool.ramp_amplification(150000, start_ts + MIN_RAMP_DURATION * 20);
        testing_env!(context
            .block_timestamp(start_ts + MIN_RAMP_DURATION * 5)
            .build());
        pool.stop_ramp_amplification();
    }
}
