# Test Token

## build
```bash
source build.sh
```

## deploy
```bash
near deploy token.testnet res/test_token.wasm --account_id=token.testnet
near call token.testnet new '{"name": "your token name", "symbol": "YTS", "decimals": 24}' --account_id=token.testnet

near view token.testnet ft_metadata
# if need set icon
near call token.testnet set_icon "{\"icon\": \"xxx...xxx\"}" --account_id=token.testnet
```

## usage

```bash
# register
near call token.testnet storage_deposit '{"account_id": "alice.testnet"}' --account_id=alice.testnet --amount=0.00125

# mint fake token
near call token.testnet mint '{"account_id": "alice.testnet", "amount": "1000000000000"}' --account_id=alice.testnet

# burn fake token
near call token.testnet burn '{"account_id": "alice.testnet", "amount": "1000000000000"}' --account_id=alice.testnet
```