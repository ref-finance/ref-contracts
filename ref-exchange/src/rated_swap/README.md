### Generic RatedSwapPool and implementation for stNEAR/wNEAR pool

Based on StableSwapPool source code

Major changes:
- added rates to swap & add/predict/remove_liquidity calculations
- math upgraded to 24 decimals using U384
- generic rates acquisition from another smart contract via cross-contract call with caching
- minimum boilerplate for rates acquisition implementation
- includes sample implementation of rates acquisition for stNEAR

New external methods:
- ```add_rated_swap_pool```
- ```add_rated_liquidity```
- ```rated_swap_ramp_amp```
- ```rated_swap_stop_ramp_amp```
- ```get_rated_pool``` [view]
- ```get_rated_return``` [view]
- ```predict_add_rated_liquidity``` [view]
- ```predict_remove_rated_liquidity_by_tokens``` [view]
- ```update_pool_rates```
- ```update_pool_rates_callback``` [callback]

Add liquidity flow:
- call ```get_rated_pool``` & check rates are actual
- call ```predict_add_rated_liquidity``` with actual rates
- batch-call [```update_pool_rates```, ```add_rated_liquidity```]

Remove liquidity by tokens flow:
- call ```get_rated_pool``` & check rates are actual
- call ```predict_remove_rated_liquidity_by_tokens``` with actual rates
- batch-call [```update_pool_rates```, ```remove_liquidity_by_tokens```]

Remove liquidity by shares flow:
- same as StableSwapPool

Swap flow:
- call ```get_rated_pool``` & check rates are actual
- call ```get_rated_return``` with actual rates
- batch-call [```update_pool_rates```, ```swap```]

Rates acquisition implementation:
- implement instance of ```Rates``` enum with ```RatesTrait```

```rs
pub trait RatesTrait {
    /// Check chached rates are actual
    fn are_actual(&self) -> bool;
    /// Get chached rates vector
    fn get(&self) -> &Vec<Balance>;
    /// Update cached rates
    ///  if cached rates are actual returns true
    ///  else returns cross-contract call promise
    fn update(&self) -> PromiseOrValue<bool>;
    /// Update callback
    ///  receives JSON encoded cross-contract call result
    ///  and updates cached rates
    fn update_callback(&mut self, cross_call_result: &Vec<u8>) -> bool;
}
```