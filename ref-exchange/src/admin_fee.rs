use near_sdk::{env, AccountId};
use uint::construct_uint;
use crate::utils::{FEE_DIVISOR, u128_ratio};

construct_uint! {
    /// 256-bit unsigned integer.
    pub struct U256(4);
}

/// Maintain information about fees.
pub struct AdminFees {
    /// Basis points of the admin fee in total fee.
    pub admin_fee_bps: u32,
    pub exchange_id: AccountId,
    /// referral_id, referral_fee_bps,
    /// where referral_fee_bps is basis points of the referral fee in admin fee, 
    /// and remaining admin fee belongs to exchange (protocol).
    pub referral_info: Option<(AccountId, u32)>,
}

impl AdminFees {
    pub fn new(admin_fee_bps: u32) -> Self {
        AdminFees {
            admin_fee_bps,
            exchange_id: env::current_account_id(),
            referral_info: None,
        }
    }

    pub fn zero() -> Self {
        Self::new(0)
    }

    /// return (referral_share, referral_id)
    pub fn calc_referral_share(&self, admin_share: u128) -> (u128, AccountId) {
        if let Some((referral, referral_fee)) = &self.referral_info {
            let referral_share = u128_ratio(admin_share, *referral_fee as u128, FEE_DIVISOR as u128);
            (referral_share, referral.clone())
        } else {
            (0, self.exchange_id.clone())
        }
    }
}
