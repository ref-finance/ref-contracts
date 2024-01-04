use std::ops::{Mul, Div};

use crate::*;
use crate::utils::*;
use uint::construct_uint;

pub const RECORD_INTERVAL_SEC: u32 = 10 * 60; // 10 min 
pub const RECORD_COUNT_LIMIT: usize = 6;

construct_uint! {
    #[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
    #[serde(crate = "near_sdk::serde")]
    pub struct U256(4);
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug))]
#[serde(crate = "near_sdk::serde")]
pub struct CumulativeRecord {
    time_sec: u32,
    cumulative_token_amounts: Vec<U256>
}


#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug))]
#[serde(crate = "near_sdk::serde")]
pub struct UnitShareCumulativeInfo{
    pub last_update_sec: u32,
    pub cumulative_token_amounts: Vec<U256>,
    pub records: Vec<CumulativeRecord>
}

#[derive(BorshDeserialize, BorshSerialize)]
pub enum VUnitShareCumulativeInfo {
    Current(UnitShareCumulativeInfo),
}

impl From<VUnitShareCumulativeInfo> for UnitShareCumulativeInfo {
    fn from(v: VUnitShareCumulativeInfo) -> Self {
        match v {
            VUnitShareCumulativeInfo::Current(c) => c,
        }
    }
}

impl From<UnitShareCumulativeInfo> for VUnitShareCumulativeInfo {
    fn from(shadow_record: UnitShareCumulativeInfo) -> Self {
        VUnitShareCumulativeInfo::Current(shadow_record)
    }
}

#[derive(Serialize, Deserialize)]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug))]
#[serde(crate = "near_sdk::serde")]
pub struct CumulativeRecordView {
    time_sec: u32,
    cumulative_token_amounts: Vec<String>
}


#[derive(Serialize, Deserialize)]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug))]
#[serde(crate = "near_sdk::serde")]
pub struct UnitShareCumulativeInfoView{
    pub last_update_sec: u32,
    pub cumulative_token_amounts: Vec<String>,
    pub records: Vec<CumulativeRecordView>
}

impl From<UnitShareCumulativeInfo> for UnitShareCumulativeInfoView {
    fn from(usci: UnitShareCumulativeInfo) -> Self {
        Self {
            last_update_sec: usci.last_update_sec,
            cumulative_token_amounts: usci.cumulative_token_amounts.iter().map(|v| v.to_string()).collect(),
            records: usci.records.iter().map(|v| CumulativeRecordView{
                time_sec: v.time_sec,
                cumulative_token_amounts: v.cumulative_token_amounts.iter().map(|v| v.to_string()).collect()
            }).collect()
        }
    }
}

impl UnitShareCumulativeInfo {
    pub fn new(current_time_sec: u32, amounts: Vec<u128>) -> Self {
        let amounts: Vec<U256> = amounts.into_iter().map(|x| U256::from(x)).collect();
        Self{
            last_update_sec: current_time_sec,
            cumulative_token_amounts: amounts.clone(),
            records: vec![
                CumulativeRecord {
                    time_sec: current_time_sec,
                    cumulative_token_amounts: amounts
                }
            ]
        }
    }

    pub fn update(&mut self, current_time_sec: u32, amounts: Vec<u128>) {
        let amounts: Vec<U256> = amounts.into_iter().map(|x| U256::from(x)).collect();
        let last_record = &self.records[self.records.len() - 1];
        let time_elapsed = current_time_sec - self.last_update_sec;
        if time_elapsed > 0 {
            let mut new_cumulative_token_amounts = vec![];
            for (index, cumulative_amount) in self.cumulative_token_amounts.iter().enumerate() {
                let (new_cumulative_amount, _) = cumulative_amount.overflowing_add(amounts[index].mul(U256::from(time_elapsed)));
                new_cumulative_token_amounts.push(new_cumulative_amount);
            }
            self.last_update_sec = current_time_sec;
            self.cumulative_token_amounts = new_cumulative_token_amounts;

            if current_time_sec - last_record.time_sec >= RECORD_INTERVAL_SEC {
                self.records.push(CumulativeRecord {
                    time_sec: current_time_sec,
                    cumulative_token_amounts: self.cumulative_token_amounts.clone()
                });

                if self.records.len() > RECORD_COUNT_LIMIT {
                    self.records.remove(0);
                }
            }
        }
    }

