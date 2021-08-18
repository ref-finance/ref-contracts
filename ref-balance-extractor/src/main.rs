mod exchange;
mod farming;
mod fungible_token;
mod lazy_option;
mod lookup_map;
mod skyward;
mod state;
mod unordered_map;
mod unordered_set;
mod utils;
mod vector;

use crate::exchange::*;
use crate::farming::*;
use crate::fungible_token::*;
use crate::lazy_option::*;
use crate::lookup_map::*;
use crate::skyward::*;
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
// The skyward state is queried later at: 45295764

// Skyward token transfers generated using the following query:
//
// select receiver_account_id,
// 	   args->'args_json'->>'receiver_id' receiver_id,
//        args->'args_json'->>'amount' amount
// from action_receipt_actions join receipts using(receipt_id) join execution_outcomes using(receipt_id)
// where
//     action_kind = 'FUNCTION_CALL' and
//     status = 'SUCCESS_VALUE' and
//     predecessor_account_id = 'skyward.near' and
//     receiver_account_id = 'token.ref-finance.near' and
//     included_in_block_timestamp >= 1628939340182442322 and
//     included_in_block_timestamp <= 1629040887454994457 and
// 	    args->>'method_name' = 'ft_transfer'

lazy_static_include::lazy_static_include_bytes! {
    REF_FINANCE_STATE => "res/ref-finance.near.state.45195764.json",
    REF_FARMING_STATE => "res/ref-farming.near.state.45195764.json",
    REF_TOKEN_STATE => "res/token.ref-finance.near.state.45195764.json",
    DEPOSITS =>  "res/deposit_success.json",
    WITHDRAWALS =>  "res/withdraw_success.json",
    SKYWARD_STATE => "res/skyward.near.state.45295764.json",
    SKYWARD_REF_TOKEN_WITHDRAWALS => "res/skyward_ref_withdrawal.json",
}

pub type TokenAccountId = AccountId;

#[derive(Debug, Default)]
pub struct TokenBalance {
    pub internal: Balance,
    pub liquidity: Balance,
    pub deposits: Balance,
    pub withdrawals: Balance,
    pub token_balance: Balance,
    pub skyward_balance: Balance,
    pub skyward_unclaimed: Balance,
    pub skyward_withdrawal: Balance,
}

impl TokenBalance {
    pub fn input(&self) -> Balance {
        self.internal
            + self.liquidity
            + self.deposits
            + self.token_balance
            + self.skyward_balance
            + self.skyward_unclaimed
            + self.skyward_withdrawal
    }
}

#[derive(Debug, Default)]
pub struct InternalAccount {
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

const REF_TOKEN_ID: &str = "token.ref-finance.near";

const SKYWARD_ACCOUNT_ID: &str = "skyward.near";

const REF_SALES: &[u64] = &[6, 7, 8];

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

    let ref_token_contract: TokenContract = {
        let mut state = parse_json_state(&REF_TOKEN_STATE);
        let mut contract =
            TokenContract::try_from_slice(&state.remove(&b"STATE".to_vec()).unwrap()).unwrap();
        contract.parse(&mut state);
        assert!(state.is_empty());
        contract
    };

