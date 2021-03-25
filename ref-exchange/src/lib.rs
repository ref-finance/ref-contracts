use std::convert::TryInto;

use near_contract_standards::storage_management::{
    StorageBalance, StorageBalanceBounds, StorageManagement,
};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LookupMap, UnorderedSet, Vector};
use near_sdk::json_types::{ValidAccountId, U128};
use near_sdk::{assert_one_yocto, env, log, near_bindgen, AccountId, PanicOnDefault, Promise};

use crate::account_deposit::AccountDeposit;
pub use crate::action::*;
use crate::errors::*;
use crate::pool::Pool;
use crate::simple_pool::SimplePool;
use crate::utils::check_token_duplicates;
pub use crate::views::PoolInfo;

mod account_deposit;
mod action;
mod errors;
mod legacy;
mod owner;
mod pool;
mod simple_pool;
mod storage_impl;
mod token_receiver;
mod utils;
mod views;

near_sdk::setup_alloc!();

#[near_bindgen]
#[derive(BorshSerialize, BorshDeserialize, PanicOnDefault)]
pub struct Contract {
    /// Account of the owner.
    owner_id: AccountId,
    /// Exchange fee, that goes to exchange itself (managed by governance).
    exchange_fee: u32,
    /// Referral fee, that goes to referrer in the call.
    referral_fee: u32,
    /// List of all the pools.
    pools: Vector<Pool>,
    /// Balances of deposited tokens for each account.
    deposited_amounts: LookupMap<AccountId, AccountDeposit>,
    /// Set of whitelisted tokens by "owner".
    whitelisted_tokens: UnorderedSet<AccountId>,
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new(owner_id: ValidAccountId, exchange_fee: u32, referral_fee: u32) -> Self {
        Self {
            owner_id: owner_id.as_ref().clone(),
            exchange_fee,
            referral_fee,
            pools: Vector::new(b"p".to_vec()),
            deposited_amounts: LookupMap::new(b"d".to_vec()),
            whitelisted_tokens: UnorderedSet::new(b"w".to_vec()),
        }
    }

    /// Adds new "Simple Pool" with given tokens and given fee.
    /// Attached NEAR should be enough to cover the added storage.
    #[payable]
    pub fn add_simple_pool(&mut self, tokens: Vec<ValidAccountId>, fee: u32) -> u64 {
        check_token_duplicates(&tokens);
        self.internal_add_pool(Pool::SimplePool(SimplePool::new(
            self.pools.len() as u32,
            tokens,
            fee + self.exchange_fee + self.referral_fee,
            self.exchange_fee,
            self.referral_fee,
        )))
    }

    /// Execute set of swap actions between pools.
    /// If referrer provided, pays referral_fee to it.
    #[payable]
    pub fn swap(&mut self, actions: Vec<SwapAction>, referral_id: Option<ValidAccountId>) -> U128 {
        assert_one_yocto();
        let sender_id = env::predecessor_account_id();
        let mut prev_amount = None;
        let referral_id = referral_id.map(|r| r.as_ref().clone());
        for action in actions {
            let amount_in = action
                .amount_in
                .unwrap_or_else(|| prev_amount.expect("ERR_FIRST_SWAP_MISSING_AMOUNT"));
            prev_amount = Some(self.internal_swap(
                &sender_id,
                action.pool_id,
                action.token_in,
                amount_in,
                action.token_out,
                action.min_amount_out,
                referral_id.clone(),
            ));
        }
        prev_amount.unwrap()
    }

    /// Add liquidity from already deposited amounts to given pool.
    #[payable]
    pub fn add_liquidity(&mut self, pool_id: u64, amounts: Vec<U128>) {
        assert_one_yocto();
        let sender_id = env::predecessor_account_id();
        let mut amounts: Vec<u128> = amounts.into_iter().map(|amount| amount.into()).collect();
        let mut pool = self.pools.get(pool_id).expect("ERR_NO_POOL");
        // Add amounts given to liquidity first. It will return the balanced amounts.
        pool.add_liquidity(&sender_id, &mut amounts);
        let mut deposits = self.deposited_amounts.get(&sender_id).unwrap_or_default();
        let tokens = pool.tokens();
        // Subtract updated amounts from deposits. This will fail if there is not enough funds for any of the tokens.
        for i in 0..tokens.len() {
            deposits.sub(tokens[i].clone(), amounts[i]);
        }
        self.deposited_amounts.insert(&sender_id, &deposits);
        self.pools.replace(pool_id, &pool);
    }

