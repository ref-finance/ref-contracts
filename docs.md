# Ref Finance

[Ref Finance](https://ref.finance) is the protocol for creating synthetic tokenized assets on NEAR blockchain.

It uses NEAR as collateral and allows to mint any token that has a whitelisted price feed, for example: rUSD, rBTC, rTESLA, rSPBEAR (short S&P500) or rOIL.

The protocol will be governed by an instance of SputnikDAO.

## Approach

There are two main approaches to create synthetic assets:
- Collateralized Debt Position, where the position is over collateralized and serves to borrow the synthetic asset
- A fractional reserve approach, selling tokens above the peg value and allowing to trade in below with various incentives.

Ref Finance is taking an approach of fractional reserve that is used to provide liquidity instead of just sitting as collateral, while allowing people to use other lending protocols for CDP-like experience.

*This design leverages Fei Protocol's stabilization mechanics extended to multiple assets.*

Ref Finance maintains internal pools of <collateral, synthetic asset> for each asset. These assets provide liquidity in the form of Uniswap exchange.

Additionally new assets are minted with price above `x%` (configurable parameter) the oracle price. This allows to expand the supply when there is demand.

If the price is below the pool price:
- For anyone who is buying under the price, additional tokens are minted to reward them based on time-weighted difference with the market price `r * num_blocks * (target_price - current_price) * amount`.
- If someone is selling more below the oracle price - the slippage increases and extra tokens `(target_price - end_price) ^ 2 * 100 * amount` are burnt.

If that didn't help to bring the price, the pool can be reweighted by burning the extra Ref tokens to bring price to the target price. Condition for reweight to happen if `num_blocks = (target_price - current_price) * 100 / r`.

There is a central contract "Controller" that provides all operations including minting and exchange functionality.

Next functions are available on `Controller`:
- `add_asset(asset_name, oracle_feed, config)` - adds new assets to available with given oracle feed. Only allowed by `owner`.
- `set_asset_config(asset_name, config)` - sets config including all the trading parameters for given asset. Only allowed by `owner`.
- `buy(asset_name, amount_out)` - buy given `amount_out` of `asset_name`, must attach enough $NEAR to cover the trade. This will first buy from the Uniswap formula (rewarding with new minted tokens if the price of liquidity below oracle price) and if the amount brings price above current bonding curve - will buy from bonding curve expanding the supply.
- `sell(asset_name, amount_in, amount_out)` - (works as callback from `asset_name`) sell `amount_in` of given `asset_name` and expecting at least `amount_out` of $N. This uses Uniswap curve and uses quadratic penalty to disinsentivize large sell orders. Penalty is getting burnt.
- `trade(asset_name_in, amount_in, asset_name_in, amount_out)` - (works as callback from `asset_name_in`) trade two assets between each other. Uses internal calculations to exchange them to save on the fees.
- `reweight(asset_name)` - if the internal exchange rate is way below the oracle price for given `asset_name` and ??? - allows to reweight the Uniswap pool to bring it back to the pool.
- `stage_upgrade(contract)` - Stage upgrade for this contract. Only allowed by `owner`.
- `upgrade()` - Upgrade to staged contract. Only allowed by `owner`.

Each Ref Token will have it's own contract living under sub-name of the Controller contract. These token contracts provide `owner` ability for Controller to mint / burn assets .

Future features:
- Allowing to add external capital as LP to the exchange.

# References

- MarkerDAO - https://makerdao.com/whitepaper/
- Synthetix - https://synthetix.io/
- Mirror Protocol - https://mirror.finance/
- Fei Protocol - https://fei.money/static/media/whitepaper.7d5e2986.pdf
