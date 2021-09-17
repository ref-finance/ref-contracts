use near_sdk::{env, AccountId};

/// Maintain information about fees.
pub struct SwapFees {
    /// Basis points of the fee for exchange.
    pub exchange_fee: u32,
    /// Basis points of the fee for referrer.
    pub referral_fee: u32,
    pub exchange_id: AccountId,
    pub referral_id: Option<AccountId>,
}

impl SwapFees {
    pub fn new(exchange_fee: u32) -> Self {
        SwapFees {
            exchange_fee,
            exchange_id: env::current_account_id(),
            referral_fee: 0,
            referral_id: None,
        }
    }

    pub fn zero() -> Self {
        Self::new(0)
    }
}
