use crate::*;

#[derive(Clone, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct SwapVolume {
    pub input: U128,
    pub output: U128,
}

#[derive(BorshSerialize, BorshDeserialize)]
pub enum Pool {
    SimplePool(SimplePool),
}

impl Pool {
    pub fn parse(&mut self, state: &mut State) {
        match self {
            Pool::SimplePool(simple_pool) => {
                simple_pool.parse(state);
            }
        };
    }
}

/// Account deposits information and storage cost.
#[derive(BorshSerialize, BorshDeserialize, Default, Clone)]
pub struct Account {
    /// Native NEAR amount sent to the exchange.
    /// Used for storage right now, but in future can be used for trading as well.
    pub near_amount: Balance,
    /// Amounts of various tokens deposited to this account.
    pub tokens: HashMap<AccountId, Balance>,
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct SimplePool {
    /// List of tokens in the pool.
    pub token_account_ids: Vec<AccountId>,
    /// How much NEAR this contract has.
    pub amounts: Vec<Balance>,
    /// Volumes accumulated by this pool.
    pub volumes: Vec<SwapVolume>,
    /// Fee charged for swap (gets divided by FEE_DIVISOR).
    pub total_fee: u32,
    /// Portion of the fee going to exchange.
    pub exchange_fee: u32,
    /// Portion of the fee going to referral.
    pub referral_fee: u32,
    /// Shares of the pool by liquidity providers.
    pub shares: LookupMap<AccountId, Balance>,
    /// Total number of shares.
    pub shares_total_supply: Balance,
}

impl SimplePool {
    pub fn parse(&mut self, state: &mut State) {
        self.shares.parse(state);
    }
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct RefExchangeContract {
    /// Account of the owner.
    pub owner_id: AccountId,
    /// Exchange fee, that goes to exchange itself (managed by governance).
    pub exchange_fee: u32,
    /// Referral fee, that goes to referrer in the call.
    pub referral_fee: u32,
    /// List of all the pools.
    pub pools: Vector<Pool>,
    /// Accounts registered, keeping track all the amounts deposited, storage and more.
    pub accounts: LookupMap<AccountId, Account>,
    /// Set of whitelisted tokens by "owner".
    pub whitelisted_tokens: UnorderedSet<AccountId>,
}

impl RefExchangeContract {
    pub fn parse(&mut self, state: &mut State) {
        // parsing pools
        self.pools.parse(state);
        for pool in self.pools.data.iter_mut() {
            pool.parse(state);
        }
        self.accounts.parse(state);

        self.whitelisted_tokens.parse(state);
    }
}
