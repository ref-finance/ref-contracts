use near_sdk::collections::LookupMap;
use near_sdk::json_types::ValidAccountId;
use near_sdk::{env, AccountId, Balance};

use crate::errors::{ERR13_LP_NOT_REGISTERED, ERR14_LP_ALREADY_REGISTERED};
use crate::stable_swap::math::{Fees, StableSwap};
use crate::utils::{add_to_collection, SwapVolume};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};

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
    /// Portion of the fee going to exchange.
    pub exchange_fee: u32,
    /// Portion of the fee going to referral.
    pub referral_fee: u32,
    /// Shares of the pool by liquidity providers.
    pub shares: LookupMap<AccountId, Balance>,
    /// Total number of shares.
    pub shares_total_supply: Balance,
}

impl StableSwapPool {
    pub fn new(
        id: u32,
        token_account_ids: Vec<ValidAccountId>,
        total_fee: u32,
        exchange_fee: u32,
        referral_fee: u32,
    ) -> Self {
        assert_eq!(token_account_ids.len(), 2, "ERR_WRONG_TOKEN_COUNT");
        Self {
            token_account_ids: token_account_ids.iter().map(|a| a.clone().into()).collect(),
            amounts: vec![0u128; token_account_ids.len()],
            volumes: vec![SwapVolume::default(); token_account_ids.len()],
            total_fee,
            exchange_fee,
            referral_fee,
            shares: LookupMap::new(format!("ss{}", id).into_bytes()),
            shares_total_supply: 0,
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
    pub fn add_liquidity(&mut self, sender_id: &AccountId, amounts: &mut Vec<Balance>) -> Balance {
        assert_eq!(
            amounts.len(),
            self.token_account_ids.len(),
            "ERR_WRONG_TOKEN_COUNT"
        );
        let calc = StableSwap::new(0, 0, 0, 0, 0);
        let new_shares = calc
            .compute_lp_amount_for_deposit(
                self.amounts[0],
                self.amounts[1],
                0,
                0,
                self.shares_total_supply,
                &Fees {
                    trade_fee: 0,
                    admin_fee: 0,
                },
            )
            // TODO: proper error
            .expect("ERR_FAILED");
        self.mint_shares(sender_id, new_shares);
        new_shares
    }

    /// Mint new shares for given user.
    fn mint_shares(&mut self, account_id: &AccountId, shares: Balance) {
        if shares == 0 {
            return;
        }
        self.shares_total_supply += shares;
        add_to_collection(&mut self.shares, &account_id, shares);
    }

    /// Remove liquidity from the pool.
    /// Allows to remove liquidity of a subset of tokens, by providing 0 in `min_amount` for the tokens to not withdraw.
    pub fn remove_liquidity(
        &mut self,
        sender_id: &AccountId,
        shares: Balance,
        min_amounts: Vec<Balance>,
    ) -> Vec<Balance> {
        assert_eq!(
            min_amounts.len(),
            self.token_account_ids.len(),
            "ERR_WRONG_TOKEN_COUNT"
        );
        let prev_shares_amount = self.shares.get(&sender_id).expect("ERR_NO_SHARES");
        assert!(prev_shares_amount >= shares, "ERR_NOT_ENOUGH_SHARES");
        let mut result = vec![];
        result
    }
    /// Returns number of tokens in outcome, given amount.
    /// Tokens are provided as indexes into token list for given pool.
    fn internal_get_return(
        &self,
        token_in: usize,
        amount_in: Balance,
        token_out: usize,
    ) -> Balance {
        0
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
        0
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::{testing_env, MockedBlockchain};
    use near_sdk_sim::to_yocto;

    #[test]
    fn test_basics() {
        let mut context = VMContextBuilder::new();
        context.predecessor_account_id(accounts(0));
        testing_env!(context.build());
        let mut pool = StableSwapPool::new(0, vec![accounts(1), accounts(2)], 0, 0, 0);
        let mut amounts = vec![to_yocto("5"), to_yocto("10")];
        let num_shares = pool.add_liquidity(accounts(0).as_ref(), &mut amounts);
        assert_eq!(amounts, vec![to_yocto("5"), to_yocto("10")]);
        assert_eq!(pool.share_balance_of(accounts(0).as_ref()), 1);
    }
}
