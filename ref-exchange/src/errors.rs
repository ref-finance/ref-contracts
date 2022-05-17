// Storage errors.

pub const ERR10_ACC_NOT_REGISTERED: &str = "E10: account not registered";
pub const ERR11_INSUFFICIENT_STORAGE: &str = "E11: insufficient $NEAR storage deposit";
pub const ERR12_TOKEN_NOT_WHITELISTED: &str = "E12: token not whitelisted";
pub const ERR13_LP_NOT_REGISTERED: &str = "E13: LP not registered";
pub const ERR14_LP_ALREADY_REGISTERED: &str = "E14: LP already registered";
pub const ERR15_NO_STORAGE_CAN_WITHDRAW: &str = "E15: no storage can withdraw";
pub const ERR16_STORAGE_WITHDRAW_TOO_MUCH: &str = "E16: storage withdraw too much";
pub const ERR17_DEPOSIT_LESS_THAN_MIN_STORAGE: &str = "E17: deposit less than min storage";
pub const ERR18_TOKENS_NOT_EMPTY: &str = "E18: storage unregister tokens not empty";

// Accounts.

pub const ERR21_TOKEN_NOT_REG: &str = "E21: token not registered";
pub const ERR22_NOT_ENOUGH_TOKENS: &str = "E22: not enough tokens in deposit";
// pub const ERR23_NOT_ENOUGH_NEAR: &str = "E23: not enough NEAR in deposit";
pub const ERR24_NON_ZERO_TOKEN_BALANCE: &str = "E24: non-zero token balance";
pub const ERR25_CALLBACK_POST_WITHDRAW_INVALID: &str =
    "E25: expected 1 promise result from withdraw";
// [AUDIT_05]
// pub const ERR26_ACCESS_KEY_NOT_ALLOWED: &str = "E26: access key not allowed";
pub const ERR27_DEPOSIT_NEEDED: &str = 
    "E27: attach 1yN to swap tokens not in whitelist";
pub const ERR28_WRONG_MSG_FORMAT: &str = "E28: Illegal msg in ft_transfer_call";
pub const ERR29_ILLEGAL_WITHDRAW_AMOUNT: &str = "E29: Illegal withdraw amount";

// Liquidity operations.

pub const ERR31_ZERO_AMOUNT: &str = "E31: adding zero amount";
pub const ERR32_ZERO_SHARES: &str = "E32: minting zero shares";
// [AUDIT_07]
pub const ERR33_TRANSFER_TO_SELF: &str = "E33: transfer to self";
pub const ERR34_INSUFFICIENT_LP_SHARES: &str = "E34: insufficient lp shares";
pub const ERR35_AT_LEAST_ONE_YOCTO: &str = "E35: requires attached deposit of at least 1 yoctoNEAR";

// Action result.

pub const ERR41_WRONG_ACTION_RESULT: &str = "E41: wrong action result type";

// Contract Level
pub const ERR51_CONTRACT_PAUSED: &str = "E51: contract paused";

// Swap
pub const ERR60_DECIMAL_ILLEGAL: &str = "E60: illegal decimal";
pub const ERR61_AMP_ILLEGAL: &str = "E61: illegal amp";
pub const ERR62_FEE_ILLEGAL: &str = "E62: illegal fee";
pub const ERR63_MISSING_TOKEN: &str = "E63: missing token";
pub const ERR64_TOKENS_COUNT_ILLEGAL: &str = "E64: illegal tokens count";
pub const ERR65_INIT_TOKEN_BALANCE: &str = "E65: init token balance should be non-zero";
pub const ERR66_INVARIANT_CALC_ERR: &str = "E66: encounter err when calc invariant D";
pub const ERR67_LPSHARE_CALC_ERR: &str = "E67: encounter err when calc lp shares";
pub const ERR68_SLIPPAGE: &str = "E68: slippage error";
pub const ERR69_MIN_RESERVE: &str = "E69: pool reserved token balance less than MIN_RESERVE";
pub const ERR70_SWAP_OUT_CALC_ERR: &str = "E70: encounter err when calc swap out";
pub const ERR71_SWAP_DUP_TOKENS: &str = "E71: illegal swap with duplicated tokens";
pub const ERR72_AT_LEAST_ONE_SWAP: &str = "E72: at least one swap";
pub const ERR73_SAME_TOKEN: &str = "E73: same token swap";
pub const ERR75_INVARIANT_REDUCE: &str = "E75: invariant can not reduce ";
pub const ERR76_INVALID_PARAMS: &str = "E76: invalid params";

// pool manage
pub const ERR81_AMP_IN_LOCK: &str = "E81: amp is currently in lock";
pub const ERR82_INSUFFICIENT_RAMP_TIME: &str = "E82: insufficient ramp time";
pub const ERR83_INVALID_AMP_FACTOR: &str = "E83: invalid amp factor";
pub const ERR84_AMP_LARGE_CHANGE: &str = "E84: amp factor change is too large";
pub const ERR85_NO_POOL: &str = "E85: invalid pool id";
pub const ERR86_MIN_AMOUNT: &str = "E86: amount need above min amount";
pub const ERR87_ILLEGAL_POOL_ID: &str = "E87: illegal pool id";
pub const ERR88_NOT_STABLE_POOL: &str = "E88: not stable pool";
pub const ERR89_WRONG_TOKEN_COUNT: &str = "E89: wrong token count";
pub const ERR90_FEE_TOO_LARGE: &str = "E90: fee too large";
pub const ERR91_NOT_ENOUGH_SHARES: &str = "E91: not enough shares";
pub const ERR92_TOKEN_DUPLICATES: &str = "E92: token duplicated";
pub const ERR89_WRONG_AMOUNT_COUNT: &str = "E89: wrong amount count";


// owner
pub const ERR100_NOT_ALLOWED: &str = "E100: no permission to invoke this";
pub const ERR101_ILLEGAL_FEE: &str = "E101: illegal fee";
pub const ERR102_INVALID_TOKEN_ID: &str = "E102: invalid token id";
pub const ERR103_NOT_INITIALIZED: &str = "E103: contract is not initialized";


//mft
pub const ERR110_INVALID_REGISTER: &str = "E110: Invalid register";

// rated pool
pub const ERR120_RATES_EXPIRED: &str = "E120: Rates expired";
pub const ERR121_SWAPPED_AMOUNT_EQUALS_0: &str = "E121: Swapped amount equals 0";
// pub const ERR122_FAILED_TO_UPDATE_RATES: &str = "E122: Failed to update rates";
pub const ERR123_ONE_PROMISE_RESULT: &str = "E123: Cross-contract call should have exactly one promise result";
pub const ERR124_CROSS_CALL_FAILED: &str = "E124: Cross-contract call failed";
// pub const ERR125_FAILED_TO_APPLY_RATES: &str = "E125: Failed to apply new rates";
pub const ERR126_FAILED_TO_PARSE_RESULT: &str = "E126: Failed to parse cross-contract call result";
pub const ERR127_INVALID_RATE_TYPE: &str = "E127: Invalid rate type";
