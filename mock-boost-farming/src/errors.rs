// unify all error content for whole project
pub const E000_ALREADY_INIT: &str = "E000: already initialized";
pub const E001_PROMISE_RESULT_COUNT_INVALID: &str = "E001: promise result count invalid";
pub const E002_NOT_ALLOWED: &str = "E002: not allowed for the caller";
pub const E003_NOT_INIT: &str = "E003: not initialized";
pub const E004_CONTRACT_PAUSED: &str = "E004: contract paused";
pub const E005_NOT_ALLOWED_ON_CUR_STATE: &str = "E005: not allowed on current state";
pub const E006_NOT_IMPLEMENTED: &str = "E006: not implemented";
pub const E007_INVALID_OPERATOR: &str = "E007: invalid operator";

pub const E100_ACC_NOT_REGISTERED: &str = "E100: account not registered";
pub const E101_INSUFFICIENT_BALANCE: &str = "E101: insufficient balance";
pub const E102_INSUFFICIENT_STORAGE: &str = "E102: insufficient storage";
pub const E103_STILL_HAS_REWARD: &str = "E103: still has reward";
pub const E104_STILL_HAS_SEED: &str = "E104: still has seed";

pub const E200_INVALID_RATIO: &str = "E200: invalid ratio";
pub const E201_INVALID_DURATION: &str = "E201: invalid duration";
pub const E202_FORBID_SELF_BOOST: &str = "E202: self boost is forbidden";
pub const E203_EXCEED_FARM_NUM_IN_BOOST: &str = "E203: exceed max farm num in one boost";
pub const E204_EXCEED_SEED_NUM_IN_BOOSTER: &str = "E204: exceed max seed num in one booster";
pub const E205_INVALID_SLASH_RATE: &str = "E205: invalid slash rate";

pub const E300_FORBID_LOCKING: &str = "E300: locking on this seed is forbidden";
pub const E301_SEED_NOT_EXIST: &str = "E301: seed not exist";
pub const E302_SEED_ALREADY_EXIST: &str = "E302: seed already exist";
pub const E303_EXCEED_FARM_NUM_IN_SEED: &str = "E303: exceed max farm num in one seed";
pub const E304_CAUSE_PRE_UNLOCK: &str = "E304: would cause pre unlock";
pub const E305_STILL_IN_LOCK: &str = "E305: still in locking";
pub const E306_LOCK_AMOUNT_TOO_SMALL: &str = "E306: locking amount is too small";
pub const E307_BELOW_MIN_DEPOSIT: &str = "E307: below minimum deposit amount";
pub const E308_INVALID_SEED_ID: &str = "E308: invalid seed id";
pub const E309_NO_NEED_FORCE: &str = "E309: can directly unlock without force";

pub const E401_FARM_NOT_EXIST: &str = "E401: farm not exist";
pub const E403_FARM_ALREADY_DEPOSIT_REWARD: &str = "E403: farm can not be cancelled due to already deposit reward";
pub const E404_UNMATCHED_REWARD_TOKEN: &str = "E404: reward token does NOT match";
pub const E405_FARM_NOT_ENDED: &str = "E405: farm not ended";
pub const E406_INVALID_FARM_ID: &str = "E406: invalid farm id";
// pub const E402_FARM_ALREADY_EXIST: &str = "E402: farm already exist";

pub const E500_INVALID_MSG: &str = "E500: invalid msg";

pub const E600_MFT_INVALID_TOKEN_ID: &str = "E600: MFT token_id is invalid";
pub const E601_MFT_CAN_NOT_BE_REWARD: &str = "E601: MFT can NOT be reward token";