
use crate::*;
use near_sdk::serde_json::json;

const EVENT_STANDARD: &str = "exchange.ref";
const EVENT_STANDARD_VERSION: &str = "1.0.0";

#[derive(Serialize, Debug, Clone)]
#[serde(crate = "near_sdk::serde")]
#[serde(tag = "event", content = "data")]
#[serde(rename_all = "snake_case")]
#[must_use = "Don't forget to `.emit()` this event"]
pub enum Event<'a> {
    DonationShare {
        account_id: &'a AccountId,
        pool_id: u64,
        amount: U128,
    },
    DonationToken {
        account_id: &'a AccountId,
        token_id: &'a AccountId,
        amount: U128,
    }
}

impl Event<'_> {
    pub fn emit(&self) {
        let data = json!(self);
        let event_json = json!({
            "standard": EVENT_STANDARD,
            "version": EVENT_STANDARD_VERSION,
            "event": data["event"],
            "data": [data["data"]]
        })
        .to_string();
        log!("EVENT_JSON:{}", event_json);
    }
}