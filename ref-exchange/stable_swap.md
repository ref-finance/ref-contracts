# Stable Swap Pool Instruction

## Logic
---
It is for swapping among stable coins.  

The stable swap pool can have more than 2 kinds of tokens (maximum 9 kinds of tokens with each 100 billion balances in simulation test ENV).  

The decimal of each token must in [1, 18].  

The most likely first stable pool in REF would be [nDAI, nUSDT, nUSDC].  

## Interfaces
---
### Create Stable Swap Pool
Only owner or guardians of the ref-exchange contract can create stable swap pool.
```Bash
near call ref-exchange.testnet add_stable_swap_pool '{"tokens": ["ndai.testnet", "nusdt.testnet", "nusdc.testnet"], "decimals": [18, 6, 6], "fee": 25, "amp_factor": 100000}' --account_id=owner.testnet --amount=1
# it will return pool_id
```

### Add Initial Liquidity
This interface is only for stable swap pools. Anyone can supply initial liquidity, but all tokens should be filled.
```Bash
# add 1 dai, 1 usdt and 1 usdc as initial liquidity with minimum 3 lpt shares
near call ref-exchange.testnet add_stable_liquidity '{"pool_id": 100, "amounts": ["1000000000000000000", "1000000", "1000000"], "min_shares": "3000000000000000000"}' --account_id=owner.testnet --amount=1
# will return actually minted lpt shares
```

### Add Subsequent Liquidity
This interface is only for stable swap pools. Anyone can supply subsequent liquidity with subset of tokens.
```Bash
# add 100 dai, 10 usdt and 0 usdc with minimum 103 lpt shares
near call ref-exchange.testnet add_stable_liquidity '{"pool_id": 100, "amounts": ["100000000000000000000", "10000000", "0"], "min_shares": "103000000000000000000"}' --account_id=user.testnet --amount=1
# will return actually minted lpt shares
```

### Withdraw Liquidity by Share
Anyone can withdraw liquidity by shares. the output less than `min_amounts` would cause TX failure.
```Bash
# withdraw 100 shares with min_amount 10 balance each
near call ref-exchange.testnet remove_liquidity '{"pool_id": 100, "shares": "100000000000000000000", "min_amounts": ["10000000000000000000", "10000000", "10000000"]}' --account_id=user.testnet --amount=0.000000000000000001
```

### Withdraw Liquidity by Tokens
This interface is only for stable swap pools. Anyone can withdraw liquidity by tokens. It will return designated tokens to user's inner account, but if burned shares more than `max_burn_shares` would cause TX failure.
```Bash
# withdraw 50 nUSDT with max_burn_shares 60 balance
near call ref-exchange.testnet remove_liquidity_by_tokens '{"pool_id": 100, "max_burn_shares": "60000000000000000000", "amounts": ["0", "50000000", "0"]}' --account_id=user.testnet --amount=0.000000000000000001
# will return actually burned lpt shares
```

### Swap in Stable Swap Pool
This interface is same as regular pool.
```Bash
# swap 1 nusdt to nusdc and if output is less than 0.99 the TX would failure
near call ref-exchange.testnet swap '{"actions": [{"pool_id": 100, "token_in": "nusdt.testnet", "amount_in": "1000000", "token_out": "nusdc.testnet", "min_amount_out": "990000"}], "referral_id": "referral.testnet"}' --account_id=user.testnet --amount=0.000000000000000001
```
