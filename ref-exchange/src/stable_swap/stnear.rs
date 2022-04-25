use near_sdk::{ext_contract, json_types::U128, near_bindgen, AccountId, PromiseOrValue};

use crate::*;

use super::PRECISION;

pub const METAPOOL_ADDRESS: &str = "metapool.near";

pub mod gas {
    use near_sdk::Gas;

    /// The base amount of gas for a regular execution.
    const BASE: Gas = 10_000_000_000_000;

    /// The amount of gas for cross-contract call
    pub const GET_PRICE: Gas = BASE;

    /// The amount of gas for callback
    pub const CALLBACK: Gas = BASE;
}

#[ext_contract(ext_metapool)]
pub trait ExtMetapool {
    //https://github.com/Narwallets/meta-pool/blob/40636d9004d28dc9cb214b9703692061c93f613c/metapool/src/owner.rs#L254
    fn get_st_near_price(&self) -> U128;
}

#[ext_contract(ext_self)]
pub trait SelfCallbacks {
    fn st_near_price_callback(&mut self, pool_id: u64, #[callback] price: U128) -> U128;
}

#[near_bindgen]
impl Contract {

    /// 
    #[payable]
    pub fn update_pool_rates(&mut self, pool_id: u64) -> PromiseOrValue<U128> {
        let pool = self.pools.get(pool_id).expect(ERR85_NO_POOL);
        match pool {
            Pool::SimplePool(_) => unimplemented!(),
            Pool::StableSwapPool(pool) => {
                if pool.rates_updated_at == env::epoch_height() {
                    return PromiseOrValue::Value(pool.stored_rates[0].into());
                }
            }
        }

        ext_metapool::get_st_near_price(&AccountId::from(METAPOOL_ADDRESS), 0, gas::GET_PRICE)
            .then(ext_self::st_near_price_callback(
                pool_id,
                &env::current_account_id(),
                0,
                gas::CALLBACK,
            ))
            .into()
    }

    /// 
    #[private]
    fn st_near_price_callback(&mut self, pool_id: u64, #[callback] price: U128) -> U128 {
        let mut pool = self.pools.get(pool_id).expect(ERR85_NO_POOL);
        match &mut pool {
            Pool::SimplePool(_) => unimplemented!(),
            Pool::StableSwapPool(pool) => {
                let mut rates = vec![1 * PRECISION; pool.tokens().len()];
                rates[0] = price.0;
                pool.stored_rates = rates;
                pool.rates_updated_at = env::epoch_height();
            }
        }
        self.pools.replace(pool_id, &pool);
        price
    }
}
