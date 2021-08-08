use std::collections::HashMap;

use near_contract_standards::fungible_token::core_impl::ext_fungible_token;
use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::LookupMap;
use near_sdk::json_types::{ValidAccountId, U128};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{
    env, ext_contract, near_bindgen, serde_json, AccountId, Balance, Gas, PanicOnDefault, Promise,
    PromiseOrValue, PromiseResult,
};

/// Amount of gas for fungible token transfers.
pub const GAS_FOR_FT_TRANSFER: Gas = 10_000_000_000_000;

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[serde(untagged)]
pub enum ReceiverMessage {
    Offer {
        taker: Option<ValidAccountId>,
        take_token_id: ValidAccountId,
        take_min_amount: U128,
    },
    Take {
        offer_id: u32,
    },
}

#[ext_contract(ext_self)]
pub trait RefEscrow {
    fn exchange_callback_post_withdraw(
        &mut self,
        token_id: AccountId,
        sender_id: AccountId,
        amount: U128,
    );
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct Offer {
    pub offerer: AccountId,
    /// Optionally only a single taker can take this offer.
    pub taker: Option<AccountId>,
    pub offer_token_id: AccountId,
    pub offer_amount: Balance,
    pub take_token_id: AccountId,
    pub take_min_amount: Balance,
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct Account {
    pub amounts: HashMap<AccountId, Balance>,
}

impl Account {
    pub fn deposit(&mut self, token_id: &AccountId, amount: Balance) {
        *self.amounts.entry(token_id.clone()).or_insert(0) += amount;
    }
}

#[near_bindgen]
#[derive(BorshSerialize, BorshDeserialize, PanicOnDefault)]
pub struct Contract {
    last_offer_id: u32,
    offers: LookupMap<u32, Offer>,
    accounts: LookupMap<AccountId, Account>,
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new() -> Self {
        Self {
            last_offer_id: 0,
            offers: LookupMap::new(b"o"),
            accounts: LookupMap::new(b"a"),
        }
    }

    pub fn withdraw(&mut self, token_id: ValidAccountId, amount: U128) -> Promise {
        let sender_id = env::predecessor_account_id();
        let account = self.accounts.get(&sender_id).expect("ERR_MISSING_ACCOUNT");
        assert!(
            *account
                .amounts
                .get(token_id.as_ref())
                .expect("ERR_MISSING_TOKEN")
                >= amount.0,
            "ERR_NOT_ENOUGH_AMOUNT"
        );
        ext_fungible_token::ft_transfer(
            sender_id.clone(),
            amount,
            None,
            token_id.as_ref(),
            1,
            GAS_FOR_FT_TRANSFER,
        )
        .then(ext_self::exchange_callback_post_withdraw(
            token_id.as_ref().clone(),
            sender_id.clone(),
            amount,
            &env::current_account_id(),
            0,
            GAS_FOR_FT_TRANSFER,
        ))
    }

    #[private]
    pub fn exchange_callback_post_withdraw(
        &mut self,
        token_id: AccountId,
        sender_id: AccountId,
        amount: U128,
    ) {
        assert_eq!(
            env::promise_results_count(),
            1,
            "ERR_CALLBACK_POST_WITHDRAW_INVALID",
        );
        match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Successful(_) => {}
            PromiseResult::Failed => {
                // This reverts the changes from withdraw function. If account doesn't exit, deposits to the owner's account.
                if let Some(mut account) = self.accounts.get(&sender_id) {
                    account.deposit(&token_id, amount.0);
                    self.accounts.insert(&sender_id, &account);
                } else {
                    env::log(
                        format!(
                            "Account {} is not registered or not enough storage. Money are stuck in this contract.",
                            sender_id
                        )
                            .as_bytes(),
                    );
                }
            }
        };
    }
}

#[near_bindgen]
impl FungibleTokenReceiver for Contract {
    /// Callback on receiving tokens by this contract.
    /// `msg` format is JSON serialized `ReceiverMessage`.
    fn ft_on_transfer(
        &mut self,
        sender_id: ValidAccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128> {
        let token_id = env::predecessor_account_id();
        let message = serde_json::from_str::<ReceiverMessage>(&msg).expect("ERR_MSG_WRONG_FORMAT");
        match message {
            ReceiverMessage::Offer {
                taker,
                take_token_id,
                take_min_amount,
            } => {
                self.offers.insert(
                    &self.last_offer_id,
                    &Offer {
                        offerer: sender_id.as_ref().clone(),
                        taker: taker.map(|a| a.as_ref().clone()),
                        offer_token_id: token_id,
                        offer_amount: amount.0,
                        take_token_id: take_token_id.as_ref().clone(),
                        take_min_amount: take_min_amount.0,
                    },
                );
                self.last_offer_id += 1;
                PromiseOrValue::Value(U128(0))
            }
            ReceiverMessage::Take { offer_id } => {
                let offer = self.offers.get(&offer_id).expect("ERR_MISSING_OFFER");
                let mut offerer_account = self
                    .accounts
                    .get(&offer.offerer)
                    .expect("ERR_MISSING_ACCOUNT");
                let mut taker_account = self
                    .accounts
                    .get(sender_id.as_ref())
                    .expect("ERR_MISSING_ACCOUNT");
                assert_eq!(offer.take_token_id, token_id, "ERR_WRONG_TAKE_TOKEN");
                assert!(amount.0 >= offer.take_min_amount, "ERR_NOT_ENOUGH_AMOUNT");
                if let Some(taker) = offer.taker {
                    assert_eq!(&taker, sender_id.as_ref(), "ERR_INCORRECT_TAKER");
                }
                self.offers.remove(&offer_id);
                offerer_account.deposit(&offer.take_token_id, amount.0);
                taker_account.deposit(&offer.offer_token_id, offer.offer_amount);
                self.accounts.insert(&offer.offerer, &offerer_account);
                self.accounts.insert(sender_id.as_ref(), &taker_account);

                PromiseOrValue::Value(U128(0))
            }
        }
    }
}
