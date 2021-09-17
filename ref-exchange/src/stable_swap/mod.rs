use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::LookupMap;
use near_sdk::json_types::ValidAccountId;
use near_sdk::{env, AccountId, Balance, Timestamp};

use crate::errors::{ERR13_LP_NOT_REGISTERED, ERR14_LP_ALREADY_REGISTERED};
use crate::fees::SwapFees;
use crate::stable_swap::math::{
    Fees, StableSwap, SwapResult, MAX_AMP, MAX_AMP_CHANGE, MIN_AMP, MIN_RAMP_DURATION, N_COINS,
};
use crate::utils::{add_to_collection, SwapVolume, FEE_DIVISOR};
use crate::StorageKey;

mod math;

#[derive(BorshSerialize, BorshDeserialize)]
pub struct StableSwapPool {
    /// List of tokens in the pool.
    pub token_account_ids: Vec<AccountId>,
    /// How much NEAR this contract has.
    pub amounts: Vec<Balance>,
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
        amp_factor: u128,
        total_fee: u32,
    ) -> Self {
        assert!(
            amp_factor >= MIN_AMP && amp_factor <= MAX_AMP,
            "ERR_WRONG_AMP"
        );
        assert_eq!(
            token_account_ids.len() as u32,
            math::N_COINS,
            "ERR_WRONG_TOKEN_COUNT"
        );
        assert!(total_fee < FEE_DIVISOR, "ERR_FEE_TOO_LARGE");
        Self {
            token_account_ids: token_account_ids.iter().map(|a| a.clone().into()).collect(),
            amounts: vec![0u128; token_account_ids.len()],
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

    /// Returns token index for given pool.
    fn token_index(&self, token_id: &AccountId) -> usize {
        self.token_account_ids
            .iter()
            .position(|id| id == token_id)
            .expect("ERR_MISSING_TOKEN")
    }

    /// Returns given pool's total fee.
    pub fn get_fee(&self) -> u32 {
        self.total_fee
    }

    /// Returns volumes of the given pool.
    pub fn get_volumes(&self) -> Vec<SwapVolume> {
        self.volumes.clone()
    }

    /// Add liquidity into the pool.
    /// Allows to add liquidity of a subset of tokens.
    pub fn add_liquidity(
        &mut self,
        sender_id: &AccountId,
        amounts: &mut Vec<Balance>,
        fees: &SwapFees,
    ) -> Balance {
        assert_eq!(
            amounts.len(),
            self.token_account_ids.len(),
            "ERR_WRONG_TOKEN_COUNT"
        );
        let invariant = StableSwap::new(
            self.init_amp_factor,
            self.target_amp_factor,
            env::block_timestamp(),
            self.init_amp_time,
            self.stop_amp_time,
        );
        let new_shares = if self.shares_total_supply == 0 {
            // Bootstrapping the pool.
            invariant
                .compute_d(amounts[0], amounts[1])
                .expect("ERR_CALC_FAILED")
                .as_u128()
        } else {
            invariant
                .compute_lp_amount_for_deposit(
                    amounts[0],
                    amounts[1],
                    self.amounts[0],
                    self.amounts[1],
                    self.shares_total_supply,
                    &Fees::new(self.total_fee, &fees),
                )
                // TODO: proper error
                .expect("ERR_CALC_FAILED")
        };

        // TODO: add slippage check on the LP tokens.
        self.amounts[0] += amounts[0];
        self.amounts[1] += amounts[1];

        self.mint_shares(sender_id, new_shares);
        new_shares
    }

    /// Remove liquidity from the pool.
    /// Allows to remove liquidity of a subset of tokens, by providing 0 in `min_amount` for the tokens to not withdraw.
    pub fn remove_liquidity(
        &mut self,
        sender_id: &AccountId,
        shares: Balance,
        min_amounts: Vec<Balance>,
        fees: &SwapFees,
    ) -> Vec<Balance> {
        assert_eq!(
            min_amounts.len(),
            self.token_account_ids.len(),
            "ERR_WRONG_TOKEN_COUNT"
        );
        let prev_shares_amount = self.shares.get(&sender_id).expect("ERR_NO_SHARES");
        assert!(prev_shares_amount >= shares, "ERR_NOT_ENOUGH_SHARES");
        let mut result = vec![0u128; N_COINS as usize];
        let invariant = StableSwap::new(
            self.init_amp_factor,
            self.target_amp_factor,
            env::block_timestamp(),
            self.init_amp_time,
            self.stop_amp_time,
        );
        let mut fee_amounts = vec![0u128; N_COINS as usize];
        let stable_swap_fees = Fees::new(self.total_fee, &fees);
        for (idx, min_amount) in min_amounts.iter().enumerate() {
            if *min_amount != 0 {
                let (amount_out, fee) = invariant
                    .compute_withdraw_one(
                        shares,
                        self.shares_total_supply,
                        self.amounts[idx],
                        self.amounts[1 - idx],
                        &stable_swap_fees,
                    )
                    .expect("ERR_CALC");
                assert!(amount_out >= *min_amount, "ERR_SLIPPAGE");
                fee_amounts[idx] += fee;
                result[idx] = amount_out;
            }
        }
        println!("fees: {:?}", fee_amounts);
        for i in 0..N_COINS {
            self.amounts[i as usize] = self.amounts[i as usize]
                .checked_sub(result[i as usize])
                .expect("ERR_CALC");
        }
        self.burn_shares(&sender_id, prev_shares_amount, shares);
        env::log(
            format!(
                "{} shares of liquidity removed: receive back {:?}",
                shares,
                result
                    .iter()
                    .zip(self.token_account_ids.iter())
                    .map(|(amount, token_id)| format!("{} {}", amount, token_id))
                    .collect::<Vec<String>>(),
            )
            .as_bytes(),
        );
        result
    }
    /// Returns number of tokens in outcome, given amount.
    /// Tokens are provided as indexes into token list for given pool.
    fn internal_get_return(
        &self,
        token_in: usize,
        amount_in: Balance,
        token_out: usize,
        fees: &SwapFees,
    ) -> SwapResult {
        let invariant = StableSwap::new(
            self.init_amp_factor,
            self.target_amp_factor,
            env::block_timestamp(),
            self.init_amp_time,
            self.stop_amp_time,
        );
        invariant
            .swap_to(
                amount_in,
                self.amounts[token_in],
                self.amounts[token_out],
                &Fees::new(self.total_fee, &fees),
            )
            .expect("ERR_CALC")
    }

    /// Returns how much token you will receive if swap `token_amount_in` of `token_in` for `token_out`.
    pub fn get_return(
        &self,
        token_in: &AccountId,
        amount_in: Balance,
        token_out: &AccountId,
        fees: &SwapFees,
    ) -> Balance {
        self.internal_get_return(
            self.token_index(token_in),
            amount_in,
            self.token_index(token_out),
            &fees,
        )
        .amount_swapped
    }

    /// Swap `token_amount_in` of `token_in` token into `token_out` and return how much was received.
    /// Assuming that `token_amount_in` was already received from `sender_id`.
    pub fn swap(
        &mut self,
        token_in: &AccountId,
        amount_in: Balance,
        token_out: &AccountId,
        min_amount_out: Balance,
        fees: &SwapFees,
    ) -> Balance {
        assert_ne!(token_in, token_out, "ERR_SAME_TOKEN_SWAP");
        let in_idx = self.token_index(token_in);
        let out_idx = self.token_index(token_out);
        let result = self.internal_get_return(in_idx, amount_in, out_idx, &fees);
        assert!(result.amount_swapped >= min_amount_out, "ERR_MIN_AMOUNT");
        env::log(
            format!(
                "Swapped {} {} for {} {}",
                amount_in, token_in, result.amount_swapped, token_out
            )
            .as_bytes(),
        );

        self.amounts[in_idx] = result.new_source_amount;
        self.amounts[out_idx] = result.new_destination_amount;

        // TODO: add admin / referral fee here.

        // mint
        println!("{:?}", self.amounts);

        result.amount_swapped
    }

    /// Mint new shares for given user.
    fn mint_shares(&mut self, account_id: &AccountId, shares: Balance) {
        if shares == 0 {
            return;
        }
        self.shares_total_supply += shares;
        add_to_collection(&mut self.shares, &account_id, shares);
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
        self.shares_total_supply -= shares;
        self.shares
            .insert(&account_id, &(prev_shares_amount - shares));
    }

    /// Register given account with 0 balance in shares.
    /// Storage payment should be checked by caller.
    pub fn share_register(&mut self, account_id: &AccountId) {
        if self.shares.contains_key(account_id) {
            env::panic(ERR14_LP_ALREADY_REGISTERED.as_bytes());
        }
        self.shares.insert(account_id, &0);
    }

    /// Transfers shares from predecessor to receiver.
    pub fn share_transfer(&mut self, sender_id: &AccountId, receiver_id: &AccountId, amount: u128) {
        let balance = self.shares.get(&sender_id).expect("ERR_NO_SHARES");
        if let Some(new_balance) = balance.checked_sub(amount) {
            self.shares.insert(&sender_id, &new_balance);
        } else {
            env::panic(b"ERR_NOT_ENOUGH_SHARES");
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
            "ERR_RAMP_LOCKED"
        );
        assert!(
            future_amp_time >= current_time + MIN_RAMP_DURATION,
            "ERR_INSUFFICIENT_RAMP_TIME"
        );
        let invariant = StableSwap::new(
            self.init_amp_factor,
            self.target_amp_factor,
            current_time,
            self.init_amp_time,
            self.stop_amp_time,
        );
        let amp_factor = invariant.compute_amp_factor().expect("ERR_CALC");
        assert!(
            future_amp_factor > 0 && future_amp_factor < MAX_AMP,
            "ERR_INVALID_AMP_FACTOR"
        );
        assert!(
            (future_amp_factor >= amp_factor && future_amp_factor <= amp_factor * MAX_AMP_CHANGE)
                || (future_amp_factor < amp_factor
                    && future_amp_factor * MAX_AMP_CHANGE >= amp_factor),
            "ERR_AMP_LARGE_CHANGE"
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
        let amp_factor = invariant.compute_amp_factor().expect("ERR_CALC");
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
    use near_sdk_sim::to_yocto;

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
            1,
            &SwapFees::zero(),
        )
    }

    #[test]
    fn test_basics() {
        let mut context = VMContextBuilder::new();
        testing_env!(context.predecessor_account_id(accounts(0)).build());
        let fees = SwapFees::zero();
        let mut pool = StableSwapPool::new(0, vec![accounts(1), accounts(2)], 1, 0);
        assert_eq!(
            pool.tokens(),
            vec![accounts(1).to_string(), accounts(2).to_string()]
        );

        let mut amounts = vec![to_yocto("5"), to_yocto("10")];
        let _ = pool.add_liquidity(accounts(0).as_ref(), &mut amounts, &fees);

        let out = swap(&mut pool, 1, to_yocto("1"), 2);
        assert_eq!(out, 1313682630255414606428571);
        assert_eq!(pool.amounts, vec![to_yocto("6"), 8686317369744585393571429]);
        let out2 = swap(&mut pool, 2, out, 1);
        assert_eq!(out2, to_yocto("1") + 2); // due to precision difference.
        assert_eq!(pool.amounts, vec![to_yocto("5") - 2, to_yocto("10")]);

        // Add only one side of the capital.
        let mut amounts2 = vec![to_yocto("5"), to_yocto("0")];
        let num_shares = pool.add_liquidity(accounts(0).as_ref(), &mut amounts2, &fees);

        // Withdraw on another side of the capital.
        let amounts_out =
            pool.remove_liquidity(accounts(0).as_ref(), num_shares, vec![0, 1], &fees);
        assert_eq!(amounts_out, vec![0, to_yocto("5")]);
    }

    /// Test everything with fees.
    #[test]
    fn test_with_fees() {
        let mut context = VMContextBuilder::new();
        testing_env!(context.predecessor_account_id(accounts(0)).build());
        let mut pool = StableSwapPool::new(0, vec![accounts(1), accounts(2)], 1, 2000);
        let mut amounts = vec![to_yocto("5"), to_yocto("10")];
        let fees = SwapFees::new(1000);
        let num_shares = pool.add_liquidity(accounts(0).as_ref(), &mut amounts, &fees);
        let amount_out = pool.swap(
            accounts(1).as_ref(),
            to_yocto("1"),
            accounts(2).as_ref(),
            1,
            &fees,
        );
        println!("swap out: {}", amount_out);
        let amounts_out =
            pool.remove_liquidity(accounts(0).as_ref(), num_shares, vec![1, 1], &fees);
        println!("amount out: {:?}", amounts_out);
    }

    /// Test that adding and then removing all of the liquidity leaves the pool empty and with no shares.
    #[test]
    fn test_add_transfer_remove_liquidity() {
        let mut context = VMContextBuilder::new();
        testing_env!(context.predecessor_account_id(accounts(0)).build());
        let mut pool = StableSwapPool::new(0, vec![accounts(1), accounts(2)], 1, 0);
        let mut amounts = vec![to_yocto("5"), to_yocto("10")];
        let fees = SwapFees::zero();
        let num_shares = pool.add_liquidity(accounts(0).as_ref(), &mut amounts, &fees);
        assert_eq!(amounts, vec![to_yocto("5"), to_yocto("10")]);
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
        let out_amounts =
            pool.remove_liquidity(accounts(3).as_ref(), num_shares, vec![1, 1], &fees);

        // Check it's all taken out. Due to precision there is ~1 yN.
        assert_eq!(
            vec![amounts[0], amounts[1]],
            vec![out_amounts[0] + 1, out_amounts[1] + 1]
        );
        assert_eq!(pool.share_total_balance(), 0);
        assert_eq!(pool.share_balance_of(accounts(0).as_ref()), 0);
        assert_eq!(pool.share_balance_of(accounts(3).as_ref()), 0);
        assert_eq!(pool.amounts, vec![1, 1]);
    }

    /// Test ramping up amplification factor, ramping it even more and then stopping.
    #[test]
    fn test_ramp_amp() {
        let mut context = VMContextBuilder::new();
        testing_env!(context.predecessor_account_id(accounts(0)).build());
        let mut pool = StableSwapPool::new(0, vec![accounts(1), accounts(2)], 1, 0);

        let start_ts = 1_000_000_000;
        testing_env!(context.block_timestamp(start_ts).build());
        pool.ramp_amplification(5, start_ts + MIN_RAMP_DURATION * 10);
        testing_env!(context
            .block_timestamp(start_ts + MIN_RAMP_DURATION * 3)
            .build());
        pool.ramp_amplification(15, start_ts + MIN_RAMP_DURATION * 20);
        testing_env!(context
            .block_timestamp(start_ts + MIN_RAMP_DURATION * 5)
            .build());
        pool.stop_ramp_amplification();
    }
}
