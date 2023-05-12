use crate::*;
use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;
use near_sdk::{serde_json, PromiseOrValue, json_types::ValidAccountId};

/// Message parameters to receive via token function call.
#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
enum TokenReceiverMessage {
    Free,
    Lock { duration_sec: u32 },
    Reward { farm_id: FarmId },
}

#[near_bindgen]
impl FungibleTokenReceiver for Contract {
    fn ft_on_transfer(
        &mut self,
        sender_id: ValidAccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128> {
        assert!(self.data().state == RunningState::Running, "{}", E004_CONTRACT_PAUSED);

        let amount: u128 = amount.into();
        let token_id = env::predecessor_account_id();
        let message =
            serde_json::from_str::<TokenReceiverMessage>(&msg).expect(E500_INVALID_MSG);
        match message {
            TokenReceiverMessage::Free => {
                self.stake_free_seed(sender_id.as_ref(), &token_id.into(), amount);
            }
            TokenReceiverMessage::Lock { duration_sec } => {
                self.stake_lock_seed(sender_id.as_ref(), &token_id.into(), amount, duration_sec);
            }
            TokenReceiverMessage::Reward { farm_id } => {
                let (total_amount, start_at) =
                    self.internal_deposit_reward(&farm_id, &token_id, amount);

                Event::RewardDeposit {
                    caller_id: sender_id.as_ref(),
                    farm_id: &farm_id,
                    deposit_amount: &U128(amount),
                    total_amount: &U128(total_amount),
                    start_at,
                }
                .emit();
            }
        }
        PromiseOrValue::Value(U128(0))
    }
}

pub trait MFTTokenReceiver {
    fn mft_on_transfer(
        &mut self,
        token_id: String,
        sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128>;
}


#[near_bindgen]
impl MFTTokenReceiver for Contract {
    /// Callback on receiving tokens by this contract.
    fn mft_on_transfer(
        &mut self,
        token_id: String,
        sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128> {
        assert!(self.data().state == RunningState::Running, "{}", E004_CONTRACT_PAUSED);
        
        let amount: u128 = amount.into();
        assert!(token_id.starts_with(MFT_TAG), "{}", E600_MFT_INVALID_TOKEN_ID);
        let sub_token_id = &token_id[1..token_id.len()];
        let seed_id = format!("{}{}{}", env::predecessor_account_id(), SEED_TAG, sub_token_id);
        
        let message =
            serde_json::from_str::<TokenReceiverMessage>(&msg).expect(E500_INVALID_MSG);
        match message {
            TokenReceiverMessage::Free => {
                self.stake_free_seed(&sender_id, &seed_id, amount);
            }
            TokenReceiverMessage::Lock { duration_sec } => {
                self.stake_lock_seed(&sender_id, &seed_id, amount, duration_sec);
            }
            TokenReceiverMessage::Reward { farm_id: _ } => {
                panic!("{}", E601_MFT_CAN_NOT_BE_REWARD)
            }
        }
        PromiseOrValue::Value(U128(0))
    }
}


impl Contract {
    pub fn stake_free_seed(&mut self, farmer_id: &AccountId, seed_id: &SeedId, amount: u128) {
        let mut farmer = self.internal_unwrap_farmer(&farmer_id);
        let mut seed = self.internal_unwrap_seed(&seed_id);
        assert!(amount >= seed.min_deposit, "{}", E307_BELOW_MIN_DEPOSIT);

        self.internal_do_farmer_claim(&mut farmer, &mut seed);

        let mut farmer_seed = farmer.seeds.get(&seed_id).unwrap();
        let increased_seed_power = farmer_seed.add_free(amount);
        farmer.seeds.insert(&seed_id, &farmer_seed);

        seed.total_seed_amount += amount;
        seed.total_seed_power += increased_seed_power;

        self.update_impacted_seeds(&mut farmer, &seed_id);

        self.internal_set_farmer(&farmer_id, farmer);
        self.internal_set_seed(&seed_id, seed);

        Event::SeedDeposit {
            farmer_id,
            seed_id,
            deposit_amount: &U128(amount),
            increased_power: &U128(increased_seed_power),
            duration: 0,
        }
        .emit();
    }

    pub fn stake_lock_seed(
        &mut self,
        farmer_id: &AccountId,
        seed_id: &SeedId,
        amount: u128,
        duration_sec: u32,
    ) {
        let mut farmer = self.internal_unwrap_farmer(&farmer_id);
        let mut seed = self.internal_unwrap_seed(&seed_id);
        assert!(amount >= seed.min_deposit, "{}", E307_BELOW_MIN_DEPOSIT);

        assert!(seed.min_locking_duration_sec > 0, "{}", E300_FORBID_LOCKING);
        assert!(duration_sec >= seed.min_locking_duration_sec, "{}", E201_INVALID_DURATION);
        let config = self.internal_config();
        assert!(duration_sec <= config.maximum_locking_duration_sec, "{}", E201_INVALID_DURATION);

        self.internal_do_farmer_claim(&mut farmer, &mut seed);

        let mut farmer_seed = farmer.seeds.get(&seed_id).unwrap();
        let increased_seed_power = farmer_seed.add_lock(amount, duration_sec, &config);
        farmer.seeds.insert(&seed_id, &farmer_seed);

        seed.total_seed_amount += amount;
        seed.total_seed_power += increased_seed_power;

        self.update_impacted_seeds(&mut farmer, &seed_id);

        self.internal_set_farmer(&farmer_id, farmer);
        self.internal_set_seed(&seed_id, seed);

        Event::SeedDeposit {
            farmer_id,
            seed_id,
            deposit_amount: &U128(amount),
            increased_power: &U128(increased_seed_power),
            duration: duration_sec,
        }
        .emit();
    }
}
