use std::collections::HashMap;
use std::convert::TryInto;

use near_contract_standards::storage_management::{
    StorageBalance, StorageBalanceBounds, StorageManagement,
};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LookupMap, Vector};
use near_sdk::json_types::{ValidAccountId, U128};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{
    assert_one_yocto, env, log, near_bindgen, AccountId, Balance, PanicOnDefault, Promise,
};

use crate::pool::Pool;
use crate::simple_pool::SimplePool;
use crate::utils::{check_token_duplicates, ext_fungible_token, GAS_FOR_FT_TRANSFER, GAS_FOR_UPGRADE_CALL, GAS_FOR_DEPLOY_CALL};
pub use crate::views::PoolInfo;

mod pool;
mod simple_pool;
mod storage_impl;
mod token_receiver;
mod utils;
mod views;

near_sdk::setup_alloc!();

const MAX_ACCOUNT_LENGTH: u128 = 64;
const MIN_ACCOUNT_DEPOSIT_LENGTH: u128 = MAX_ACCOUNT_LENGTH + 16 + 4;

/// Single swap action.
#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct SwapAction {
    /// Pool which should be used for swapping.
    pub pool_id: u64,
    /// Token to swap from.
    pub token_in: ValidAccountId,
    /// Amount to exchange.
    /// If amount_in is None, it will take amount_out from previous step.
    /// Will fail if amount_in is None on the first step.
    pub amount_in: Option<U128>,
    /// Token to swap into.
    pub token_out: ValidAccountId,
    /// Required minimum amount of token_out.
    pub min_amount_out: U128,
}

/// Account deposits information and storage cost.
#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub struct AccountDeposit {
    /// Native amount sent to the exchange.
    /// Used for storage now, but in future can be used for trading as well.
    pub amount: Balance,
    /// Amounts of various tokens in this account.
    pub tokens: HashMap<AccountId, Balance>,
}

impl AccountDeposit {
    /// Adds amount to the balance of given token while checking that storage is covered.
    fn add(&mut self, token: AccountId, amount: Balance) {
        let prev_amount = *self.tokens.get(&token).unwrap_or(&0);
        self.tokens.insert(token, prev_amount + amount);
        assert!(
            self.storage_usage() <= self.amount,
            "ERR_INSUFFICIENT_STORAGE"
        );
    }

    /// Subtract from balance of given token, removes record if 0.
    fn sub(&mut self, token: AccountId, amount: Balance) {
        let value = *self
            .tokens
            .get(&token)
            .expect(&format!("ERR_MISSING_TOKEN:{}", token));
        assert!(value >= amount, format!("ERR_NOT_ENOUGH_TOKEN:{}", token));
        if value == amount {
            self.tokens.remove(&token);
        } else {
            self.tokens.insert(token, value - amount);
        }
    }