    pub fn twap_token_amounts(&self) -> Vec<u128> {
        let earliest_record = &self.records[0];
        let numerators = self.cumulative_token_amounts.iter().zip(earliest_record.cumulative_token_amounts.iter()).map(|(x, y)| {
            let (amount, _) = x.overflowing_sub(*y);
            amount
        }).collect::<Vec<U256>>();
        let denominator = self.last_update_sec - earliest_record.time_sec;
        assert!(denominator > 0, "Just initialized, try again!");
        numerators.into_iter().map(|x| x.div(U256::from(denominator)).as_u128()).collect::<Vec<u128>>()
    }
}

impl Contract {
    pub fn internal_unit_share_token_amounts(&self, pool_id: u64) -> Vec<u128> {
        let mut pool = self.pools.get(pool_id).expect(ERR85_NO_POOL);
        let share_decimals = pool.get_share_decimal();
        pool.remove_liquidity(&String::from("@view"), 10u128.pow(share_decimals as u32), vec![0; pool.tokens().len()], true)
    }

    pub fn internal_update_unit_share_cumulative_info(&mut self, pool_id: u64) {
        if let Some(mut unit_share_cumulative_info) =  self.internal_get_unit_share_cumulative_infos(pool_id) {
            let tokens = self.internal_unit_share_token_amounts(pool_id);
            unit_share_cumulative_info.update(nano_to_sec(env::block_timestamp()), tokens);
            self.internal_set_unit_share_cumulative_infos(pool_id, unit_share_cumulative_info);

        }
    }
}

#[near_bindgen]
impl Contract {
    #[payable]
    pub fn register_pool_twap_record(&mut self, pool_id: u64) {
        assert_one_yocto();
        assert!(self.is_owner_or_guardians(), "{}", ERR100_NOT_ALLOWED);
        assert!(self.unit_share_cumulative_infos.get(&pool_id).is_none(), "Already register");
        let amounts = self.internal_unit_share_token_amounts(pool_id);
        self.internal_set_unit_share_cumulative_infos(pool_id, UnitShareCumulativeInfo::new(nano_to_sec(env::block_timestamp()), amounts));
    }

    #[payable]
    pub fn unregister_pool_twap_record(&mut self, pool_id: u64) {
        assert_one_yocto();
        assert!(self.is_owner_or_guardians(), "{}", ERR100_NOT_ALLOWED);
        self.unit_share_cumulative_infos.remove(&pool_id).expect(ERR85_NO_POOL);
    }

    pub fn sync_pool_twap_record(&mut self, pool_id: u64) {
        let mut unit_share_cumulative_info =  self.internal_unwrap_unit_share_cumulative_infos(pool_id);
        let amounts = self.internal_unit_share_token_amounts(pool_id);
        unit_share_cumulative_info.update(nano_to_sec(env::block_timestamp()), amounts);
        self.internal_set_unit_share_cumulative_infos(pool_id, unit_share_cumulative_info);
    }

    pub fn get_pool_twap_info_view(&self, pool_id: u64) -> Option<UnitShareCumulativeInfoView> {
        if let Some(v) = self.internal_get_unit_share_cumulative_infos(pool_id) {
            Some(v.into())
        } else {
            None
        }
    }

    pub fn list_pool_twap_info_view(&self, from_index: Option<u64>, limit: Option<u64>) -> HashMap<u64, UnitShareCumulativeInfoView>  {
        let keys = self.unit_share_cumulative_infos.keys_as_vector();
        let from_index = from_index.unwrap_or(0);
        let limit = limit.unwrap_or(keys.len());

        (from_index..std::cmp::min(from_index + limit, keys.len()))
            .map(|index| {
                (
                    keys.get(index).unwrap(),
                    self.internal_unwrap_unit_share_cumulative_infos(keys.get(index).unwrap()).into()
                )
            })
            .collect()
    }

    pub fn get_unit_share_twap_token_amounts(&self, pool_id: u64) -> Vec<U128> {
        let mut unit_share_cumulative_info =  self.internal_unwrap_unit_share_cumulative_infos(pool_id);
        let tokens = self.internal_unit_share_token_amounts(pool_id);
        unit_share_cumulative_info.update(nano_to_sec(env::block_timestamp()), tokens);
        unit_share_cumulative_info.twap_token_amounts().into_iter().map(|v| U128(v)).collect()
    }

    pub fn get_unit_share_token_amounts(&self, pool_id: u64) -> Vec<U128> {
        self.internal_unit_share_token_amounts(pool_id).into_iter().map(|v| U128(v)).collect()
    }
}

impl Contract {
    pub fn internal_get_unit_share_cumulative_infos(&self, pool_id: u64) -> Option<UnitShareCumulativeInfo> {
        self.unit_share_cumulative_infos
            .get(&pool_id)
            .map(|o| o.into())
    }

