// Storage errors //

pub const ERR10_ACC_NOT_REGISTERED: &str = "E10: account not registered";
pub const ERR11_INSUFFICIENT_STORAGE: &str = "E11: insufficient $NEAR storage deposit";
pub const ERR12_TOKEN_NOT_WHITELISTED: &str = "E12: token not whitelisted";
pub const ERR13_LP_NOT_REGISTERED: &str = "E13: LP not registered";
pub const ERR14_LP_ALREADY_REGISTERED: &str = "E14: LP already registered";

// Accounts //

pub const ERR21_TOKEN_NOT_REG: &str = "E21: token not registered";
pub const ERR22_NOT_ENOUGH_TOKENS: &str = "E22: not enough tokens in deposit";
// pub const ERR23_NOT_ENOUGH_NEAR: &str = "E23: not enough NEAR in deposit";
pub const ERR24_NON_ZERO_TOKEN_BALANCE: &str = "E24: non-zero token balance";
pub const ERR25_CALLBACK_POST_WITHDRAW_INVALID: &str =
    "E25: expected 1 promise result from withdraw";
pub const ERR26_ACCESS_KEY_NOT_ALLOWED: &str = "E26: access key not allowed";

// Liquidity operations //

pub const ERR31_ZERO_AMOUNT: &str = "E31: adding zero amount";
pub const ERR32_ZERO_SHARES: &str = "E32: minting zero shares";
