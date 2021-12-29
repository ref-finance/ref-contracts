/*!
* Ref-Adboard
*
* lib.rs is the main entry point.
*/
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::{ValidAccountId, U128};
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

#[derive(BorshDeserialize, BorshSerialize)]
pub struct ContractData {

    owner_id: AccountId,

    amm_id: AccountId,

    default_token_id: AccountId,
    default_sell_balance: Balance,

    // in seconds
    protected_period: u16,

    trading_fee: u16,

    frame_count: u16,

    frames: UnorderedMap<FrameId, FrameMetadata>,

    frames_data: UnorderedMap<FrameId, String>,

    whitelist: UnorderedSet<AccountId>,

    failed_payments: Vector<PaymentItem>,
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
    pub fn new(
        owner_id: ValidAccountId, 
        amm_id: ValidAccountId, 
        default_token_id: ValidAccountId,
        default_sell_balance: U128,
        protected_period: u16,
        frame_count: u16, 
        trading_fee: u16,
    ) -> Self {
        assert!(!env::state_exists(), "Already initialized");
        Self {
            data: VersionedContractData::Current(ContractData {
                owner_id: owner_id.into(),
                amm_id: amm_id.into(),
                default_token_id: default_token_id.into(),
                default_sell_balance: default_sell_balance.into(),
                protected_period,
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

        let metadata = self.data().frames.get(&frame_id).unwrap_or(
            FrameMetadata {
                token_price: self.data().default_sell_balance,
                token_id: self.data().default_token_id.clone(),
                owner: self.data().owner_id.clone(),
                protected_ts: 0,
            }
        );

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

    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::{testing_env, MockedBlockchain};
    use near_sdk::json_types::{U128};

    use super::*;

    const ONE_NEAR: u128 = 1_000_000_000_000_000_000_000_000;

    fn setup_contract() -> (VMContextBuilder, Contract) {
        let mut context = VMContextBuilder::new();
        testing_env!(context.predecessor_account_id(accounts(0)).build());
        let contract = Contract::new(
            accounts(0), 
            accounts(1), 
            accounts(2), 
            U128(ONE_NEAR),
            15*60, 500, 15);
        (context, contract)
    }

    fn edit_frame(
        context: &mut VMContextBuilder,
        contract: &mut Contract,
        frame_id: FrameId,
        frame_data: String,
    ) {
        testing_env!(context
            .predecessor_account_id(accounts(0))
            .build());
        contract.edit_frame(frame_id, frame_data);
    }

    #[test]
    fn test_basics() {

        let (mut context, mut contract) = setup_contract();

        let frame_id: FrameId = 33;

        let frame_data = contract.get_frame_data(frame_id).expect("NO_DATA");
        assert_eq!(frame_data, DEFAULT_DATA, "ERR_DATA");

        edit_frame(&mut context, &mut contract, frame_id, "TEST_STRINGS".to_string());

        let frame_data = contract.get_frame_data(frame_id).expect("NO_DATA");
        assert_eq!(frame_data, "TEST_STRINGS".to_string(), "ERR_DATA");

    }

    #[test]
    fn test_owner_actions() {
        let (_, mut contract) = setup_contract();
        
        let metadata = contract.get_metadata();
        assert_eq!(metadata.version, "0.1.0".to_string());
        assert_eq!(metadata.owner_id, "alice".to_string());
        assert_eq!(metadata.amm_id, "bob".to_string());
        assert_eq!(metadata.default_token_id, "charlie".to_string());
        assert_eq!(metadata.default_sell_balance, U128(ONE_NEAR));
        assert_eq!(metadata.protected_period, 15*60);
        assert_eq!(metadata.frame_count, 500);
        assert_eq!(metadata.trading_fee, 15);

        let frame_metadata = contract.get_frame_metadata(100);
        assert_eq!(frame_metadata.unwrap().token_id, "charlie".to_string());
        let frame_metadata = contract.get_frame_metadata(500);
        assert!(frame_metadata.is_none());
        contract.expand_frames(10);
        let frame_metadata = contract.get_frame_metadata(500);
        assert_eq!(frame_metadata.unwrap().token_id, "charlie".to_string());
        let frame_metadata = contract.get_frame_metadata(510);
        assert!(frame_metadata.is_none());

        contract.set_trading_fee(25);
        let metadata = contract.get_metadata();
        assert_eq!(metadata.version, "0.1.0".to_string());
        assert_eq!(metadata.owner_id, "alice".to_string());
        assert_eq!(metadata.amm_id, "bob".to_string());
        assert_eq!(metadata.default_token_id, "charlie".to_string());
        assert_eq!(metadata.default_sell_balance, U128(ONE_NEAR));
        assert_eq!(metadata.protected_period, 15*60);
        assert_eq!(metadata.frame_count, 510);
        assert_eq!(metadata.trading_fee, 25);

        contract.set_protected_period(20*60);
        let metadata = contract.get_metadata();
        assert_eq!(metadata.version, "0.1.0".to_string());
        assert_eq!(metadata.owner_id, "alice".to_string());
        assert_eq!(metadata.amm_id, "bob".to_string());
        assert_eq!(metadata.default_token_id, "charlie".to_string());
        assert_eq!(metadata.default_sell_balance, U128(ONE_NEAR));
        assert_eq!(metadata.protected_period, 20*60);
        assert_eq!(metadata.frame_count, 510);
        assert_eq!(metadata.trading_fee, 25);

        contract.set_default_token(accounts(3), U128(ONE_NEAR+1));
        let metadata = contract.get_metadata();
        assert_eq!(metadata.version, "0.1.0".to_string());
        assert_eq!(metadata.owner_id, "alice".to_string());
        assert_eq!(metadata.amm_id, "bob".to_string());
        assert_eq!(metadata.default_token_id, "danny".to_string());
        assert_eq!(metadata.default_sell_balance, U128(ONE_NEAR+1));
        assert_eq!(metadata.protected_period, 20*60);
        assert_eq!(metadata.frame_count, 510);
        assert_eq!(metadata.trading_fee, 25);

        contract.set_amm(accounts(4));
        let metadata = contract.get_metadata();
        assert_eq!(metadata.version, "0.1.0".to_string());
        assert_eq!(metadata.owner_id, "alice".to_string());
        assert_eq!(metadata.amm_id, "eugene".to_string());
        assert_eq!(metadata.default_token_id, "danny".to_string());
        assert_eq!(metadata.default_sell_balance, U128(ONE_NEAR+1));
        assert_eq!(metadata.protected_period, 20*60);
        assert_eq!(metadata.frame_count, 510);
        assert_eq!(metadata.trading_fee, 25);

        contract.add_token_to_whitelist(accounts(1));
        let tokens = contract.get_whitelist();
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0], "bob".to_string());

        contract.add_token_to_whitelist(accounts(2));
        let tokens = contract.get_whitelist();
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[1], "charlie".to_string());

        contract.remove_token_from_whitelist(accounts(1));
        let tokens = contract.get_whitelist();
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0], "charlie".to_string());

        contract.remove_token_from_whitelist(accounts(2));
        let tokens = contract.get_whitelist();
        assert_eq!(tokens.len(), 0);

        contract.set_owner(accounts(5));
        let metadata = contract.get_metadata();
        assert_eq!(metadata.version, "0.1.0".to_string());
        assert_eq!(metadata.owner_id, "fargo".to_string());
        assert_eq!(metadata.amm_id, "eugene".to_string());
        assert_eq!(metadata.default_token_id, "danny".to_string());
        assert_eq!(metadata.default_sell_balance, U128(ONE_NEAR+1));
        assert_eq!(metadata.protected_period, 20*60);
        assert_eq!(metadata.frame_count, 510);
        assert_eq!(metadata.trading_fee, 25);

        
    }

}