    pub fn internal_unwrap_unit_share_cumulative_infos(&self, pool_id: u64) -> UnitShareCumulativeInfo {
        self.internal_get_unit_share_cumulative_infos(pool_id)
            .expect("unit_share_cumulative_infos is not find")
    }

    pub fn internal_set_unit_share_cumulative_infos(&mut self, pool_id: u64, unit_share_cumulative_infos: UnitShareCumulativeInfo) {
        self.unit_share_cumulative_infos.insert(&pool_id, &unit_share_cumulative_infos.into());
    }
}

#[cfg(test)]
mod twap {
    use super::*;

    #[test]
    fn test_base() {
        let mut usci = UnitShareCumulativeInfo::new(1000, vec![100u128, 100, 100, 10000]);
        assert!(usci.cumulative_token_amounts.iter().zip(vec![100u128, 100, 100, 10000]).all(|(x, y)| x.as_u128() == y));
        assert!(usci.records.len() == 1);
        assert!(usci.records[0].time_sec == 1000);
        assert!(usci.records[0].cumulative_token_amounts.iter().zip(vec![100u128, 100, 100, 10000]).all(|(x, y)| x.as_u128() == y));

        usci.update(2000, vec![200u128, 200, 200, 20000]);
        assert!(usci.records.len() == 2);
        
        assert!(usci.cumulative_token_amounts.iter()
            .zip(vec![100u128, 100, 100, 10000])
            .zip(vec![200u128, 200, 200, 20000])
            .all(|((res, x), y)|{
                res.as_u128() == x + y * 1000
            }));

        assert!(usci.cumulative_token_amounts.iter().zip(usci.records[1].cumulative_token_amounts.iter()).all(|(x, y)| x.eq(y)));
        assert!(usci.twap_token_amounts().iter()
            .zip(usci.records[0].cumulative_token_amounts.iter())
            .zip(usci.records[1].cumulative_token_amounts.iter())
            .all(|((res, x), y)|{
                *res == (y - x).as_u128() / 1000
            }));
    }


    #[test]
    fn test_long_time_no_operation() {
        let mut usci = UnitShareCumulativeInfo::new(1000, vec![100u128, 100, 100, 10000]);
        assert!(usci.cumulative_token_amounts.iter().zip(vec![100u128, 100, 100, 10000]).all(|(x, y)| x.as_u128() == y));
        assert!(usci.records.len() == 1);
        assert!(usci.records[0].time_sec == 1000);
        assert!(usci.records[0].cumulative_token_amounts.iter().zip(vec![100u128, 100, 100, 10000]).all(|(x, y)| x.as_u128() == y));

        usci.update(1010, vec![100u128, 100, 100, 10000]);
        assert!(usci.records.len() == 1);
        assert!(usci.twap_token_amounts().into_iter().zip(vec![100u128, 100, 100, 10000]).all(|(x, y)| x == y));

        usci.update(1000 + RECORD_INTERVAL_SEC, vec![100u128, 100, 100, 10000]);
        assert!(usci.records.len() == 2);
        assert!(usci.twap_token_amounts().into_iter().zip(vec![100u128, 100, 100, 10000]).all(|(x, y)| x == y));

        usci.update(1000 + 2 * RECORD_INTERVAL_SEC, vec![100u128, 100, 100, 10000]);
        assert!(usci.records.len() == 3);
        assert!(usci.twap_token_amounts().into_iter().zip(vec![100u128, 100, 100, 10000]).all(|(x, y)| x == y));

        usci.update(1000 + 6 * RECORD_INTERVAL_SEC, vec![100u128, 100, 100, 10000]);
        assert!(usci.records.len() == 4);
        assert!(usci.twap_token_amounts().into_iter().zip(vec![100u128, 100, 100, 10000]).all(|(x, y)| x == y));

        usci.update(1000 + 7 * RECORD_INTERVAL_SEC, vec![100u128, 100, 100, 10000]);
        assert!(usci.records.len() == 5);
        assert!(usci.twap_token_amounts().into_iter().zip(vec![100u128, 100, 100, 10000]).all(|(x, y)| x == y));

        usci.update(1000 + 20 * RECORD_INTERVAL_SEC, vec![100u128, 100, 100, 10000]);
        assert!(usci.records.len() == 6);
        assert!(usci.twap_token_amounts().into_iter().zip(vec![100u128, 100, 100, 10000]).all(|(x, y)| x == y));

        usci.update(1000 + 50 * RECORD_INTERVAL_SEC, vec![100u128, 100, 100, 10000]);
        assert!(usci.records.len() == 6);
        assert!(usci.twap_token_amounts().into_iter().zip(vec![100u128, 100, 100, 10000]).all(|(x, y)| x == y));
    }
}
