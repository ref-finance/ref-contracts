use std::cmp::min;

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::LookupMap;
use near_sdk::json_types::ValidAccountId;
use near_sdk::{env, AccountId, Balance};

use crate::errors::{
    ERR13_LP_NOT_REGISTERED, ERR14_LP_ALREADY_REGISTERED, ERR31_ZERO_AMOUNT, ERR32_ZERO_SHARES,
};
use crate::utils::{
    add_to_collection, integer_sqrt, SwapVolume, FEE_DIVISOR, INIT_SHARES_SUPPLY, U256,
};

const MAX_NUM_TOKENS: usize = 2;

/// Implementation of simple pool, that maintains constant product between balances of all the tokens.
/// Similar in design to "Uniswap".
/// Liquidity providers when depositing receive shares, that can be later burnt to withdraw pool's tokens in proportion.
#[derive(BorshSerialize, BorshDeserialize)]
pub struct SimplePool {
    /// List of tokens in the pool.
    pub token_account_ids: Vec<AccountId>,
    /// How much NEAR this contract has.
    pub amounts: Vec<Balance>,
    /// Volumes accumulated by this pool.
    pub volumes: Vec<SwapVolume>,
    /// Fee charged for swap (gets divided by FEE_DIVISOR).
    pub total_fee: u32,
    /// Portion of the fee going to exchange.
    pub exchange_fee: u32,
    /// Portion of the fee going to referral.
    pub referral_fee: u32,
    /// Shares of the pool by liquidity providers.
    pub shares: LookupMap<AccountId, Balance>,
    /// Total number of shares.
    pub shares_total_supply: Balance,
}

