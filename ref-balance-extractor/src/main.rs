mod exchange;
mod farming;
mod lookup_map;
mod state;
mod unordered_map;
mod unordered_set;
mod utils;
mod vector;

use crate::exchange::*;
use crate::farming::*;
use crate::lookup_map::*;
use crate::state::*;
use crate::unordered_map::*;
use crate::unordered_set::*;
use crate::utils::*;
use crate::vector::*;

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::{Base64VecU8, U128};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::serde_json;
use near_sdk::{AccountId, Balance};
use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::io::Write;

// Deposits were generated using the following query:
//
// select predecessor_account_id,
// 		args->'args_json'->>'sender_id' as sender_id,
//        SUM(CAST(args->'args_json'->>'amount' as numeric)) amount_sum
// from action_receipt_actions join receipts using(receipt_id) join execution_outcomes using(receipt_id)
// where
//     action_kind = 'FUNCTION_CALL' and
//     receiver_account_id = 'ref-finance.near' and
//     included_in_block_timestamp >= 1628939340182442322 and
//     args->>'method_name' = 'ft_on_transfer' and
//     status = 'SUCCESS_VALUE'
//     group by predecessor_account_id, sender_id
//     order by predecessor_account_id, amount_sum desc

// Withdrawals were generated using the following query:
//
// select args->'args_json'->>'token_id' token_id,
//        predecessor_account_id,
//        SUM(CAST(args->'args_json'->>'amount' as numeric)) amount_sum
// from action_receipt_actions join receipts using(receipt_id) join execution_outcomes using(receipt_id)
// where
//     action_kind = 'FUNCTION_CALL' and
//     receiver_account_id = 'ref-finance.near' and
//     included_in_block_timestamp >= 1628939340182442322 and
//     args->>'method_name' = 'withdraw' and
//         status = 'SUCCESS_VALUE'
//
//     group by predecessor_account_id, token_id
//     order by token_id, amount_sum desc

// States were extracted at the block height: 45195764

lazy_static_include::lazy_static_include_bytes! {
    REF_FINANCE_STATE => "res/ref-finance.near.state.45195764.json",
    REF_FARMING_STATE => "res/ref-farming.near.state.45195764.json",
    DEPOSITS =>  "res/deposit_success.json",
    WITHDRAWALS =>  "res/withdraw_success.json",
}

pub type TokenAccountId = AccountId;

#[derive(Debug, Default)]
pub struct TokenBalance {
    internal: Balance,
    liquidity: Balance,
    deposits: Balance,
    withdrawals: Balance,
}

#[derive(Debug, Default)]
pub struct InternalAccount {
    pub account_id: AccountId,
    pub near_amount: Balance,
    pub tokens: HashMap<TokenAccountId, TokenBalance>,
}

#[derive(Clone, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct DepositOrWithdrawal {
    #[serde(rename = "c0")]
    pub token_account_id: TokenAccountId,
    #[serde(rename = "c1")]
    pub account_id: AccountId,
    #[serde(rename = "c2")]
    pub amount: U128,
}

fn format_token(token_account_id: &TokenAccountId, amount: Balance) -> String {
    let precision = if token_account_id == "wrap.near" {
        24
    } else {
        18
    };
    let integer = amount / 10u128.pow(precision);
    format!(
        "{}.{}",
        integer,
        (amount - integer) / 10u128.pow(precision - 3)
    )
}

const REF_EXCHANGE_ACCOUNT_ID: &str = "ref-finance.near";
const REF_FARMING_ACCOUNT_ID: &str = "ref-farming.near";