    /// Remove liquidity from the pool into general pool of liquidity.
    #[payable]
    pub fn remove_liquidity(&mut self, pool_id: u64, shares: U128, min_amounts: Vec<U128>) {
        assert_one_yocto();
        let sender_id = env::predecessor_account_id();
        let mut pool = self.pools.get(pool_id).expect("ERR_NO_POOL");
        let amounts = pool.remove_liquidity(
            &sender_id,
            shares.into(),
            min_amounts
                .into_iter()
                .map(|amount| amount.into())
                .collect(),
        );
        self.pools.replace(pool_id, &pool);
        let tokens = pool.tokens();
        let mut deposits = self.deposited_amounts.get(&sender_id).unwrap_or_default();
        for i in 0..tokens.len() {
            deposits.add(tokens[i].clone(), amounts[i]);
        }
        self.deposited_amounts.insert(&sender_id, &deposits);
    }
}

/// Internal methods implementation.
impl Contract {
    /// Adds given pool to the list and returns it's id.
    /// If there is not enough attached balance to cover storage, fails.
    /// If too much attached - refunds it back.
    fn internal_add_pool(&mut self, pool: Pool) -> u64 {
        let prev_storage = env::storage_usage();
        let id = self.pools.len() as u64;
        self.pools.push(&pool);

        // Check how much storage cost and refund the left over back.
        let storage_cost = (env::storage_usage() - prev_storage) as u128 * env::storage_byte_cost();
        assert!(
            storage_cost <= env::attached_deposit(),
            "ERR_STORAGE_DEPOSIT"
        );
        let refund = env::attached_deposit() - storage_cost;
        if refund > 0 {
            Promise::new(env::predecessor_account_id()).transfer(refund);
        }
        id
    }

    /// Swaps given amount_in of token_in into token_out via given pool.
    /// Should be at least min_amount_out or swap will fail (prevents front running and other slippage issues).
    fn internal_swap(
        &mut self,
        sender_id: &AccountId,
        pool_id: u64,
        token_in: ValidAccountId,
        amount_in: U128,
        token_out: ValidAccountId,
        min_amount_out: U128,
        referral_id: Option<AccountId>,
    ) -> U128 {
        let mut deposits = self.deposited_amounts.get(&sender_id).unwrap_or_default();
        let amount_in: u128 = amount_in.into();
        deposits.sub(token_in.as_ref().clone(), amount_in);
        let mut pool = self.pools.get(pool_id).expect("ERR_NO_POOL");
        let amount_out = pool.swap(
            token_in.as_ref(),
            amount_in,
            token_out.as_ref(),
            min_amount_out.into(),
            &self.owner_id,
            referral_id,
        );
        deposits.add(token_out.as_ref().clone(), amount_out);
        self.deposited_amounts.insert(&sender_id, &deposits);
        self.pools.replace(pool_id, &pool);
        amount_out.into()
    }
}

#[cfg(test)]
mod tests {
    use std::convert::TryFrom;

    use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::{testing_env, Balance, MockedBlockchain};
    use near_sdk_sim::to_yocto;

    use super::*;

    /// Creates contract and a pool with tokens with 0.3% of total fee.
    fn setup_contract() -> (VMContextBuilder, Contract) {
        let mut context = VMContextBuilder::new();
        testing_env!(context.predecessor_account_id(accounts(0)).build());
        let contract = Contract::new(accounts(0), 4, 1);
        (context, contract)
    }

    fn deposit_tokens(
        context: &mut VMContextBuilder,
        contract: &mut Contract,
        account_id: ValidAccountId,
        token_amounts: Vec<(ValidAccountId, Balance)>,
    ) {
        if contract.storage_balance_of(account_id.clone()).is_none() {
            testing_env!(context
                .predecessor_account_id(account_id.clone())
                .attached_deposit(to_yocto("1"))
                .build());
            contract.storage_deposit(None, None);
        }
        testing_env!(context
            .predecessor_account_id(account_id.clone())
            .attached_deposit(to_yocto("1"))
            .build());
        let tokens = token_amounts
            .iter()
            .map(|(token_id, _)| token_id.clone().into())
            .collect();
        contract.register_tokens(tokens);
        for (token_id, amount) in token_amounts {
            testing_env!(context
                .predecessor_account_id(token_id)
                .attached_deposit(1)
                .build());
            contract.ft_on_transfer(account_id.clone(), U128(amount), "".to_string());
        }
    }

    fn create_pool_with_liquidity(
        context: &mut VMContextBuilder,
        contract: &mut Contract,
        account_id: ValidAccountId,
        token_amounts: Vec<(ValidAccountId, Balance)>,
    ) -> u64 {
        let tokens = token_amounts
            .iter()
            .map(|(x, _)| x.clone())
            .collect::<Vec<_>>();
        testing_env!(context.predecessor_account_id(accounts(0)).build());
        contract.extend_whitelisted_tokens(tokens.clone());
        testing_env!(context
            .predecessor_account_id(account_id.clone())
            .attached_deposit(env::storage_byte_cost() * 300)
            .build());
        let pool_id = contract.add_simple_pool(tokens, 25);
        testing_env!(context
            .predecessor_account_id(account_id.clone())
            .attached_deposit(to_yocto("0.03"))
            .build());
        contract.storage_deposit(None, None);
        deposit_tokens(context, contract, accounts(3), token_amounts.clone());
        testing_env!(context
            .predecessor_account_id(account_id.clone())
            .attached_deposit(1)
            .build());
        contract.add_liquidity(
            pool_id,
            token_amounts.into_iter().map(|(_, x)| U128(x)).collect(),
        );
        pool_id
    }

