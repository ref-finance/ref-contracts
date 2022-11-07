# Ref Finance Features Introduction

## FronzenList
Frozenlist is for freezing token related actions on ref-exchange contract. In another word, Any actions (including swap, add/remove liquidity, deposit/withdraw token) that has tokens in frozenlist involved, would panic at contract level.  

The fronzen list is managed by owner and guardians of the ref-exchange contract, through the following 2 interfaces:  
```rust=

```