impl SimplePool {
    pub fn new(
        id: u32,
        token_account_ids: Vec<ValidAccountId>,
        total_fee: u32,
        exchange_fee: u32,
        referral_fee: u32,
    ) -> Self {
        assert!(
            total_fee < FEE_DIVISOR && (exchange_fee + referral_fee) <= total_fee,
            "ERR_FEE_TOO_LARGE"
        );
        assert_ne!(token_account_ids.len(), 1, "ERR_NOT_ENOUGH_TOKENS");
        assert!(
            token_account_ids.len() <= MAX_NUM_TOKENS,
            "ERR_TOO_MANY_TOKENS"
        );
        Self {
            token_account_ids: token_account_ids.iter().map(|a| a.clone().into()).collect(),
            amounts: vec![0u128; token_account_ids.len()],
            volumes: vec![SwapVolume::default(); token_account_ids.len()],
            total_fee,
            exchange_fee,
            referral_fee,
            shares: LookupMap::new(format!("s{}", id).into_bytes()),
            shares_total_supply: 0,
        }
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

    /// Adds the amounts of tokens to liquidity pool and returns number of shares that this user receives.
    /// Updates amount to amount kept in the pool.
    pub fn add_liquidity(&mut self, sender_id: &AccountId, amounts: &mut Vec<Balance>) -> Balance {
        assert_eq!(
            amounts.len(),
            self.token_account_ids.len(),
            "ERR_WRONG_TOKEN_COUNT"
        );
        let shares = if self.shares_total_supply > 0 {
            let mut fair_supply = U256::max_value();
            for i in 0..self.token_account_ids.len() {
                assert!(amounts[i] > 0, "{}", ERR31_ZERO_AMOUNT);
                fair_supply = min(
                    fair_supply,
                    U256::from(amounts[i]) * U256::from(self.shares_total_supply) / self.amounts[i],
                );
            }
            for i in 0..self.token_account_ids.len() {
                let amount = (U256::from(self.amounts[i]) * fair_supply
                    / U256::from(self.shares_total_supply))
                .as_u128();
                assert!(amount > 0, "{}", ERR31_ZERO_AMOUNT);
                self.amounts[i] += amount;
                amounts[i] = amount;
            }
            fair_supply.as_u128()
        } else {
            for i in 0..self.token_account_ids.len() {
                self.amounts[i] += amounts[i];
            }
            INIT_SHARES_SUPPLY
        };
        self.mint_shares(&sender_id, shares);
        assert!(shares > 0, "{}", ERR32_ZERO_SHARES);
        env::log(
            format!(
                "Liquidity added {:?}, minted {} shares",
                amounts
                    .iter()
                    .zip(self.token_account_ids.iter())
                    .map(|(amount, token_id)| format!("{} {}", amount, token_id))
                    .collect::<Vec<String>>(),
                shares
            )
            .as_bytes(),
        );
        shares
    }

    /// Mint new shares for given user.
    fn mint_shares(&mut self, account_id: &AccountId, shares: Balance) {
        if shares == 0 {
            return;
        }
        self.shares_total_supply += shares;
        add_to_collection(&mut self.shares, &account_id, shares);
    }

    /// Removes given number of shares from the pool and returns amounts to the parent.
    pub fn remove_liquidity(
        &mut self,
        sender_id: &AccountId,
        shares: Balance,
        min_amounts: Vec<Balance>,
    ) -> Vec<Balance> {
        let prev_shares_amount = self.shares.get(&sender_id).expect("ERR_NO_SHARES");
        assert!(prev_shares_amount >= shares, "ERR_NOT_ENOUGH_SHARES");
        let mut result = vec![];
        for i in 0..self.token_account_ids.len() {
            let amount = (U256::from(self.amounts[i]) * U256::from(shares)
                / U256::from(self.shares_total_supply))
            .as_u128();
            assert!(amount >= min_amounts[i], "ERR_MIN_AMOUNT");
            self.amounts[i] -= amount;
            result.push(amount);
        }
        if prev_shares_amount == shares {
            self.shares.remove(&sender_id);
        } else {
            self.shares
                .insert(&sender_id, &(prev_shares_amount - shares));
        }
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
        self.shares_total_supply -= shares;
        result
    }

    /// Returns token index for given pool.
    fn token_index(&self, token_id: &AccountId) -> usize {
        self.token_account_ids
            .iter()
            .position(|id| id == token_id)
            .expect("ERR_MISSING_TOKEN")
    }

    /// Returns number of tokens in outcome, given amount.
    /// Tokens are provided as indexes into token list for given pool.
    fn internal_get_return(
        &self,
        token_in: usize,
        amount_in: Balance,
        token_out: usize,
    ) -> Balance {
        let in_balance = U256::from(self.amounts[token_in]);
        let out_balance = U256::from(self.amounts[token_out]);
        assert!(
            in_balance > U256::zero()
                && out_balance > U256::zero()
                && token_in != token_out
                && amount_in > 0,
            "ERR_INVALID"
        );
        let amount_with_fee = U256::from(amount_in) * U256::from(FEE_DIVISOR - self.total_fee);
        (amount_with_fee * out_balance / (U256::from(FEE_DIVISOR) * in_balance + amount_with_fee))
            .as_u128()
    }

    /// Returns how much token you will receive if swap `token_amount_in` of `token_in` for `token_out`.
    pub fn get_return(
        &self,
        token_in: &AccountId,
        amount_in: Balance,
        token_out: &AccountId,
    ) -> Balance {
        self.internal_get_return(
            self.token_index(token_in),
            amount_in,
            self.token_index(token_out),
        )
    }

    /// Returns given pool's total fee.
    pub fn get_fee(&self) -> u32 {
        self.total_fee
    }

    /// Returns volumes of the given pool.
    pub fn get_volumes(&self) -> Vec<SwapVolume> {
        self.volumes.clone()
    }

    /// Swap `token_amount_in` of `token_in` token into `token_out` and return how much was received.
    /// Assuming that `token_amount_in` was already received from `sender_id`.
    pub fn swap(
        &mut self,
        token_in: &AccountId,
        amount_in: Balance,
        token_out: &AccountId,
        min_amount_out: Balance,
        exchange_id: &AccountId,
        referral_id: &Option<AccountId>,
    ) -> Balance {
        let in_idx = self.token_index(token_in);
        let out_idx = self.token_index(token_out);
        let amount_out = self.internal_get_return(in_idx, amount_in, out_idx);
        assert!(amount_out >= min_amount_out, "ERR_MIN_AMOUNT");
        env::log(
            format!(
                "Swapped {} {} for {} {}",
                amount_in, token_in, amount_out, token_out
            )
            .as_bytes(),
        );

        let prev_invariant =
            integer_sqrt(U256::from(self.amounts[in_idx]) * U256::from(self.amounts[out_idx]));

        self.amounts[in_idx] += amount_in;
        self.amounts[out_idx] -= amount_out;

        // "Invariant" is by how much the dot product of amounts increased due to fees.
        let new_invariant =
            integer_sqrt(U256::from(self.amounts[in_idx]) * U256::from(self.amounts[out_idx]));

        // Invariant can not reduce (otherwise loosing balance of the pool and something it broken).
        assert!(new_invariant >= prev_invariant, "ERR_INVARIANT");
        let numerator = (new_invariant - prev_invariant) * U256::from(self.shares_total_supply);

        // Allocate exchange fee as fraction of total fee by issuing LP shares proportionally.
        if self.exchange_fee > 0 && numerator > U256::zero() {
            let denominator = new_invariant * self.total_fee / self.exchange_fee;
            self.mint_shares(&exchange_id, (numerator / denominator).as_u128());
        }

        // If there is referral provided and the account already registered LP, allocate it % of LP rewards.
        if let Some(referral_id) = referral_id {
            if self.referral_fee > 0
                && numerator > U256::zero()
                && self.shares.contains_key(referral_id)
            {
                let denominator = new_invariant * self.total_fee / self.referral_fee;
                self.mint_shares(&referral_id, (numerator / denominator).as_u128());
            }
        }

        // Keeping track of volume per each input traded separately.
        // Reported volume with fees will be sum of `input`, without fees will be sum of `output`.
        self.volumes[in_idx].input.0 += amount_in;
        self.volumes[in_idx].output.0 += amount_out;

        amount_out
    }
}

#[cfg(test)]
mod tests {
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::{testing_env, MockedBlockchain};
    use near_sdk_sim::to_yocto;

