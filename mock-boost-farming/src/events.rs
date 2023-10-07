use near_sdk::{
    AccountId, log,
    serde::{Serialize},
    serde_json::{json},
    json_types::U128,
};

const EVENT_STANDARD: &str = "ref-farming";
const EVENT_STANDARD_VERSION: &str = "1.0.0";

#[derive(Serialize, Debug, Clone)]
#[serde(crate = "near_sdk::serde")]
#[serde(tag = "event", content = "data")]
#[serde(rename_all = "snake_case")]
pub enum Event<'a> {
    SeedCreate {
        caller_id: &'a AccountId,
        seed_id: &'a String,
        min_deposit: &'a U128,
        slash_rate: u32,
        min_locking_duration: u32,
    },
    FarmCreate {
        caller_id: &'a AccountId,
        reward_token: &'a AccountId,
        farm_id: &'a String,
        daily_reward: &'a U128,
        start_at: u32,
    },
    FarmCancel {
        caller_id: &'a AccountId,
        farm_id: &'a String,
    },
    RewardDeposit {
        caller_id: &'a AccountId,
        farm_id: &'a String,
        deposit_amount: &'a U128,
        total_amount: &'a U128,
        start_at: u32,
    },
    SeedDeposit {
        farmer_id: &'a AccountId,
        seed_id: &'a String,
        deposit_amount: &'a U128,
        increased_power: &'a U128,
        duration: u32,
    },
    SeedFreeToLock {
        farmer_id: &'a AccountId,
        seed_id: &'a String,
        amount: &'a U128,
        increased_power: &'a U128,
        duration: u32,
    },
    SeedUnlock {
        farmer_id: &'a AccountId,
        seed_id: &'a String,
        unlock_amount: &'a U128,
        decreased_power: &'a U128,
        slashed_seed: &'a U128,
    },
    SeedWithdraw {
        farmer_id: &'a AccountId,
        seed_id: &'a String,
        withdraw_amount: &'a U128,
        success: bool,
    },
    SeedWithdrawLostfound {
        farmer_id: &'a AccountId,
        seed_id: &'a String,
        withdraw_amount: &'a U128,
        success: bool,
    },
    SeedWithdrawSlashed {
        owner_id: &'a AccountId,
        seed_id: &'a String,
        withdraw_amount: &'a U128,
        success: bool,
    },
    RewardWithdraw {
        farmer_id: &'a AccountId,
        token_id: &'a AccountId,
        withdraw_amount: &'a U128,
        success: bool,
    },
    RewardLostfound {
        farmer_id: &'a AccountId,
        token_id: &'a AccountId,
        withdraw_amount: &'a U128,
    },
}

impl Event<'_> {
    pub fn emit(&self) {
        emit_event(&self);
    }
}

// Emit event that follows NEP-297 standard: https://nomicon.io/Standards/EventsFormat
// Arguments
// * `standard`: name of standard, e.g. nep171
// * `version`: e.g. 1.0.0
// * `event`: type of the event, e.g. nft_mint
// * `data`: associate event data. Strictly typed for each set {standard, version, event} inside corresponding NEP
pub (crate) fn emit_event<T: ?Sized + Serialize>(data: &T) {
    let result = json!(data);
    let event_json = json!({
        "standard": EVENT_STANDARD,
        "version": EVENT_STANDARD_VERSION,
        "event": result["event"],
        "data": [result["data"]]
    })
    .to_string();
    log!("{}", format!("EVENT_JSON:{}", event_json));
}
