use near_sdk::AccountId;

/// Maintain information about fees.
pub struct SwapFees {
    /// Basis points of the fee for exchange.
    pub exchange_fee: u32,
    /// Basis points of the fee for referrer.
    pub referral_fee: u32,
    pub exchange_id: AccountId,
    pub referral_id: Option<AccountId>,
}
