use crate::errors::*;
use crate::*;
use near_sdk::{env, json_types::U128, AccountId, Balance};

/// an virtual inner account (won't appear in storage), used for instant swap
pub const VIRTUAL_ACC: &str = "@";
/// a special inner account, used to store all inner MFT when they act as any pool's backend assets 
pub const MFT_LOCKER: &str = "_MFT_LOCKER@";

/// Message parameters to receive via token function call.
/// used for instant swap.
#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[serde(untagged)]
pub(crate) enum TokenReceiverMessage {
    /// Alternative to deposit + execute actions call.
    Execute {
        referral_id: Option<ValidAccountId>,
        /// List of sequential actions.
        actions: Vec<Action>,
    },
}

/// All kinds of token accepted in the contract.
/// basically, for mft token, the format is {token_contract_id:inner_id}
/// for nep-141 token, the format is {token_contract_id}
pub enum TokenType {
    Nep141 {token_id: AccountId},
    InnerMFT {pool_id: u64},
    OuterMFT { token_contract_id: AccountId, inner_id: u64},
    Illegal,
}

impl TokenType {
    pub fn parse_token(token: &String) -> Self {
        let parts: Vec<&str> = token.split(":").collect();
        if parts.len() == 2 {
            if let Ok(pool_id) = str::parse::<u64>(parts[1]) {
                if parts[0].to_string() == env::current_account_id() || parts[0].to_string() == "" {
                    TokenType::InnerMFT {pool_id,}
                } else {
                    // outer mft
                    TokenType::OuterMFT {
                        token_contract_id: parts[0].to_string(),
                        inner_id: pool_id,
                    }
                }
            } else {
                TokenType::Illegal
            }
        } else { 
            TokenType::Nep141 {token_id: parts[0].to_string()}
        }
    }
}

/// Internal methods implementation.
impl Contract {

    pub(crate) fn assert_contract_running(&self) {
        match self.state {
            RunningState::Running => (),
            _ => env::panic(ERR51_CONTRACT_PAUSED.as_bytes()),
        };
    }

    /// Check how much storage taken costs and refund the left over back.
    pub(crate) fn internal_check_storage(&self, prev_storage: StorageUsage) {
        let storage_cost = env::storage_usage()
            .checked_sub(prev_storage)
            .unwrap_or_default() as Balance
            * env::storage_byte_cost();

        let refund = env::attached_deposit()
            .checked_sub(storage_cost)
            .expect(
                format!(
                    "ERR_STORAGE_DEPOSIT need {}, attatched {}", 
                    storage_cost, env::attached_deposit()
                ).as_str()
            );
        if refund > 0 {
            Promise::new(env::predecessor_account_id()).transfer(refund);
        }
    }

    /// Adds given pool to the list and returns it's id.
    /// If there is not enough attached balance to cover storage, fails.
    /// If too much attached - refunds it back.
    pub(crate) fn internal_add_pool(&mut self, mut pool: Pool) -> u64 {
        let prev_storage = env::storage_usage();
        let id = self.pools.len() as u64;
        // exchange share was registered at creation time
        pool.share_register(&env::current_account_id());
        self.pools.push(&pool);
        self.internal_check_storage(prev_storage);
        id
    }

    /// Execute sequence of actions on given account. Modifies passed account.
    /// Returns result of the last action.
    pub(crate) fn internal_execute_actions(
        &mut self,
        account: &mut Account,
        referral_id: &Option<AccountId>,
        actions: &[Action],
        prev_result: ActionResult,
        user_id: &AccountId,
    ) -> ActionResult {
        let mut result = prev_result;
        for action in actions {
            result = self.internal_execute_action(account, referral_id, action, result, user_id);
        }
        result
    }

