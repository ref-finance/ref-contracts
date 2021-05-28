use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::json_types::{U64, U128};
use crate::*;

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct HumanReadablePaymentItem {
    pub amount: U128,
    pub token_id: AccountId,
    pub receiver_id: AccountId,
}

impl From<&PaymentItem> for HumanReadablePaymentItem {
    fn from(item: &PaymentItem) -> Self {
        Self {
            amount: item.amount.into(),
            token_id: item.token_id.clone(),
            receiver_id: item.receiver_id.clone(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct HumanReadableFrameMetadata {
    pub token_price: U128,
    pub token_id: AccountId,
    pub owner: AccountId,
    pub protected_ts: U64,
}

impl From<&FrameMetadata> for HumanReadableFrameMetadata {
    fn from(metadata: &FrameMetadata) -> Self {
        Self {
            token_price: metadata.token_price.into(),
            token_id: metadata.token_id.clone(),
            owner: metadata.owner.clone(),
            protected_ts: metadata.protected_ts.into(),
        }
    }
}

#[near_bindgen]
impl Contract {
    pub fn get_whitelist(&self) -> Vec<String> {
        self.data().whitelist.to_vec()
    }

    pub fn get_frame_metadata(&self, index: FrameId) -> Option<HumanReadableFrameMetadata> {
        if index >= self.data().frame_count {
            None
        } else {
            let metadata = self.data().frames.get(&index).unwrap_or_default();
            Some((&metadata).into())
        }
    }

    pub fn list_frame_metadata(&self, from_index: u64, limit: u64) ->Vec<HumanReadableFrameMetadata> {
        (from_index..std::cmp::min(from_index + limit, self.data().frame_count as u64))
            .map(
                |index| {
                    let metadata = self.data().frames.get(&(index as u16)).unwrap_or_default();
                    (&metadata).into()
                }
            ).collect()
    }

    pub fn get_frame_data(&self, index: FrameId) -> Option<String> {
        if index >= self.data().frame_count {
            None
        } else {
            Some(
                self.data()
                .frames_data.get(&index)
                .unwrap_or(DEFAULT_DATA.to_string())
            )
        }
    }

    pub fn list_frame_data(&self, from_index: u64, limit: u64) ->Vec<String> {
        (from_index..std::cmp::min(from_index + limit, self.data().frame_count as u64))
            .map(|index| 
                self.data().frames_data.get(&(index as u16))
                .unwrap_or(DEFAULT_DATA.to_string())
            ).collect()
    }

    pub fn list_failed_payments(&self, from_index: u64, limit: u64) ->Vec<HumanReadablePaymentItem> {
        (from_index..std::cmp::min(from_index + limit, self.data().failed_payments.len()))
            .map(|index| {
                    let item = self.data().failed_payments.get(index).unwrap();
                    (&item).into()
                }
            ).collect()
    }

}