    #[test]
    fn test_basics() {
        let one_near = 10u128.pow(24);
        let (mut context, mut contract) = setup_contract();
        // add liquidity of (1,2) tokens
        create_pool_with_liquidity(
            &mut context,
            &mut contract,
            accounts(3),
            vec![(accounts(1), to_yocto("5")), (accounts(2), to_yocto("10"))],
        );
        deposit_tokens(
            &mut context,
            &mut contract,
            accounts(3),
            vec![
                (accounts(1), to_yocto("100")),
                (accounts(2), to_yocto("100")),
            ],
        );

        assert_eq!(
            contract.get_deposit(accounts(3), accounts(1)),
            to_yocto("100").into()
        );
        assert_eq!(
            contract.get_deposit(accounts(3), accounts(2)),
            to_yocto("100").into()
        );
        assert_eq!(
            contract.get_pool_total_shares(0).0,
            crate::utils::INIT_SHARES_SUPPLY
        );

        // Get price from pool #0 1 -> 2 tokens.
        let expected_out = contract.get_return(0, accounts(1), one_near.into(), accounts(2));
        assert_eq!(expected_out.0, 1662497915624478906119726);

        testing_env!(context
            .predecessor_account_id(accounts(3))
            .attached_deposit(1)
            .build());
        let amount_out = contract.swap(
            vec![SwapAction {
                pool_id: 0,
                token_in: accounts(1),
                amount_in: Some(one_near.into()),
                token_out: accounts(2),
                min_amount_out: U128(1),
            }],
            None,
        );
        assert_eq!(amount_out, expected_out);
        assert_eq!(
            contract.get_deposit(accounts(3), accounts(1)).0,
            99 * one_near
        );
        assert_eq!(
            contract.get_deposit(accounts(3), accounts(2)).0,
            100 * one_near + amount_out.0
        );

        testing_env!(context.predecessor_account_id(accounts(3)).build());
        contract.remove_liquidity(
            0,
            contract.get_pool_shares(0, accounts(3)),
            vec![1.into(), 2.into()],
        );
        // Exchange fees left in the pool as liquidity.
        assert_eq!(contract.get_pool_total_shares(0).0, 33337501041992301475);

        contract.withdraw(
            accounts(1),
            contract.get_deposit(accounts(3), accounts(1)),
            None,
        );
        assert_eq!(contract.get_deposit(accounts(3), accounts(1)).0, 0);
    }

    /// Test liquidity management.
    #[test]
    fn test_liquidity() {
        let (mut context, mut contract) = setup_contract();
        deposit_tokens(
            &mut context,
            &mut contract,
            accounts(3),
            vec![
                (accounts(1), to_yocto("100")),
                (accounts(2), to_yocto("100")),
            ],
        );
        testing_env!(context
            .predecessor_account_id(accounts(3))
            .attached_deposit(to_yocto("1"))
            .build());
        let id = contract.add_simple_pool(vec![accounts(1), accounts(2)], 25);
        testing_env!(context.attached_deposit(1).build());
        contract.add_liquidity(id, vec![U128(to_yocto("50")), U128(to_yocto("10"))]);
        contract.add_liquidity(id, vec![U128(to_yocto("50")), U128(to_yocto("50"))]);
        contract.remove_liquidity(id, U128(to_yocto("1")), vec![U128(1), U128(1)]);

        // Check that amounts add up to deposits.
        let amounts = contract.get_pool(id).amounts;
        let deposit1 = contract.get_deposit(accounts(3), accounts(1)).0;
        let deposit2 = contract.get_deposit(accounts(3), accounts(2)).0;
        assert_eq!(amounts[0].0 + deposit1, to_yocto("100"));
        assert_eq!(amounts[1].0 + deposit2, to_yocto("100"));
    }

    /// Should deny creating a pool with duplicate tokens.
    #[test]
    #[should_panic(expected = "ERR_TOKEN_DUPLICATES")]
    fn test_deny_duplicate_tokens_pool() {
        let (mut context, mut contract) = setup_contract();
        create_pool_with_liquidity(
            &mut context,
            &mut contract,
            accounts(3),
            vec![(accounts(1), to_yocto("5")), (accounts(1), to_yocto("10"))],
        );
    }

