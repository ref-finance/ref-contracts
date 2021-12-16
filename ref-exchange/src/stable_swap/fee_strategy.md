# FEE Strategy in REF Stable Swap Pool
## Fee Structure

```rust
// fee rate in bps
pub struct StableSwapPool {
    // ... ...
    
    // total fee rate
    pub total_fee: u32,

    // ... ...
}

// used in math of stable swap
pub struct Fees {
    pub trade_fee: u32, // equal to total_fee above
    pub admin_fee: u32, // based on trade_fee amount.
}

/// details of admin_fee,
/// admin_fee = exchange_fee + referral_fee
pub struct AdminFees {
    /// Basis points of the fee for exchange.
    pub exchange_fee: u32,
    /// Basis points of the fee for referrer.
    pub referral_fee: u32,
    pub exchange_id: AccountId,
    pub referral_id: Option<AccountId>,
}
```

## Actions Involved
### Add liquidity
* D0 = original invariant D;  
* D1 = D after adding deposit tokens;  
* D2 = D after adding deposit tokens (subtract with fee per token);  

Relations: D1 >= D2 >= D0;  

```bash
share_increased = share_supply * (D1-D0)/D0;
share_mint_for_user = share_supply * (D2-D0)/D0;
share_fee_parts = share_increased - share_mint_for_user;
```
**exchange_fee:**  
```bash
share_mint_for_ex = share_fee_parts * exchange_fee / FEE_DIVISOR;
```
**referral_fee:**  
```bash
share_mint_for_re = share_fee_parts * referral_fee / FEE_DIVISOR;
```

**Remark**  
A portion of share does NOT mint:
```bash
share_gap = share_fee_parts - share_mint_for_ex - share_mint_for_re
```
This gap actually promotes the unit share value, that is to say, will benefit to all LP of this pool.

***Note:*** 
* Why charge fee when adding/removing liquidity?
    imbalanced token in/out won't be good for the pool, so need fee;
* Fee algorithm when adding/removing liquidity?  
     Based on the difference between real token in/out and an ideal in/out amount per token. 

### Remove liquidity
**Remove by share won't involve any fee;**  
*Cause there is no difference with ideal token amount*  

**Remove by token amounts**  
Same as add liquidity,  
* D0 = original invariant D;  
* D1 = D after remove tokens;  
* D2 = D after remove tokens (subtract with fee per token);  

Relations: D0 >= D1 >= D2;

```bash
share_decreased = share_supply * (D0-D1)/D0;
share_burn_for_user = share_supply * (D0-D2)/D0;
share_fee_parts = share_burn_for_user - share_decreased;
```
**exchange_fee:**  
```bash
share_mint_for_ex = share_fee_parts * exchange_fee / FEE_DIVISOR;
```
**referral_fee:**  
```bash
share_mint_for_re = share_fee_parts * referral_fee / FEE_DIVISOR;
```

**Remark**  
A portion of share was over burned:
```bash
share_gap = share_fee_parts - share_mint_for_ex - share_mint_for_re
```
This gap actually promotes the unit share value, that is to say, will benefit to all LP of this pool.

## Swap
Given that Alice want swap dX tokenA to get tokenB, then:  
dY is the out-amount of tokenB to keep D unchanged and despite any fees;  
`trading_fee_amount = dY * trade_fee / FEE_DIVISOR`  
Alice actually got `dY - trading_fee_amount` tokenB.  
We have:  
`admin_fee_amount = trading_fee_amount * admin_fee / FEE_DIVISOR`  

If referral and its account is valid, referral got:  
`referral_tokenB = admin_fee_amount * referral_fee / (referral_fee + exchange_fee)`  

Exchange would got:  
`exchange_tokenB = admin_fee_amount - referral_tokenB`  
That is to say, the exchange got all admin fee if referral is invalid.

Both referral and exchange pour their tokenB back to pool as an adding liquidity process with 0 fee. That is the way they got their fee incoming as shares.

**Remark**  
A portion of TokenB was sustained in pool:
```bash
tokenB_gap = trading_fee_amount - admin_fee_amount
```
This gap actually promotes the unit share value, that is to say, will benefit to all LP of this pool.


