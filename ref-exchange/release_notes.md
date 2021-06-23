# Release Notes

### Version 0.2.1
---
1. Support for direct swap  
Allows to swap with a single transaction without needing to deposit / withdraw. Not even storage deposits are required for the pool, if force=1 is passed (but FE must make sure that receiver is registered in the outgoing token).  
Example usage: 
    ```bash
    contract.ft_transfer_call(
        to_va(swap()),
        to_yocto("1").into(),
        None,
        "{{\"force\": 0, \"actions\": [{{\"pool_id\": 0, \"token_in\": \"dai\", \"token_out\": \"eth\", \"min_amount_out\": \"1\"}}]}}".to_string()
    ),
    ```  
    Specifically for TokenReceiverMessage message parameters are:  
    ```rust
    enum TokenReceiverMessage {
        /// Alternative to deposit + execute actions call.
        Execute {
            referral_id: Option<ValidAccountId>,
            /// If force != 0, doesn't require user to even have account. In case of failure to deposit to the user's outgoing balance, tokens will be returned to the exchange and can be "saved" via governance.
            /// If force == 0, the account for this user still have been registered. If deposit of outgoing tokens will fail, it will deposit it back into the account.
            force: u8,
            /// List of sequential actions.
            actions: Vec<Action>,
        },
    }
    ```
    where Action is either SwapAction or any future action added there.

2. Allow function access key to trade if all tokens are whitelisted  
There are two changes:  
    * register / unregister tokens for the user requires a 1 yocto Near deposit to prevent access keys whitelisted tokens.  
    * `swap` function supports 0 attached deposit, but all tokens must be already registered or globally whitelisted.  