fn main() {
    let ref_exchange_contract: RefExchangeContract = {
        let mut state = parse_json_state(&REF_FINANCE_STATE);
        let mut contract =
            RefExchangeContract::try_from_slice(&state.remove(&b"STATE".to_vec()).unwrap())
                .unwrap();
        contract.parse(&mut state);
        assert!(state.is_empty());
        contract
    };

    // Extracting native balances
    let mut accounts: HashMap<_, _> = ref_exchange_contract
        .accounts
        .data
        .into_iter()
        .map(|(account_id, account)| {
            (
                account_id.clone(),
                InternalAccount {
                    account_id,
                    near_amount: account.near_amount,
                    tokens: account
                        .tokens
                        .into_iter()
                        .map(|(token_account_id, balance)| {
                            (
                                token_account_id,
                                TokenBalance {
                                    internal: balance,
                                    liquidity: 0,
                                    deposits: 0,
                                    withdrawals: 0,
                                },
                            )
                        })
                        .collect(),
                },
            )
        })
        .collect();
    println!("Num accounts: {}", accounts.len());

    let mut pools_data = ref_exchange_contract.pools.data;

    let ref_farming_contract: RefFarmingContractData = {
        let mut state = parse_json_state(&REF_FARMING_STATE);
        let mut contract =
            RefFarmingContract::try_from_slice(&state.remove(&b"STATE".to_vec()).unwrap()).unwrap();
        contract.parse(&mut state);
        assert!(state.is_empty());
        match contract.data {
            VersionedContractData::Current(data) => data,
        }
    };

    // Extracting ref farm into pool data
    for (account_id, VersionedFarmer::V101(farmer)) in ref_farming_contract.farmers.data.into_iter()
    {
        for (seed_id, balance) in farmer.seeds.into_iter() {
            let (ref_exchange_account_id, token_id) = parse_seed_id(&seed_id);
            assert_ne!(ref_exchange_account_id, token_id);
            assert_eq!(ref_exchange_account_id, REF_EXCHANGE_ACCOUNT_ID);
            let pool_index: u64 = str::parse(&token_id).unwrap();
            let Pool::SimplePool(pool) = pools_data.get_mut(pool_index as usize).unwrap();

            {
                let ref_farming_balance = pool
                    .shares
                    .data
                    .get_mut(REF_FARMING_ACCOUNT_ID)
                    .expect("Farming account is missing");
                *ref_farming_balance -= balance;
            }
            {
                let account_balance = pool.shares.data.entry(account_id.clone()).or_default();
                *account_balance += balance;
            }
        }
    }

    // Extracting liquidity
    for Pool::SimplePool(pool) in pools_data {
        // Verify total num_shares
        let mut total_shares = 0;
        for (account_id, shares) in pool.shares.data.into_iter() {
            if shares == 0 {
                continue;
            }
            total_shares += shares;

            let account: &mut InternalAccount =
                accounts.entry(account_id.clone()).or_insert_with(|| {
                    let mut account = InternalAccount::default();
                    account.account_id = account_id.clone();
                    println!("Found unregistered account in liquidity {}", account_id);
                    account
                });

            for (token_account_id, &balance) in
                pool.token_account_ids.iter().zip(pool.amounts.iter())
            {
                let amount = (U256::from(balance) * U256::from(shares)
                    / U256::from(pool.shares_total_supply))
                .as_u128();
                let balances = account.tokens.entry(token_account_id.clone()).or_default();
                balances.liquidity += amount;
            }
        }
        assert!(total_shares <= pool.shares_total_supply);
    }

    // Extracting deposits
    let deposits: Vec<DepositOrWithdrawal> = serde_json::from_slice(&DEPOSITS).unwrap();
    for DepositOrWithdrawal {
        token_account_id,
        account_id,
        amount,
    } in deposits
    {
        let account: &mut InternalAccount =
            accounts.entry(account_id.clone()).or_insert_with(|| {
                let mut account = InternalAccount::default();
                account.account_id = account_id.clone();
                println!("Found unregistered account in deposits {}", account_id);
                account
            });

        let balances = account.tokens.entry(token_account_id.clone()).or_default();
        balances.deposits += amount.0;
    }

    // Extracting withdrawals
    let withdrawals: Vec<DepositOrWithdrawal> = serde_json::from_slice(&WITHDRAWALS).unwrap();
    for DepositOrWithdrawal {
        token_account_id,
        account_id,
        amount,
    } in withdrawals
    {
        if let Some(account) = accounts.get_mut(&account_id) {
            let balances = account.tokens.entry(token_account_id.clone()).or_default();
            balances.withdrawals += amount.0;
        } else {
            println!("Missing withdrawal account {}", account_id);
        }
    }

    for account in accounts.values() {
        for (token_account_id, balances) in account.tokens.iter() {
            let total_input = balances.internal + balances.liquidity + balances.deposits;
            let total_output = balances.withdrawals;
            if total_input < total_output {
                println!(
                    "{},{},{}",
                    account.account_id,
                    token_account_id,
                    format_token(&token_account_id, total_output - total_input)
                );
            }
        }
    }

    let mut tokens: BTreeMap<TokenAccountId, Balance> = BTreeMap::new();

    println!("Aggregated token balances");

    for account in accounts.values() {
        for (token_account_id, balances) in account.tokens.iter() {
            let total_input = balances.internal + balances.liquidity + balances.deposits;
            let total_output = balances.withdrawals;
            if total_input > total_output {
                *tokens.entry(token_account_id.clone()).or_default() += total_input - total_output;
            }
        }
    }

    for (token_account_id, balance) in tokens.into_iter() {
        println!("{},{}", token_account_id, balance);
    }

    let output_file = "output/balances.csv";
    let mut file = File::create(output_file).expect("Failed to create the output file");
    for account in accounts.values() {
        for (token_account_id, balances) in account.tokens.iter() {
            let total_input = balances.internal + balances.liquidity + balances.deposits;
            let total_output = balances.withdrawals;
            if total_input > total_output {
                writeln!(
                    file,
                    "{},{},{}",
                    account.account_id,
                    token_account_id,
                    total_input - total_output
                )
                .unwrap();
            }
        }
    }
}
