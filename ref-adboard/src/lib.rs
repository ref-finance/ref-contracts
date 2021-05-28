/*!
* Ref-Adboard
*
* lib.rs is the main entry point.
*/
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::{ValidAccountId};
use near_sdk::collections::{UnorderedSet, UnorderedMap, Vector};
use near_sdk::{env, near_bindgen, Balance, AccountId, PanicOnDefault, Timestamp};
use crate::utils::*;
// use near_sdk::BorshStorageKey;

mod utils;
mod owner;
mod token_receiver;
mod view;

near_sdk::setup_alloc!();


#[derive(BorshSerialize, BorshDeserialize)]
#[cfg_attr(feature = "test", derive(Clone))]
pub struct PaymentItem {
    pub amount: u128,
    pub token_id: AccountId,
    pub receiver_id: AccountId,
}

#[derive(BorshSerialize, BorshDeserialize)]
#[cfg_attr(feature = "test", derive(Clone))]
pub struct FrameMetadata {

    pub token_price: u128,

    pub token_id: AccountId,

    pub owner: AccountId,

    pub protected_ts: Timestamp,
}

impl Default for FrameMetadata {
    fn default() -> Self {
        FrameMetadata {
            token_price: ONE_NEAR,
            token_id: String::from(WRAPPED_NEAR_CONTRACT),
            owner: String::from(CONTRACT_SELF),
            protected_ts: 0,
        }
    }
}

// wait for main project upgrade to near-sdk 3.1.0
// #[derive(BorshStorageKey, BorshSerialize)]
// pub enum StorageKeys {
//     Seed,
//     Farmer,
//     RewardInfo,
// }

#[derive(BorshDeserialize, BorshSerialize)]
pub struct ContractData {

    owner_id: AccountId,

    frames: UnorderedMap<FrameId, FrameMetadata>,

    frames_data: UnorderedMap<FrameId, String>,

    whitelist: UnorderedSet<AccountId>,

    failed_payments: Vector<PaymentItem>,

    frame_count: u16,

    trading_fee: u16,
}



/// Versioned contract data. Allows to easily upgrade contracts.
#[derive(BorshSerialize, BorshDeserialize)]
pub enum VersionedContractData {
    Current(ContractData),
}

impl VersionedContractData {}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {

    data: VersionedContractData,
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new(owner_id: ValidAccountId, frame_count: u16, trading_fee: u16) -> Self {
        assert!(!env::state_exists(), "Already initialized");
        Self {
            data: VersionedContractData::Current(ContractData {
                owner_id: owner_id.into(),
                frames: UnorderedMap::new(b"f".to_vec()),
                frames_data: UnorderedMap::new(b"d".to_vec()),
                whitelist: UnorderedSet::new(b"w".to_vec()),
                failed_payments: Vector::new(b"l".to_vec()),
                frame_count,
                trading_fee,
            }),
        }
    }

    pub fn edit_frame(&mut self, frame_id: FrameId, frame_data: String) {

        let metadata = self.data().frames.get(&frame_id).unwrap_or_default();

        assert_eq!(
            env::predecessor_account_id(),
            metadata.owner,
            "ERR_ONLY_OWNER_CAN_MODIFY"
        );

        self.data_mut().frames_data.insert(&frame_id, &frame_data);
    }
}

impl Contract {
    fn data(&self) -> &ContractData {
        match &self.data {
            VersionedContractData::Current(data) => data,
        }
    }

    fn data_mut(&mut self) -> &mut ContractData {
        match &mut self.data {
            VersionedContractData::Current(data) => data,
        }
    }
}

#[cfg(test)]
mod tests {

    use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::{testing_env, Balance, MockedBlockchain, BlockHeight};
    use near_sdk::json_types::{ValidAccountId, U64, U128};
    use near_contract_standards::storage_management::{StorageBalance, StorageManagement};

    use super::*;

    fn setup_contract() -> (VMContextBuilder, Contract) {
        let mut context = VMContextBuilder::new();
        testing_env!(context.predecessor_account_id(accounts(0)).build());
        let contract = Contract::new(accounts(0), 500, 15);
        (context, contract)
    }

}