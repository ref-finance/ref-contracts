# Release Notes

### Version 1.7.1
```
DoZzCXpLVetkNgNWtavriquzb7mkMmm83rbNdoLsgh5P
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
