use near_contract_standards::fungible_token::core_impl::ext_fungible_token;
use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;
use near_contract_standards::storage_management::{
    StorageBalance, StorageBalanceBounds, StorageManagement,
};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::LookupMap;
use near_sdk::json_types::{ValidAccountId, WrappedDuration, WrappedTimestamp, U128};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{
    assert_one_yocto, env, ext_contract, near_bindgen, serde_json, AccountId, Gas, PanicOnDefault,
    Promise, PromiseOrValue, PromiseResult,
};

pub use crate::account::{Account, AccountManager};

mod account;

near_sdk::setup_alloc!();

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
        min_offer_time: WrappedDuration,
        max_offer_time: WrappedDuration,
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

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct Offer {
    pub offerer: AccountId,
    /// Optionally only a single taker can take this offer.
    pub taker: Option<AccountId>,
    pub offer_token_id: AccountId,
    pub offer_amount: U128,
    pub take_token_id: AccountId,
    pub take_min_amount: U128,
    pub offer_min_expiry: WrappedTimestamp,
    pub offer_max_expiry: WrappedTimestamp,
}

#[near_bindgen]
#[derive(BorshSerialize, BorshDeserialize, PanicOnDefault)]
pub struct Contract {
    last_offer_id: u32,
    offers: LookupMap<u32, Offer>,
    account_manager: AccountManager<Account>,
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new() -> Self {
        Self {
            last_offer_id: 0,
            offers: LookupMap::new(b"o".to_vec()),
            account_manager: AccountManager::new(),
        }
    }

    /// Withdraw funds from the account.
    #[payable]
    pub fn withdraw(&mut self, token_id: ValidAccountId, amount: U128) -> Promise {
        assert_one_yocto();
        let sender_id = env::predecessor_account_id();
        self.account_manager.update_account(&sender_id, |account| {
            account.withdraw(token_id.as_ref(), amount.0);
        });
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
                if let Some(mut account) = self.account_manager.get_account(&sender_id) {
                    account.deposit(&token_id, amount.0);
                    self.account_manager.set_account(&sender_id, &account);
                } else {
                    // TODO: figure out where to send money in this case?
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

    /// Close offer. Only offerer can call this.
    /// Offer minimum expiry should pass to close it.
    /// Deposits money into the account for withdrawal.
    pub fn close_offer(&mut self, offer_id: u32) {
        let sender_id = env::predecessor_account_id();
        let offer = self.offers.get(&offer_id).expect("ERR_MISSING_OFFER");
        assert_eq!(offer.offerer, sender_id, "ERR_NOT_OFFERER");
        assert!(
            env::block_timestamp() >= offer.offer_min_expiry.0,
            "ERR_CAN_NOT_CLOSE_OFFER_YET"
        );
        self.offers.remove(&offer_id);
        self.account_manager.update_account(&sender_id, |account| {
            account.remove_offer();
            account.deposit(&offer.offer_token_id, offer.offer_amount.0);
        });
    }

    pub fn get_offer(&self, offer_id: u32) -> Offer {
        self.offers.get(&offer_id).expect("ERR_MISSING_OFFER")
    }

    pub fn get_last_offer_id(&self) -> u32 {
        self.last_offer_id
    }

    pub fn get_offers(&self, from_index: u32, limit: u32) -> Vec<Offer> {
        (from_index..std::cmp::min(from_index + limit, self.last_offer_id))
            .map(|index| self.get_offer(index))
            .collect()
    }

    pub fn get_account(&self, account_id: ValidAccountId) -> Account {
        self.account_manager.get_account_or(account_id.as_ref())
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
                min_offer_time,
                max_offer_time,
            } => {
                // Account must be registered and have enough space for an extra offer.
                self.account_manager
                    .update_account(&sender_id.as_ref(), move |account| {
                        account.add_offer();
                    });
                self.offers.insert(
                    &self.last_offer_id,
                    &Offer {
                        offerer: sender_id.as_ref().clone(),
                        taker: taker.map(|a| a.as_ref().clone()),
                        offer_token_id: token_id.clone(),
                        offer_amount: amount,
                        take_token_id: take_token_id.as_ref().clone(),
                        take_min_amount,
                        offer_min_expiry: (env::block_timestamp() + min_offer_time.0).into(),
                        offer_max_expiry: (env::block_timestamp() + max_offer_time.0).into(),
                    },
                );
                env::log(
                    format!(
                        "Offer {}: offering {} {} for {} {}",
                        self.last_offer_id, amount.0, token_id, take_min_amount.0, take_token_id
                    )
                    .as_bytes(),
                );
                self.last_offer_id += 1;
                PromiseOrValue::Value(U128(0))
            }
            ReceiverMessage::Take { offer_id } => {
                let offer = self.offers.get(&offer_id).expect("ERR_MISSING_OFFER");
                assert!(
                    env::block_timestamp() < offer.offer_max_expiry.0,
                    "ERR_OFFER_EXPIRED"
                );
                assert_ne!(
                    &offer.offerer,
                    sender_id.as_ref(),
                    "ERR_OFFER_CAN_NOT_SELF_TAKE"
                );
                let mut offerer_account = self.account_manager.get_account_or(&offer.offerer);
                let mut taker_account = self.account_manager.get_account_or(sender_id.as_ref());
                assert_eq!(offer.take_token_id, token_id, "ERR_WRONG_TAKE_TOKEN");
                assert!(amount.0 >= offer.take_min_amount.0, "ERR_NOT_ENOUGH_AMOUNT");
                if let Some(taker) = offer.taker {
                    assert_eq!(&taker, sender_id.as_ref(), "ERR_INCORRECT_TAKER");
                }
                self.offers.remove(&offer_id);
                offerer_account.remove_offer();
                offerer_account.deposit(&offer.take_token_id, amount.0);
                taker_account.deposit(&offer.offer_token_id, offer.offer_amount.0);
                self.account_manager
                    .set_account(&offer.offerer, &offerer_account);
                self.account_manager
                    .set_account(sender_id.as_ref(), &taker_account);

                PromiseOrValue::Value(U128(0))
            }
        }
    }
}

#[near_bindgen]
impl StorageManagement for Contract {
    #[payable]
    fn storage_deposit(
        &mut self,
        account_id: Option<ValidAccountId>,
        registration_only: Option<bool>,
    ) -> StorageBalance {
        self.account_manager
            .internal_storage_deposit(account_id, registration_only)
    }

    #[payable]
    fn storage_withdraw(&mut self, amount: Option<U128>) -> StorageBalance {
        self.account_manager.internal_storage_withdraw(amount)
    }

    #[payable]
    fn storage_unregister(&mut self, force: Option<bool>) -> bool {
        self.account_manager.internal_storage_unregister(force)
    }

    fn storage_balance_bounds(&self) -> StorageBalanceBounds {
        self.account_manager.internal_storage_balance_bounds()
    }

    fn storage_balance_of(&self, account_id: ValidAccountId) -> Option<StorageBalance> {
        self.account_manager.internal_storage_balance_of(account_id)
    }
}
