# Ref Finance Features Introduction

## FronzenList
Activated on Version 1.6.0+  
Frozenlist is for freezing token related actions on ref-exchange contract. In another word, Any actions (including swap, add/remove liquidity, deposit/withdraw token) that has tokens in frozenlist involved, would panic at contract level.  

The fronzen list is managed by owner and guardians of the ref-exchange contract, through the following 2 interfaces:  
```bash=
near call $REF_EX extend_frozenlist_tokens '{"tokens": ["'$TOKEN1'", "'$TOKEN2'"]}' --account_id=$REF_GUARDIAN --depositYocto=1

near call $REF_EX remove_frozenlist_tokens '{"tokens": ["'$TOKEN1'", "'$TOKEN2'"]}' --account_id=$REF_GUARDIAN --depositYocto=1
```
And it can be queried publicly:
```bash=
near view $REF_EX get_frozenlist_tokens
```