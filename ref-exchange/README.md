# Multiswap

This is a contract that contains many token swap pools.
Each pool can have up to 10 tokens and it's own fee %.

## Usage

- deposit funds / withdraw funds of the contract's virtual balance. User can maintain up to 10 distinct tokens on their balance.
- create a pool with specific set of tokens and a fee, get `pool_id`
- add liquidity to specific pool from the funds deposited
- remove liquidity from specific pool back into deposited funds on the contract
- with funds in the pool, call swap to trade 