    let skyward_contract: SkywardContract = {
        let mut state = parse_json_state(&SKYWARD_STATE);
        let mut contract =
            SkywardContract::try_from_slice(&state.remove(&b"STATE".to_vec()).unwrap()).unwrap();
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
                account_id,
                InternalAccount {
                    near_amount: account.near_amount,
                    tokens: account
                        .tokens
                        .into_iter()
                        .map(|(token_account_id, balance)| {
                            (
                                token_account_id,
                                TokenBalance {
                                    internal: balance,
                                    ..Default::default()
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

            let account: &mut InternalAccount = accounts.entry(account_id.clone()).or_default();

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
        if token_account_id == REF_TOKEN_ID {
            continue;
        }
        let account: &mut InternalAccount = accounts.entry(account_id.clone()).or_default();

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
        if token_account_id == REF_TOKEN_ID {
            continue;
        }
        let account: &mut InternalAccount = accounts.entry(account_id.clone()).or_default();
        let balances = account.tokens.entry(token_account_id.clone()).or_default();
        balances.withdrawals += amount.0;
    }

    println!("Processing REF token state");
    let mut skyward_ref_balance = 0;

    for (account_id, balance) in ref_token_contract.ft.accounts.data.into_iter() {
        if account_id == REF_EXCHANGE_ACCOUNT_ID {
            continue;
        }
        if account_id == SKYWARD_ACCOUNT_ID {
            skyward_ref_balance = balance;
            continue;
        }
        let account: &mut InternalAccount = accounts.entry(account_id.clone()).or_default();

        let balances = account.tokens.entry(REF_TOKEN_ID.to_string()).or_default();
        balances.token_balance += balance;
    }

    println!("Processing Skyward state");

    for (account_id, VAccount::Current(mut skyward_account)) in skyward_contract.accounts.data {
        let account: &mut InternalAccount = accounts.entry(account_id.clone()).or_default();
        for (token_account_id, balance) in skyward_account.balances.data {
            let balances = account.tokens.entry(token_account_id.clone()).or_default();
            balances.skyward_balance += balance;
        }
        for ref_sale_id in REF_SALES {
            if let Some(VSubscription::Current(mut sub)) =
                skyward_account.subs.data.remove(&ref_sale_id)
            {
                let sale = match skyward_contract.sales.data.get(&ref_sale_id) {
                    Some(VSale::Current(sale)) => sale,
                    _ => unreachable!(),
                };
                let unclaimed_balances = sub.touch(&sale);
                assert_eq!(unclaimed_balances.len(), 1);
                let balances = account.tokens.entry(REF_TOKEN_ID.to_string()).or_default();
                balances.skyward_unclaimed += unclaimed_balances[0];
            }
        }
    }
    // Skyward Treasury
    {
        let account: &mut InternalAccount =
            accounts.entry(SKYWARD_ACCOUNT_ID.to_string()).or_default();
        let balances = account.tokens.entry(REF_TOKEN_ID.to_string()).or_default();
        balances.skyward_balance += skyward_contract
            .treasury
            .balances
            .data
            .get(REF_TOKEN_ID)
            .unwrap_or(&0);
    }

    println!("Processing Skyward withdrawals");
    let skyward_withdrawals: Vec<DepositOrWithdrawal> =
        serde_json::from_slice(&SKYWARD_REF_TOKEN_WITHDRAWALS).unwrap();
    for DepositOrWithdrawal {
        token_account_id,
        account_id,
        amount,
    } in skyward_withdrawals
    {
        let account: &mut InternalAccount = accounts.entry(account_id.clone()).or_default();

        let balances = account.tokens.entry(token_account_id.clone()).or_default();
        balances.skyward_withdrawal += amount.0;
    }

    //
    // for account in accounts.values() {
    //     for (token_account_id, balances) in account.tokens.iter() {
    //         let total_input = balances.input();
    //         let total_output = balances.withdrawals;
    //         if total_input < total_output {
    //             println!(
    //                 "{},{},{}",
    //                 account.account_id,
    //                 token_account_id,
    //                 format_token(&token_account_id, total_output - total_input)
    //             );
    //         }
    //     }
    // }

    let mut tokens: BTreeMap<TokenAccountId, Balance> = BTreeMap::new();

    println!("Aggregated token balances");

    for account in accounts.values() {
        for (token_account_id, balances) in account.tokens.iter() {
            let total_input = balances.input();
            let total_output = balances.withdrawals;
            if total_input > total_output {
                *tokens.entry(token_account_id.clone()).or_default() += total_input - total_output;
            }
        }
    }

    println!("{},{}", REF_TOKEN_ID, tokens.get(REF_TOKEN_ID).unwrap());

    let mut ref_balances = Vec::new();
    for (account_id, account) in accounts.iter() {
        if let Some(ref_balance) = account.tokens.get(REF_TOKEN_ID) {
            ref_balances.push((
                account_id.clone(),
                ref_balance.input() - ref_balance.withdrawals,
            ));
        }
    }

    ref_balances.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    let output_file = "output/ref_balances.csv";
    let mut file = File::create(output_file).expect("Failed to create the output file");
    for (account_id, balance) in ref_balances {
        writeln!(file, "{},{}", account_id, balance).unwrap();
    }

    // for (token_account_id, balance) in tokens.into_iter() {
    //     println!("{},{}", token_account_id, balance);
    // }

    let output_file = "output/balances.csv";
    let mut file = File::create(output_file).expect("Failed to create the output file");
    for (account_id, account) in accounts {
        for (token_account_id, balances) in account.tokens.iter() {
            let total_input = balances.input();
            let total_output = balances.withdrawals;
            if total_input > total_output {
                writeln!(
                    file,
                    "{},{},{}",
                    account_id,
                    token_account_id,
                    total_input - total_output
                )
                .unwrap();
            }
        }
    }
}
