use std::collections::HashMap;
use near_sdk::collections::{LookupMap, Vector};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{env, AccountId, Balance, Timestamp};
use crate::{SeedId};
pub const MAX_CDACCOUNT_NUM: u128 = 16;
pub const MAX_FARM_NUM: u128 = 16;

/// 放在ContractData，仅owner可以变更
#[derive(BorshSerialize, BorshDeserialize)]
pub struct CDStrategy {
    pub locking_time: Vec<Timestamp>,
    /// gain additional.  
    pub additional: Vec<u32>,
    /// liquidated damages numerator.  
    pub damage: u32,
    /// additional、damage的分母
    pub denominator: u32,
}

/// locking_time, additional通过入参（index：usize，value：u32/Timestamp）进行变更，index >= vec.len()则append
/// damage通过入参（damage：u32）进行变更
/// denominator通过入参（denominator：u32）进行变更
/// 
/// 
/// Certificate of deposit account
/// 实现版本管理
#[derive(BorshSerialize, BorshDeserialize)]
pub struct CDAccount {
    pub seed_id: SeedId,
    /// CDStrategy成员变量index.
    /// CDAccount创建时受到策略影响，
    pub cd_strategy: usize,
    /// from ft_on_transfer、ft_on_transfer amount
    pub staking_amount: Balance,
    /// U256:  staking_amount * CDStrategy.additional[cd_strategy] / CDStrategy.denominator
    pub farming_amount: Balance,
    /// env::block_timestamp()
    pub begin_sec: Timestamp,
    /// begin_sec + CDStrategy.locking_time[cd_strategy]
    pub end_sec: Timestamp
}

/// famer struct
/// farmer结构体新增 cd_accounts: Vector<CDAccount> 字段
/// 
/// ft_on_transfer、ft_on_transfer
/// 判断msg是否是CDAccount json来判断是否为CDAccount操作
/// 遍历famer的cd_accounts如果发现seed_id与cd_strategy相同的，则追加CDAccount（断言end_sec > env::block_timestamp()），否则新建CDAccount
/// internal_seed_deposit函数在接收到msg是cd_account时，amount入参为CDAccount.farming_amount.
/// internal_seed_deposit添加is_cd_account: bool参数，如果是true，不调用farmer.add_seed
/// 
/// claim_user_reward_from_farm
/// 在获取user_seed的时候，要遍历farmer的cd_accounts, 累加seed_id相同的CDAccount.farming_amount
/// 
/// 通过传入farmer.cd_accounts的index删除farmer下CDAccount
/// locking_time = CDAccount.end_sec - CDAccount.begin_sec
/// keeping_time = env::block_timestamp() - CDAccount.begin_sec
/// distribute之后， 违约金额 = U256: staking_amount * CDStrategy.damage / CDStrategy.denominator * (locking_time - keeping_time) / locking_time
/// 最终取回seed数量: CDAccount.staking_amount - 违约金额
/// farm_seed.sub_amount(CDAccount.farming_amount)
enum dfd {
    EE
}