    /// Executes single action on given account. Modifies passed account. Returns a result based on type of action.
    pub(crate) fn internal_execute_action(
        &mut self,
        account: &mut Account,
        referral_id: &Option<AccountId>,
        action: &Action,
        prev_result: ActionResult,
        user_id: &AccountId,
    ) -> ActionResult {
        match action {
            Action::Swap(swap_action) => {
                let amount_in = swap_action
                    .amount_in
                    .map(|value| value.0)
                    .unwrap_or_else(|| prev_result.to_amount());

                // handle token_in
                match TokenType::parse_token(&swap_action.token_in) {
                    TokenType::Nep141 {token_id} => {
                        account.withdraw(&token_id, amount_in);
                    },
                    TokenType::InnerMFT {pool_id} => {
                        let token_id = format!(":{}", pool_id);
                        self.internal_mft_transfer(token_id, user_id, &String::from(MFT_LOCKER), amount_in, None);
                    },
                    TokenType::OuterMFT {token_contract_id: _, inner_id: _} => {
                        account.withdraw(&swap_action.token_in, amount_in);
                    },
                    TokenType::Illegal => {env::panic("ERR_TOKEN_INVALID".as_bytes());},
                }

                // do action
                let amount_out = self.internal_pool_swap(
                    swap_action.pool_id,
                    &swap_action.token_in,
                    amount_in,
                    &swap_action.token_out,
                    swap_action.min_amount_out.0,
                    referral_id,
                );

                // handle token_out
                match TokenType::parse_token(&swap_action.token_out) {
                    TokenType::Nep141 {token_id} => {
                        account.deposit(&token_id, amount_out);
                    },
                    TokenType::InnerMFT {pool_id} => {
                        let token_id = format!(":{}", pool_id);
                        self.internal_mft_transfer(token_id, &String::from(MFT_LOCKER), user_id, amount_out, None);
                    },
                    TokenType::OuterMFT {token_contract_id: _, inner_id: _} => {
                        account.deposit(&swap_action.token_out, amount_out);
                    },
                    TokenType::Illegal => {env::panic("ERR_TOKEN_INVALID".as_bytes());},
                }

                // [AUDIT_02]
                ActionResult::Amount(U128(amount_out))
            }
        }
    }

    /// Swaps given amount_in of token_in into token_out via given pool.
    /// Should be at least min_amount_out or swap will fail (prevents front running and other slippage issues).
    pub(crate) fn internal_pool_swap(
        &mut self,
        pool_id: u64,
        token_in: &AccountId,
        amount_in: u128,
        token_out: &AccountId,
        min_amount_out: u128,
        referral_id: &Option<AccountId>,
    ) -> u128 {
        let mut pool = self.pools.get(pool_id).expect("ERR_NO_POOL");
        let amount_out = pool.swap(
            token_in,
            amount_in,
            token_out,
            min_amount_out,
            AdminFees {
                exchange_fee: self.exchange_fee,
                exchange_id: env::current_account_id(),
                referral_fee: self.referral_fee,
                referral_id: referral_id.clone(),
            },
        );
        self.pools.replace(pool_id, &pool);
        amount_out
    }

    /// Executes set of actions on virtual account.
    /// Returns amounts to send to the sender directly.
    pub(crate) fn internal_direct_actions(
        &mut self,
        token_in: AccountId,
        amount_in: Balance,
        referral_id: Option<AccountId>,
        actions: &[Action],
        user_id: &AccountId,
    ) -> Vec<(AccountId, Balance)> {

        // let @ be the virtual account
        let mut account: Account = Account::new(&String::from(VIRTUAL_ACC));
        
        match TokenType::parse_token(&token_in) {
            TokenType::Nep141 {token_id} => {
                account.deposit(&token_id, amount_in);
            },
            TokenType::InnerMFT {pool_id: _} => {
                // inner mft, already deposit, no action needed
            },
            TokenType::OuterMFT {token_contract_id: _, inner_id: _} => {
                account.deposit(&token_in, amount_in);
            },
            TokenType::Illegal => {env::panic("ERR_TOKEN_INVALID".as_bytes());},
        }

        let _ = self.internal_execute_actions(
            &mut account,
            &referral_id,
            &actions,
            ActionResult::Amount(U128(amount_in)),
            user_id,
        );

        let mut result = vec![];
        for (token, amount) in account.tokens.to_vec() {
            if amount > 0 {
                result.push((token.clone(), amount));
            }
        }
        account.tokens.clear();

        result
    }
}