    use super::*;

    #[test]
    fn test_pool_swap() {
        let one_near = 10u128.pow(24);
        let mut context = VMContextBuilder::new();
        context.predecessor_account_id(accounts(0));
        testing_env!(context.build());
        let mut pool = SimplePool::new(0, vec![accounts(1), accounts(2)], 30, 0, 0);
        let mut amounts = vec![to_yocto("5"), to_yocto("10")];
        let num_shares = pool.add_liquidity(accounts(0).as_ref(), &mut amounts);
        assert_eq!(amounts, vec![to_yocto("5"), to_yocto("10")]);
        assert_eq!(
            pool.share_balance_of(accounts(0).as_ref()),
            INIT_SHARES_SUPPLY
        );
        let out = pool.swap(
            accounts(1).as_ref(),
            one_near,
            accounts(2).as_ref(),
            1,
            accounts(3).as_ref(),
            &None,
        );
        assert_eq!(
            pool.share_balance_of(accounts(0).as_ref()),
            INIT_SHARES_SUPPLY
        );
        pool.share_register(accounts(1).as_ref());
        pool.share_transfer(
            accounts(0).as_ref(),
            accounts(1).as_ref(),
            INIT_SHARES_SUPPLY / 2,
        );
        assert_eq!(
            pool.share_balance_of(accounts(0).as_ref()),
            INIT_SHARES_SUPPLY / 2
        );
        assert_eq!(
            pool.share_balance_of(accounts(1).as_ref()),
            INIT_SHARES_SUPPLY / 2
        );
        assert_eq!(
            pool.remove_liquidity(accounts(0).as_ref(), num_shares / 2, vec![1, 1]),
            [3 * one_near, 5 * one_near - out / 2]
        );
    }

    #[test]
    fn test_pool_swap_with_fees() {
        let one_near = 10u128.pow(24);
        let mut context = VMContextBuilder::new();
        context.predecessor_account_id(accounts(0));
        testing_env!(context.build());
        let mut pool = SimplePool::new(0, vec![accounts(1), accounts(2)], 100, 100, 0);
        let mut amounts = vec![to_yocto("5"), to_yocto("10")];
        let num_shares = pool.add_liquidity(accounts(0).as_ref(), &mut amounts);
        assert_eq!(amounts, vec![to_yocto("5"), to_yocto("10")]);
        assert_eq!(
            pool.share_balance_of(accounts(0).as_ref()),
            INIT_SHARES_SUPPLY
        );
        let out = pool.swap(
            accounts(1).as_ref(),
            one_near,
            accounts(2).as_ref(),
            1,
            accounts(3).as_ref(),
            &None,
        );
        assert_eq!(
            pool.share_balance_of(accounts(0).as_ref()),
            INIT_SHARES_SUPPLY
        );
        let liq1 = pool.remove_liquidity(accounts(0).as_ref(), num_shares, vec![1, 1]);
        let num_shares2 = pool.share_balance_of(accounts(3).as_ref());
        let liq2 = pool.remove_liquidity(accounts(3).as_ref(), num_shares2, vec![1, 1]);
        assert_eq!(liq1[0] + liq2[0], to_yocto("6"));
        assert_eq!(liq1[1] + liq2[1], to_yocto("10") - out);
    }

    #[test]
    #[should_panic(expected = "E31: adding zero amount")]
    fn test_rounding() {
        testing_env!(VMContextBuilder::new().build());
        let mut pool = SimplePool {
            token_account_ids: vec![accounts(0).to_string(), accounts(1).to_string()],
            amounts: vec![367701466208080431008668441, 2267116519548219219535],
            volumes: vec![SwapVolume::default(), SwapVolume::default()],
            total_fee: 30,
            exchange_fee: 5,
            referral_fee: 1,
            shares_total_supply: 35967818779820559673547466,
            shares: LookupMap::new(b"s0".to_vec()),
        };
        let mut amounts = vec![145782, 1];
        let _ = pool.add_liquidity(&accounts(2).to_string(), &mut amounts);
    }
}
