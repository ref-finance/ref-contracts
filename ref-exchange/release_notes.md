# Release Notes

### Version 1.9.17
```
MiVpPDrEDA7akuFgcSCbKic1cG99oX48qageSLHvdpK
```
1. improve lostfound workflow, user can withdraw their lostfound by themselves.
2. fix pyth price update issue on multiple tokens with same pyth price_id.

### Version 1.9.16
```
BXSNfiJC2LbVUgRTpHxAYfLiPYDwAn7cXLKJeTsVHRkG
```
1. use 10Tgas for transfer near call back.

### Version 1.9.15
```
6VT8PzHwyphkL64Gi7GLS8YyBqmeVU8y8xjSd5ej53sF
```
1. reduce gas for callback of client_echo ft_transfer_call from 20T to 5T.
2. add a new optional arg 'extra_tgas_for_client_echo' for client_echo action.   
```rust
Execute {
    referral_id: Option<ValidAccountId>,
    /// List of sequential actions.
    actions: Vec<Action>,
    /// If not None, use ft_transfer_call
    /// to send token_out back to predecessor with this msg.
    client_echo: Option<String>,
    skip_unwrap_near: Option<bool>,
    swap_out_recipient: Option<ValidAccountId>,
    skip_degen_price_sync: Option<bool>,
    /// extra Tgas for ft_on_transfer
    extra_tgas_for_client_echo: Option<u32>,
},
```
The default value of this arg is 15, means an extra 15T gas will be passed to the token contract beyond the 30T gas requried by NEP-141 'ft_transfer_call'. The client echo sender can set a small number to save gas, for example:  
```bash
# manually set extra gas to 7T, so the token_out.near contract would be guaranteed to have at least 37Tgas as prepaid gas in its 'ft_transfer_call'.
{ "msg": "{\"force\":0,\"actions\":[{\"pool_id\":0,\"token_in\":\"token_in.near\",\"token_out\":\"token_out.near\",\"min_amount_out\":\"1\"}],\"client_echo\":\"{\"receiver_id\":\"echo_receiver.near\"}\",\"extra_tgas_for_client_echo\":7}", "amount": "1000", "sender_id": "echo_sender.near" }
```


### Version 1.9.14
```
G1mrfT8dceTrrjn95LeCq6HS2LbFktd9XWp49oXZAdXH
```
1. add secure_sender_whitelist in client echo feature, if a sender falls in ssw, client_echo_token whitelist would be ignored.
2. add prefix wildcard '\*' support in client_echo whitelists. In practise, it always starts with '\*.' to indicate sub-accounts.

### Version 1.9.13
```
4izBbspd1Uiu1vjYcWpkebzoVxqmiZK7ZXSn4b2fAF2D
```
1. add batch_views.

### Version 1.9.12
```
68GhAQax4ndABL7Ks1sVncmJDuendk6hDETCq4dqS5rJ
```
1. add get_pool_shares_batch.

### Version 1.9.11
```
GKNsi9JsTWKHbAjxUPpT2rArVUuHsgf9NxA6TtPejTEd
```
1. add SwapVolumeU256.

### Version 1.9.10
```
H2sfzjphuQSTzrQByyPXoAnQnkYFejWCNsT9uwq51XnS
```
1. amendments according to audition recommendations.
2. add skip_degen_price_sync param.

### Version 1.9.9
```
2VqDt4y5CYDi1FCKu5LLVCTXNzh3kEvXpjfSe1XbDJcG
```
1. Reorg stable/rated/degen pool MIN_RESERVE to inner_precision / 1000;

### Version 1.9.8
```
CNFJRDekcistiyBHyZFif4CupZjND91VAj8hgR1E3Q35
```
1. add batch update degen price function
2. add token check to hotzap.

### Version 1.9.7
```
CMN4goNWHQjsXevLbqAC9nXKTw1yeJqysEfB647uuyro
```
1. fix identity verification for the whitelisted_postfix related functions.
2. add donation functions.
3. add mft_unregister.

### Version 1.9.6
```
2Yo8qJ5S3biFJbnBdb4ZNcGhN1RhHJq34cGF4g7Yigcw
```
1. add PoolDetailInfo view functions

### Version 1.9.5
```
3hF1UzsT5mzxbJLMA8BD7gmCH1BL8cWjvFmSZYREpZXK
```
1. add client echo limit

### Version 1.9.4
```
DBz69SAuDcvGWrEraoKjNEMiiDxE3PagejSfhYw3SfqH
```
1. add pool limit
2. add execute_actions_in_va

### Version 1.9.3
```
1PW1wtYsciZKsaRNqNMpY3P1W2wD42PjZVraL142VN4
```
1. add degen pool
2. add swap by output for simple pool

### Version 1.9.2
```
52Fmd38fqZbHoGQGRTmoXxr9xMu8zKQWaGggHSJYi23T
```
1. margin trading

