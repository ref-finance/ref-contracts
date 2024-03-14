# Ref V1 User Interfaces

## user swap
```bash
# prerequisite: make sure you have registered the swap-out token
near view token_out.testnet storage_balance_of '{"account_id": "alice.testnet"}'
# if not, register it
near call token_out.testnet storage_deposit '{"account_id": "alice.testnet", "registration_only": true}' --accountId=alice.testnet --deposit=1

# compose your instant swap actions
export ACT='{\"pool_id\": 123, \"token_in\": \"token_in.testnet\", \"token_out\": \"token_out.testnet\", \"min_amount_out\": \"0\"}'
export ACTS='{\"actions\": ['$ACT']}'

#   multiple actions are also supported
export ACT1='{\"pool_id\": 123, \"token_in\": \"token1.testnet\", \"token_out\": \"token2.testnet\", \"min_amount_out\": \"0\"}'
export ACT2='{\"pool_id\": 456, \"token_in\": \"token2.testnet\", \"token_out\": \"token3.testnet\", \"min_amount_out\": \"0\"}'
export ACTS='{\"actions\": ['$ACT1', '$ACT2']}'

#   referral_id is optional
export ACTS='{\"actions\": ['$ACT'],\"referral_id\":\"referral.testnet\"}'

#   can set optional skip_unwrap_near to false to request auto-unwrap wnear to near if the finally swap_out token is wnear
#   the default behavior is to skip the unwrap, sent wnear directly.
export ACTS='{\"actions\": ['$ACT'],\"skip_unwrap_near\":false}'

# finally, send the swap request
near call token1.testnet ft_transfer_call '{"receiver_id": "ref-v1.testnet", "amount": "1000", "msg": "'$ACTS'"}' --accountId=alice.testnet --depositYocto=1 --gas=100$TGAS
```

## liquidity management
### inner account
```bash
# user should establish inner account before interact with liquidity
# prerequisit: register to be ref-v1 user
near call ref-v1.testnet storage_deposit '{"account_id": "alice.testnet", "registration_only": false}' --accountId=alice.testnet --deposit=0.1
near view ref-v1.testnet storage_balance_of '{"account_id": "alice.testnet"}'

# deposit tokens into ref-v1 
near call token1.testnet ft_transfer_call '{"receiver_id": "ref-v1.testnet", "amount": "100", "msg": ""}' --accountId=alice.testnet --depositYocto=1  --gas=100$TGAS
#   can check the deposited tokens in user's ref-v1 inner account
near view ref-v1.testnet get_deposits '{"account_id": "alice.testnet"}'
near view ref-v1.testnet get_deposit '{"account_id": "alice.testnet", "token_id": "token1.testnet"}'

# withdraw at any time
#   the skip_unwrap_near indicator also works here
near call ref-v1.testnet withdraw '{"skip_unwrap_near": false, "token_id": "wrap.testnet","amount": "1000000000000000000000000","unregister": false}' --account_id=alice.testnet --depositYocto=1 --gas=100$TGAS
```
### Add Liquidity
Use tokens in inner account to add liquidity to the given pool.
```bash
# query pools
near view ref-v1.testnet get_pool_by_ids '{"pool_ids": [1, 2, 5]}'
near view ref-v1.testnet get_stable_pool '{"pool_id": 0}'
near view ref-v1.testnet get_rated_pool '{"pool_id": 1}'
near view ref-v1.testnet get_pool '{"pool_id": 2}'
near view ref-v1.testnet get_pool_share_price '{"pool_id": 3}'
# add liquidity to the pool,
#   the tokens would be added proportionally, 
#   and the unused part would stay in user's ref-v1 inner account
near call ref-v1.testnet add_liquidity '{"pool_id": 123, "amounts":[ "100","200"]}' --accountId=alice.testnet --deposit=0.01

#   for stable pools and rated stable pools, can add liquidity with a subset of tokens and arbitrary amounts
near call ref-v1.testnet add_liquidity '{"pool_id": 1234, "amounts":[ "100","0"]}' --accountId=alice.testnet --deposit=0.01

# user can check his lp token balance
near view ref-v1.testnet mft_balance_of '{"token_id": ":123", "account_id": "alice.testnet"}'
```
### Transfer Liquidity
```bash
# prerequisit: make sure the recipient has registered the lp token on the given pool
near view ref-v1.testnet mft_has_registered '{"token_id":":123", "account_id": "bob.testnet"}'
near call ref-v1.testnet mft_register '{"token_id":":123", "account_id": "bob.testnet"}' --accountId=alice.testnet --deposit=0.01

# transfer lp token to other user, memo is optional
near call call ref-v1.testnet mft_transfer '{"token_id":":123", "receiver_id": "bob.testnet", "amount": "12345", "memo": "test transfer"}' --accountId=alice.testnet --depositYocto 1 --gas=100$TGAS
```

### remove liquidity
Tokens acquired from remove liqidity goes to user's inner account.
```bash
# remove by shares
near call ref-v1.testnet remove_liquidity '{"pool_id": 123, "shares": "1234", "min_amounts": ["0", "0"]}' --accountId=alice.testnet --depositYocto=1

# for stable pools and rated stable pools, can also remove by tokens
near call ref-v1.testnet remove_liquidity_by_tokens '{"pool_id": 1234, "amounts": [ "123", "98"], "max_burn_shares": "120"}' --account_id=alice.testnet --depositYocto=1
```