# Release Notes

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