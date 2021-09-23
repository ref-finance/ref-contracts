# Release Notes

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

### Version 1.0.2
---
1. fixed storage_withdraw bug;