    fn storage_usage(&self) -> Balance {
        (MIN_ACCOUNT_DEPOSIT_LENGTH + self.tokens.len() as u128 * (MAX_ACCOUNT_LENGTH + 16))
            * env::storage_byte_cost()
    }
}

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
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new(owner_id: ValidAccountId, exchange_fee: u32, referral_fee: u32) -> Self {
        assert!(!env::state_exists(), "ERR_CONTRACT_IS_INITIALIZED");
        Self {
            owner_id: owner_id.as_ref().clone(),
            exchange_fee,
            referral_fee,
            pools: Vector::new(b"p".to_vec()),
            deposited_amounts: LookupMap::new(b"d".to_vec()),
        }
    }

    /// Change owner. Only can be called by owner.
    pub fn set_owner(&mut self, owner_id: ValidAccountId) {
        assert_eq!(
            env::predecessor_account_id(),
            self.owner_id,
            "ERR_NOT_ALLOWED"
        );
        self.owner_id = owner_id.as_ref().clone();
    }

    /// Get the owner of this account.
    pub fn get_owner(&self) -> AccountId {
        self.owner_id.clone()
    }

    /// Upgrades given contract. Only can be called by owner.
    /// if `migrate` is true, calls `migrate()` function right after deployment.
    pub fn upgrade(&self, #[serializer(borsh)] code: Vec<u8>, #[serializer(borsh)] migrate: bool) -> Promise {
        assert_eq!(env::predecessor_account_id(), self.owner_id, "ERR_NOT_ALLOWED");
        let mut promise = Promise::new(env::current_account_id()).deploy_contract(code);
        if migrate {
            promise = promise.function_call("migrate".as_bytes().to_vec(), vec![], 0, env::prepaid_gas() - GAS_FOR_UPGRADE_CALL - GAS_FOR_DEPLOY_CALL);
        }
        promise
    }

    /// Placeholder for migration function.
    #[init]
    pub fn migrate() {
        unimplemented!()
    }

    /// Adds new "Simple Pool" with given tokens and given fee.
    /// Attached NEAR should be enough to cover the added storage.
    #[payable]
    pub fn add_simple_pool(&mut self, tokens: Vec<ValidAccountId>, fee: u32) -> u32 {
        check_token_duplicates(&tokens);
        self.internal_add_pool(Pool::SimplePool(SimplePool::new(
            self.pools.len() as u32,
            tokens,
            fee + self.exchange_fee + self.referral_fee,
            self.exchange_fee,
            self.referral_fee,
        )))
    }

    /// Swaps given amount_in of token_in into token_out via given pool.
    /// Should be at least min_amount_out or swap will fail (prevents front running and other slippage issues).
    pub fn internal_swap(
        &mut self,
        sender_id: &AccountId,
        pool_id: u64,
        token_in: ValidAccountId,
        amount_in: U128,
        token_out: ValidAccountId,
        min_amount_out: U128,
        referral_id: Option<AccountId>,
    ) -> U128 {
        let mut deposits = self.internal_get_deposits(&sender_id);
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

    /// Execute set of swap actions between pools.
    /// If referrer provided, pays referral_fee to it.
    pub fn swap(&mut self, actions: Vec<SwapAction>, referral_id: Option<ValidAccountId>) -> U128 {
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
    pub fn add_liquidity(&mut self, pool_id: u64, amounts: Vec<U128>) {
        let sender_id = env::predecessor_account_id();
        let amounts: Vec<u128> = amounts.into_iter().map(|amount| amount.into()).collect();
        let mut pool = self.pools.get(pool_id).expect("ERR_NO_POOL");
        let mut deposits = self.internal_get_deposits(&sender_id);
        let tokens = pool.tokens();
        for i in 0..tokens.len() {
            deposits.sub(tokens[i].clone(), amounts[i]);
        }
        pool.add_liquidity(&sender_id, amounts);
        self.deposited_amounts.insert(&sender_id, &deposits);
        self.pools.replace(pool_id, &pool);
    }

    /// Remove liquidity from the pool into general pool of liquidity.
    pub fn remove_liquidity(&mut self, pool_id: u64, shares: U128, min_amounts: Vec<U128>) {
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
        let mut deposits = self.internal_get_deposits(&sender_id);
        for i in 0..tokens.len() {
            deposits.add(tokens[i].clone(), amounts[i]);
        }
        self.deposited_amounts.insert(&sender_id, &deposits);
    }

    /// Withdraws given token from the deposits of given user.
    #[payable]
    pub fn withdraw(&mut self, token_id: ValidAccountId, amount: U128) {
        assert_one_yocto();
        let amount: u128 = amount.into();
        let sender_id = env::predecessor_account_id();
        let mut deposits = self.deposited_amounts.get(&sender_id).unwrap();
        deposits.sub(token_id.as_ref().clone(), amount);
        self.deposited_amounts.insert(&sender_id, &deposits);
        ext_fungible_token::ft_transfer(
            sender_id.try_into().unwrap(),
            amount.into(),
            None,
            token_id.as_ref(),
            1,
            GAS_FOR_FT_TRANSFER,
        );
    }
}

/// Internal methods implementation.
impl Contract {
    /// Adds given pool to the list and returns it's id.
    /// If there is not enough attached balance to cover storage, fails.
    /// If too much attached - refunds it back.
    fn internal_add_pool(&mut self, pool: Pool) -> u32 {
        let prev_storage = env::storage_usage();
        let id = self.pools.len() as u32;
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

    /// Registers account in deposited amounts with given amount of $NEAR.
    /// If account already exists, adds amount to it.
    /// This should be used when it's known that storage is prepaid.
    fn internal_register_account(&mut self, account_id: &AccountId, amount: Balance) {
        let mut deposit_amount =
            self.deposited_amounts
                .get(&account_id)
                .unwrap_or_else(|| AccountDeposit {
                    amount: 0,
                    tokens: HashMap::default(),
                });
        deposit_amount.amount += amount;
        self.deposited_amounts.insert(&account_id, &deposit_amount);
    }

    /// Record deposit of some number of tokens to this contract.
    fn internal_deposit(&mut self, sender_id: &AccountId, token_id: &AccountId, amount: Balance) {
        let mut account_deposit = self
            .deposited_amounts
            .get(sender_id)
            .expect("ERR_NOT_REGISTERED");
        account_deposit.add(token_id.clone(), amount);
        self.deposited_amounts.insert(sender_id, &account_deposit);
    }

    /// Returns current balances across all tokens for given user.
    fn internal_get_deposits(&self, sender_id: &AccountId) -> AccountDeposit {
        self.deposited_amounts
            .get(sender_id)
            .expect("ERR_NO_DEPOSIT")
            .clone()
    }

    /// Returns current balance of given token for given user. If there is nothing recorded, returns 0.
    fn internal_get_deposit(&self, sender_id: &AccountId, token_id: &AccountId) -> Balance {
        self.internal_get_deposits(sender_id)
            .tokens
            .get(token_id)
            .cloned()
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::{testing_env, MockedBlockchain};
    use near_sdk_sim::to_yocto;

    use super::*;

    /// Creates contract and a pool with tokens with 0.3% of total fee.
    fn setup_contract(tokens: Vec<ValidAccountId>) -> (VMContextBuilder, Contract) {
        let mut context = VMContextBuilder::new();
        testing_env!(context.build());
        let mut contract = Contract::new(accounts(0), 4, 1);
        testing_env!(context
            .predecessor_account_id(accounts(3))
            .attached_deposit(env::storage_byte_cost() * 300)
            .build());
        contract.add_simple_pool(tokens, 25);
        (context, contract)
    }

    fn deposit_tokens(
        context: &mut VMContextBuilder,
        contract: &mut Contract,
        account_id: ValidAccountId,
        token_amounts: Vec<(ValidAccountId, Balance)>,
    ) {
        for (token_id, amount) in token_amounts {
            testing_env!(context
                .predecessor_account_id(token_id)
                .attached_deposit(1)
                .build());
            contract.ft_on_transfer(account_id.clone(), U128(amount), "".to_string());
        }
    }

    #[test]
    fn test_basics() {
        let one_near = 10u128.pow(24);
        let (mut context, mut contract) = setup_contract(vec![accounts(1), accounts(2)]);

        // add liquidity of (1,2) tokens and create 1st pool.
        testing_env!(context
            .predecessor_account_id(accounts(3))
            .attached_deposit(to_yocto("0.03"))
            .build());
        contract.storage_deposit(None, None);
        deposit_tokens(
            &mut context,
            &mut contract,
            accounts(3),
            vec![(accounts(1), 105 * one_near), (accounts(2), 110 * one_near)],
        );

        assert_eq!(
            contract.get_deposit(accounts(3), accounts(1)),
            (105 * one_near).into()
        );
        assert_eq!(
            contract.get_deposit(accounts(3), accounts(2)),
            (110 * one_near).into()
        );

        testing_env!(context.predecessor_account_id(accounts(3)).build());
        contract.add_liquidity(0, vec![U128(5 * one_near), U128(10 * one_near)]);
        assert_eq!(
            contract.get_pool_total_shares(0).0,
            crate::utils::INIT_SHARES_SUPPLY
        );

        // Get price from pool #0 1 -> 2 tokens.
        let expected_out = contract.get_return(0, accounts(1), one_near.into(), accounts(2));
        assert_eq!(expected_out.0, 1662497915624478906119726);

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

        contract.withdraw(accounts(1), contract.get_deposit(accounts(3), accounts(1)));
        assert_eq!(contract.get_deposit(accounts(3), accounts(1)).0, 0);
    }

    /// Should deny creating a pool with duplicate tokens.
    #[test]
    #[should_panic(expected = "ERR_TOKEN_DUPLICATES")]
    fn test_deny_duplicate_tokens_pool() {
        setup_contract(vec![accounts(1), accounts(1)]);
    }

    /// Deny pool with a single token
    #[test]
    #[should_panic(expected = "ERR_NOT_ENOUGH_TOKENS")]
    fn test_deny_single_token_pool() {
        setup_contract(vec![accounts(1)]);
    }

    /// Deny pool with a single token
    #[test]
    #[should_panic(expected = "ERR_TOO_MANY_TOKENS")]
    fn test_deny_too_many_tokens_pool() {
        setup_contract(vec![accounts(1), accounts(2), accounts(3)]);
    }
}