    /// Deny pool with a single token
    #[test]
    #[should_panic(expected = "ERR_NOT_ENOUGH_TOKENS")]
    fn test_deny_single_token_pool() {
        let (mut context, mut contract) = setup_contract();
        create_pool_with_liquidity(
            &mut context,
            &mut contract,
            accounts(3),
            vec![(accounts(1), to_yocto("5"))],
        );
    }

    /// Deny pool with a single token
    #[test]
    #[should_panic(expected = "ERR_TOO_MANY_TOKENS")]
    fn test_deny_too_many_tokens_pool() {
        let (mut context, mut contract) = setup_contract();
        create_pool_with_liquidity(
            &mut context,
            &mut contract,
            accounts(3),
            vec![
                (accounts(1), to_yocto("5")),
                (accounts(2), to_yocto("10")),
                (accounts(3), to_yocto("10")),
            ],
        );
    }

    #[test]
    #[should_panic(expected = "E12: token not whitelisted")]
    fn test_deny_send_malicious_token() {
        let (mut context, mut contract) = setup_contract();
        let acc = ValidAccountId::try_from("test_user").unwrap();
        testing_env!(context
            .predecessor_account_id(acc.clone())
            .attached_deposit(to_yocto("1"))
            .build());
        contract.storage_deposit(Some(acc.clone()), None);
        testing_env!(context
            .predecessor_account_id(ValidAccountId::try_from("malicious").unwrap())
            .build());
        contract.ft_on_transfer(acc, U128(1_000), "".to_string());
    }

    #[test]
    fn test_send_user_specific_token() {
        let (mut context, mut contract) = setup_contract();
        let acc = ValidAccountId::try_from("test_user").unwrap();
        let custom_token = ValidAccountId::try_from("custom").unwrap();
        testing_env!(context
            .predecessor_account_id(acc.clone())
            .attached_deposit(to_yocto("1"))
            .build());
        contract.storage_deposit(None, None);
        contract.register_tokens(vec![custom_token.clone()]);
        testing_env!(context.predecessor_account_id(custom_token.clone()).build());
        contract.ft_on_transfer(acc.clone(), U128(1_000), "".to_string());
        let prev = contract.storage_balance_of(acc.clone()).unwrap();
        testing_env!(context
            .predecessor_account_id(acc.clone())
            .attached_deposit(1)
            .build());
        contract.withdraw(custom_token, U128(1_000), Some(true));
        let new = contract.storage_balance_of(acc.clone()).unwrap();
        // More available storage after withdrawing & unregistering the token.
        assert!(new.available.0 > prev.available.0);
    }

    #[test]
    #[should_panic(expected = "ERR_MIN_AMOUNT")]
    fn test_deny_min_amount() {
        let (mut context, mut contract) = setup_contract();
        create_pool_with_liquidity(
            &mut context,
            &mut contract,
            accounts(3),
            vec![(accounts(1), to_yocto("1")), (accounts(2), to_yocto("1"))],
        );
        let acc = ValidAccountId::try_from("test_user").unwrap();
        deposit_tokens(
            &mut context,
            &mut contract,
            acc.clone(),
            vec![(accounts(1), 1_000_000)],
        );
        testing_env!(context
            .predecessor_account_id(acc.clone())
            .attached_deposit(1)
            .build());
        contract.swap(
            vec![SwapAction {
                pool_id: 0,
                token_in: accounts(1),
                amount_in: Some(U128(1_000_000)),
                token_out: accounts(2),
                min_amount_out: U128(1_000_000),
            }],
            None,
        );
    }

    #[test]
    fn test_roundtrip_swap() {
        let (mut context, mut contract) = setup_contract();
        create_pool_with_liquidity(
            &mut context,
            &mut contract,
            accounts(3),
            vec![(accounts(1), to_yocto("5")), (accounts(2), to_yocto("10"))],
        );
        let acc = ValidAccountId::try_from("test_user").unwrap();
        deposit_tokens(
            &mut context,
            &mut contract,
            acc.clone(),
            vec![(accounts(1), 1_000_000)],
        );
        testing_env!(context
            .predecessor_account_id(acc.clone())
            .attached_deposit(1)
            .build());
        contract.swap(
            vec![
                SwapAction {
                    pool_id: 0,
                    token_in: accounts(1),
                    amount_in: Some(U128(1_000)),
                    token_out: accounts(2),
                    min_amount_out: U128(1),
                },
                SwapAction {
                    pool_id: 0,
                    token_in: accounts(2),
                    amount_in: None,
                    token_out: accounts(1),
                    min_amount_out: U128(1),
                },
            ],
            None,
        );
        // Roundtrip returns almost everything except 0.3% fee.
        assert_eq!(contract.get_deposit(acc, accounts(1)).0, 1_000_000 - 7);
    }
}
