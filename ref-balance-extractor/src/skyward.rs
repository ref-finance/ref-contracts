use crate::*;
use near_sdk::{BlockHeight, Duration, Timestamp};

pub(crate) type InnerU256 = [u64; 4];
pub type BasicPoints = u16;
pub(crate) const MULTIPLIER: u128 = 10u128.pow(38);

#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub struct Subscription {
    pub shares: Balance,
    pub last_in_balance: Balance,
    pub spent_in_balance_without_shares: Balance,
    pub last_out_token_per_share: Vec<InnerU256>,
    pub claimed_out_balance: Vec<Balance>,
    pub referral_id: Option<AccountId>,
}

impl Subscription {
    pub fn touch(&mut self, sale: &Sale) -> Vec<Balance> {
        let shares = U256::from(self.shares);
        let multiplier = U256::from(MULTIPLIER);
        self.last_out_token_per_share
            .iter_mut()
            .zip(sale.out_tokens.iter())
            .map(|(last_out_token_per_share, out_token)| {
                let out_token_per_share = U256(out_token.per_share.clone());
                let u256_last_out_token_per_share = U256(last_out_token_per_share.clone());
                let out_token_amount = if out_token_per_share == U256::zero() {
                    0
                } else {
                    let diff = out_token_per_share - u256_last_out_token_per_share;
                    (diff * shares / multiplier).as_u128()
                };
                *last_out_token_per_share = out_token_per_share.0;
                out_token_amount
            })
            .collect()
    }
}

#[derive(BorshDeserialize, BorshSerialize, Clone)]
pub enum VSubscription {
    Current(Subscription),
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct SkywardAccount {
    pub balances: UnorderedMap<TokenAccountId, Balance>,
    pub subs: UnorderedMap<u64, VSubscription>,
    pub sales: UnorderedSet<u64>,
}

#[derive(BorshDeserialize, BorshSerialize)]
pub enum VAccount {
    Current(SkywardAccount),
}

impl VAccount {
    pub fn parse(&mut self, state: &mut State) {
        match self {
            VAccount::Current(account) => {
                account.balances.parse(state);
                account.subs.parse(state);
                account.sales.parse(state);
            }
        }
    }
}

#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub struct SaleOutToken {
    pub token_account_id: TokenAccountId,
    pub remaining: Balance,
    pub distributed: Balance,
    pub treasury_unclaimed: Option<Balance>,
    pub per_share: InnerU256,
    pub referral_bpt: Option<BasicPoints>,
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct Sale {
    pub owner_id: AccountId,

    pub title: String,
    pub url: Option<String>,
    pub permissions_contract_id: Option<AccountId>,

    pub out_tokens: Vec<SaleOutToken>,

    pub in_token_account_id: AccountId,
    pub in_token_remaining: Balance,
    pub in_token_paid_unclaimed: Balance,
    pub in_token_paid: Balance,

    pub start_time: Timestamp,
    pub duration: Duration,

    pub total_shares: Balance,
    pub last_timestamp: Timestamp,

    pub start_block_height: BlockHeight,
    pub end_block_height: Option<BlockHeight>,
}

#[derive(BorshDeserialize, BorshSerialize)]
pub enum VSale {
    First,
    Current(Sale),
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct VestingInterval {
    pub start_timestamp: Timestamp,
    pub end_timestamp: Timestamp,
    pub amount: Balance,
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct Treasury {
    pub balances: UnorderedMap<TokenAccountId, Balance>,
    pub skyward_token_id: TokenAccountId,

    pub skyward_burned_amount: Balance,
    pub skyward_vesting_schedule: LazyOption<Vec<VestingInterval>>,

    pub listing_fee_near: Balance,

    pub w_near_token_id: TokenAccountId,

    // The amount of NEAR locked while the permissions are being verified.
    pub locked_attached_deposits: Balance,
}

impl Treasury {
    pub fn parse(&mut self, state: &mut State) {
        self.balances.parse(state);
        self.skyward_vesting_schedule.parse(state);
    }
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct SkywardContract {
    pub accounts: LookupMap<AccountId, VAccount>,

    pub sales: LookupMap<u64, VSale>,

    pub num_sales: u64,

    pub treasury: Treasury,
}

impl SkywardContract {
    pub fn parse(&mut self, state: &mut State) {
        self.accounts.parse(state);
        for account in self.accounts.data.values_mut() {
            account.parse(state);
        }
        self.sales.parse(state);
        self.treasury.parse(state);
    }
}
