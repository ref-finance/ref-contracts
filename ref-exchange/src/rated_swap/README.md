### RatedSwapPool for stNEAR/wNEAR pool

Based on StableSwapPool source code

Major changes:
- added rates to swap & add/predict/remove_liquidity calculations
- math upgraded to 24 decimals using U384
- rates acquired from another smart contract via cross-contract call

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