### Version 1.9.1
```
Fdv9RSb4JgK7E76Z1RhJw69ZzzYQ2gG5XfnSdYi5nksq
```
1. support new rated token: sFrax.

### Version 1.9.0
```
B83JY6Ga7A82ojKyYjQBsFBA45EAxgAPpjjzqqHcp9rH
```
1. support skip_unwrap_near indication in swap.
2. support auto_whitelist tokens according to their postfix of account ids.

### Version 1.8.0
```
Gnwp8fNWrPWZ7NR4867cNepgCkX2uj85zExzcMNZiGws
```
1. lp as collateral.

### Version 1.7.2
```
9KuaBbp9FT1g17YCaCsaezGxWDuZkKrjPsXu1RCN8Ux7
```
1. add modify_total_fee;

### Version 1.7.1
```
6ZU3rmDwEs988pvYWyDxg6DJm7fP1F3FnAwHGo698ATw
```
1. import hotzap;

### Version 1.7.0
1. amendments according to audition recommendations;
2. add mft_has_registered interface for user to query whether or not they need to register as LP for a given pool;
3. Let referrals to be managed by DAO and Guardians with independent referral fee rate.

### Version 1.6.2
1. add view interface get_pool_by_ids

### Version 1.6.1
1. fix a rounding bug when adding liquidity

### Version 1.6.0
1. Add frozenlist feature;
2. panic if at least one token not exist in list when remove tokens from whitelist or frozenlist;

### Version 1.5.3
1. Support nearx in rated stable pool;
2. Add check when initializing simple pool liquidity
3. Add ERR36_SHARES_TOTAL_SUPPLY_OVERFLOW 

### Version 1.5.2
1. Lower MIN_RESERVE to 1*10**15;

### Version 1.5.1
1. Import rated stable pool;
2. Add return value to withdraw callback;

### Version 1.4.5
1. Fix off-by-one issue in stable-swap;
2. Support up to 24 decimal tokens in stable-swap;
3. Avoid mandatory all token register when add stable liquidity with subset of tokens;

### Version 1.4.4
1. Return minted shares for `add_liquidity`;
2. Return received tokens amount for `remove_liquidity`;
3. Request one yocto deposit in all owner and guardians interfaces;
4. Unify error msg;
5. Other modification according to audition recommendation;

### Version 1.4.3
1. Let both guardians and owner can remove exchange liquidity to owner inner account by remove_exchange_fee_liquidity;
2. Let both guardians and owner can withdraw owner token to owner wallet by withdraw_owner_token;

### Version 1.4.2
1. Let owner can retrieve unmanaged NEP-141 tokens in contract account;
2. support withdraw token's full amount in inner-account with 0 in amount parameter;

### Version 1.4.1
1. Introduce Stable-Swap-Pool;

### Version 1.4.0
1. Make exchange fee and referal fee inclusive in total fee;
2. Make exchange fee (in the form of lp shares) belongs to contract itself;

### Version 1.3.1
1. Apply HOTFIX in v1.0.3;

### Version 1.3.0
---
1. feature instant swap;  
Allows to swap with a single transaction without needing to deposit / withdraw. Not even storage deposits are required for the pool (inner account not touched). But FE must make sure that receiver is registered in the outgoing token, or they would go to inner account or lost-found account.  
Example usage: 
    ```bash
    contract.ft_transfer_call(
        to_va(swap()),
        to_yocto("1").into(),
        None,
        "{{\"actions\": [{{\"pool_id\": 0, \"token_in\": \"dai\", \"token_out\": \"eth\", \"min_amount_out\": \"1\"}}]}}".to_string()
    ),
    ```  
    Specifically for TokenReceiverMessage message parameters are:  
    ```rust
    enum TokenReceiverMessage {
        /// Alternative to deposit + execute actions call.
        Execute {
            referral_id: Option<ValidAccountId>,
            /// List of sequential actions.
            actions: Vec<Action>,
        },
    }
    ```
    where Action is either SwapAction or any future action added there.


### Version 1.2.0
---
1. upgrade inner account;
    * inner account upgrade to use `UnorderedMap`;
    * keep exist deposits in `legacy_tokens` in `HashMap`; 
    * move it to `tokens` in `UnorderedMap` when deposit or withdraw token;
    
### Version 1.1.0
---
1. feature Guardians;
    * guardians are managed by owner;
    * guardians and owner can switch contract state to Paused;
    * owner can resume the contract;
    * guardians and owner can manager global whitelist;
    * a new view method metadata to show overall info includes version, owner, guardians, state, pool counts.

### Version 1.0.3
---
1. HOTFIX -- increase ft_transfer GAS from 10T to 20T;

### Version 1.0.2
---
1. fixed storage_withdraw bug;
