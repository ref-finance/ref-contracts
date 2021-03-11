# Ref Finance Contracts

This mono repo contains the source code for the smart contracts of Ref Finance on [NEAR](https://near.org).

## Contracts

| Contract | Reference | Description |
| - | - | - |
| [test-token](test-token/src/lib.rs) | - | Test token contract |
| [ref-exchange](ref-exchange/src/lib.rs) | [docs](https://ref-finance.gitbook.io/ref-finance/smart-contracts/ref-exchange) | Main exchange contract, that allows to deposit and withdraw tokens, exchange them via various pools |

## Development

1. Install `rustup` via https://rustup.rs/
2. Run the following:

```
rustup default stable
rustup target add wasm32-unknown-unknown
```

### Testing

Contracts have unit tests and also integration tests using NEAR Simulation framework. All together can be run:

```
cd ref-exchange
cargo test --all
```

### Compiling

You can build release version by running next scripts inside each contract folder:

```
cd ref-exchange
./build.sh
```

### Deploying to TestNet

To deploy to TestNet, you can use next command:
```
near dev-deploy
```

This will output on the contract ID it deployed.
