// Storage errors //
pub const ERR10_ACC_NOT_REGISTERED: &str = "E10: account not registered";
pub const ERR11_INSUFFICIENT_STORAGE: &str = "E11: insufficient $NEAR storage deposit";
pub const ERR12_STORAGE_UNREGISTER_REWARDS_NOT_EMPTY: &str = "E12: still has rewards when unregister";
pub const ERR13_STORAGE_UNREGISTER_SEED_POWER_NOT_EMPTY: &str = "E13: still has seed power when unregister";
pub const ERR14_NO_STORAGE_CAN_WITHDRAW: &str = "E14: no storage can withdraw";

// Reward errors //
pub const ERR21_TOKEN_NOT_REG: &str = "E21: token not registered";
pub const ERR22_NOT_ENOUGH_TOKENS: &str = "E22: not enough tokens in deposit";

pub const ERR25_CALLBACK_POST_WITHDRAW_INVALID: &str = "E25: expected 1 promise result from withdraw";

// Seed errors //
pub const ERR31_SEED_NOT_EXIST: &str = "E31: seed not exist";
pub const ERR32_NOT_ENOUGH_SEED: &str = "E32: not enough amount of seed";
pub const ERR33_INVALID_SEED_ID: &str = "E33: invalid seed id";
pub const ERR34_BELOW_MIN_SEED_DEPOSITED: &str = "E34: below min_deposit of this seed";
pub const ERR35_ILLEGAL_TOKEN_ID: &str = "E35: illegal token_id in mft_transfer_call";
pub const ERR36_FARMS_NUM_HAS_REACHED_LIMIT: &str = "E36: the number of farms has reached its limit";

// farm errors //
pub const ERR41_FARM_NOT_EXIST: &str = "E41: farm not exist";
pub const ERR42_INVALID_FARM_ID: &str = "E42: invalid farm id";
pub const ERR43_INVALID_FARM_STATUS: &str = "E43: invalid farm status";
pub const ERR44_INVALID_FARM_REWARD: &str = "E44: invalid reward token for this farm";

// transfer errors //
pub const ERR51_WRONG_MSG_FORMAT: &str = "E51: Illegal msg in (m)ft_transfer_call";
pub const ERR52_MSG_NOT_SUPPORT: &str = "E52: Illegal msg in mft_transfer_call";

// CD account errors //
pub const ERR61_CDACCOUNT_NUM_HAS_REACHED_LIMIT: &str = "E61: the number of CDAccounts has reached its limit";
pub const ERR62_INVALID_CD_STRATEGY_INDEX: &str = "E62: invalid CDStrategy index";
pub const ERR63_INVALID_CD_ACCOUNT_INDEX: &str = "E63: invalid CDAccount index";
pub const ERR64_EXPIRED_CD_ACCOUNT: &str = "E64: expired CDAccount";
pub const ERR65_NON_EMPTY_CD_ACCOUNT: &str = "E65: Non-empty CDAccount";
pub const ERR66_EMPTY_CD_ACCOUNT: &str = "E66: Empty CDAccount";
pub const ERR67_UNMATCHED_SEED_ID: &str = "E67: Unmatched SeedId";
pub const ERR68_INVALID_CD_STRATEGY: &str = "E68: Invalid CD Strategy";

pub const ERR500: &str = "E500: Internal ERROR!";