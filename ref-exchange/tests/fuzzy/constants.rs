#![allow(unused)] 
pub const TOKENS: [&str; 10] = ["ref", "dai", "usdt", "usdc", "weth", "wnear", "1inch", "grt", "oct", "uni"];

pub const EVERY_PREFERENCE_NUM: i32 = 1;
pub const INIT_ACCOUNT_FOR_TOKEN: u64 = 200;

pub const INIT_TOKEN_TO_SWAP_POOL_LIMIT: u64 = 100;
pub const ADD_LIQUIDITY_LIMIT: u64 = 20;
pub const REMOVE_LIQUIDITY_LIMIT: u64 = 20;
pub const FEE_LIMIT: i32 = 30;

pub const FUZZY_NUM: usize = 2;
pub const OPERATION_NUM: i32 = 10;
pub const AMOUNT_IN_LIMIT: u128 = 10;
pub const TRANSFER_AMOUNT_LIMIT: u128 = 20;

pub const LP_LIMIT: u128 = 10;
pub const STABLE_TOKENS: [&str; 3] = ["dai001", "usdt", "usdc"];
pub const DECIMALS: [u8; 3] = [18, 6, 6];
pub const TARGET_DECIMAL: u8 = 18;

pub const ONE_LPT: u128 = 1000000000000000000;
pub const ONE_DAI: u128 = 1000000000000000000;
pub const ONE_USDT: u128 = 1000000;
pub const ONE_USDC: u128 = 1000000;