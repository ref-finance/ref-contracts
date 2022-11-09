# FEE in Ref-exchange Contract

## Fee Policy
- Each pool has its independent total fee rate, presented in bps number.
- Total fee is charged on input token (as swap in SimplePool), output token (as swap in stable/rated pool) or liquidity shares (as add/remove liquidity of stable/rated pool).
- Total fee is composed of LP profit and admin fee.
- LP profit would benefit all LP in the pool as it is in the form of in-pool assets that belongs to all LP.
- Admin fee, on the contrary, is in the from of liquidity belongs to exchange(protocol) itself and possible referral of the action.
- Relation between total fee and admin fee, as well as relation between exchange_fee and referral_fee, are described in AdminFees structure.

```rust
pub const FEE_DIVISOR: u32 = 10_000;
/// Maintain information about fees.
pub struct AdminFees {
    /// Basis points of the admin fee in total fee.
    pub admin_fee_bps: u32,
    pub exchange_id: AccountId,
    /// referral_id, referral_fee_bps,
    /// where referral_fee_bps is basis points of the referral fee in admin fee, 
    /// and remaining admin fee belongs to exchange (protocol).
    pub referral_info: Option<(AccountId, u32)>,
}
```
We have:  
`admin_fee = total_fee * admin_fee_bps / FEE_DIVISOR`  
`referral_share = admin_fee_share * referral_info.1 / FEE_DIVISOR`  
`exchange_share = admin_fee_share - referral_share`  


## Referrals

The role of referral can be exist in a swap action, but ref-exchange only allow registered referrals. They are recorded in a map (`referrals: UnorderedMap<AccountId, u32>,`) and managed by smart contract owner (the ref DAO) and guardians.  

Each referral has his own referral_fee_rate recorded as value of the map in bps format. The relationship has been described above.  

*Note: A referral need to register as LP of a pool to receive referral_fee from swap action in that pool.*  
*as we mentioned before, as part of admin fee, it is in the form of liquidity shares of the pool. Otherwise, no referral fee would happen, admin fee as a whole belongs to exchange.*

## SimplePool
### Swap
Given that Alice want swap dX tokenA to get tokenB, then:  
dX is the in-amount of tokenA to keep D unchanged and despite any fees;  
`trading_fee_amount = dX * trade_fee / FEE_DIVISOR`  
Alice actually got `dX - trading_fee_amount` tokenA to participate in swap.  

Then, we convert all admin fee into `admin_fee_share`:  
```bash
# As increasing of invariant is caused by fee, we have:
total_fee_shares : prev_shares = (new_invariant - prev_invariant) : prev_invariant
admin_fee_shares = total_fee_shares * admin_fee_bps / FEE_DIVISOR
```
*Note: Although we calculate total_fee_shares above, but we don't mint share for total fee actually, only admin_fee part would be minted to share.*

And then distribute it between referral and exchange:  

If referral and its account is registered, referral got:  
`referral_share = admin_fee_share * referral_info.1 / FEE_DIVISOR`  

Exchange would got:  
`exchange_share = admin_fee_share - referral_share`  
That is to say, the exchange got all admin fee if referral is invalid.

## Stable/Rated pool
### Swap
Given that Alice want swap dX tokenA to get tokenB, then:  
dY is the out-amount of tokenB to keep D unchanged and despite any fees;  
`trading_fee_amount = dY * trade_fee / FEE_DIVISOR`  
Alice actually got `dY - trading_fee_amount` tokenB.  
We have:  
`admin_fee_amount = trading_fee_amount * admin_fee_bps / FEE_DIVISOR`  

We convert all admin fee into liquidity shares, and then distribute it between referral and exchange:  

If referral and its account is registered, referral got:  
`referral_share = admin_fee_share * referral_info.1 / FEE_DIVISOR`  

Exchange would got:  
`exchange_share = admin_fee_share - referral_share`  
That is to say, the exchange got all admin fee if referral is invalid.


**Remark**  
A portion of TokenB was sustained in pool:  
`tokenB_gap = total_fee_amount - admin_fee_amount`  
This gap actually promotes the unit share value, that is to say, will benefit to all LP of this pool.  


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
We have:  
`admin_shares = share_fee_parts * admin_fee_bps / FEE_DIVISOR`

**referral_fee:**  
`share_mint_for_re = admin_shares * referral_info.1 / FEE_DIVISOR`  

*Note: currently, referral for adding liquidity is not enabled on interface.*

**exchange_fee:**  
`share_mint_for_ex = admin_shares - share_mint_for_re`

**Remark**  
A portion of share does NOT mint:  
`share_gap = share_fee_parts - admin_shares`  
This gap actually promotes the unit share value, that is to say, will benefit to all LP of this pool.

***Note:*** 
* Why charge fee when adding/removing liquidity?
    imbalanced token in/out won't be good for the pool, so need fee;
* Fee algorithm when adding/removing liquidity?  
     Based on the difference between real token in/out and an ideal in/out amount per token. 


### Remove by token amounts 
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
We have:  
`admin_shares = share_fee_parts * admin_fee_bps / FEE_DIVISOR`

**referral_fee:**  
`share_mint_for_re = admin_shares * referral_info.1 / FEE_DIVISOR`  

*Note: Currently, referral for adding liquidity is not enabled on interface.*

**exchange_fee:**  
`share_mint_for_ex = admin_shares - share_mint_for_re`

**Remark**  
A portion of share was over burned:  
`share_gap = share_fee_parts - admin_shares`  
This gap actually promotes the unit share value, that is to say, will benefit to all LP of this pool.

