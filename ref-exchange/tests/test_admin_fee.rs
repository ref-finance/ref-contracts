use near_sdk::json_types::U128;
use near_sdk::AccountId;
use std::collections::HashMap;
use near_sdk_sim::{
    call, view, to_yocto, ContractAccount, ExecutionResult, UserAccount,
};

use test_token::ContractContract as TestToken;
use ref_exchange::{ContractContract as Exchange, PoolInfo, SwapAction};

use crate::common::utils::*;
pub mod common;




#[test]
fn modify_admin_fee() {
    let (root, owner, pool, token1, token2, _) = setup_pool_with_liquidity();
    let new_user = root.create_user("new_user".to_string(), to_yocto("100"));

    let metadata = get_metadata(&pool);
    assert_eq!(metadata.exchange_fee, 4);
    assert_eq!(metadata.referral_fee, 1);
    let pool_fee = view!(pool.get_pool_fee(0)).unwrap_json::<u32>();
    assert_eq!(pool_fee, 25);

    assert_eq!(
        view!(pool.get_pool(0)).unwrap_json::<PoolInfo>(),
        PoolInfo {
            pool_kind: "SIMPLE_POOL".to_string(),
            token_account_ids: vec![dai(), eth()],
            amounts: vec![to_yocto("10").into(), to_yocto("20").into()],
            total_fee: 25,
            shares_total_supply: to_yocto("1").into(),
        }
    );
    assert_eq!(
        view!(pool.mft_balance_of(":0".to_string(), pool.valid_account_id()))
            .unwrap_json::<U128>()
            .0,
        to_yocto("0")
    );

    let mut prev_dai = to_yocto("95");
    let mut prev_eth = to_yocto("60");
    let mut prev_usdt = to_yocto("95");

    call!(
        root,
        pool.swap(
            vec![SwapAction {
                pool_id: 0,
                token_in: dai(),
                amount_in: Some(U128(to_yocto("1"))),
                token_out: eth(),
                min_amount_out: U128(1)
            }],
            None
        ),
        deposit = 1
    )
    .assert_success();
    let balances = view!(pool.get_deposits(root.valid_account_id()))
        .unwrap_json::<HashMap<AccountId, U128>>();
    assert_eq!(balances.get(&dai()).unwrap().0, prev_dai - to_yocto("1"));
    assert_eq!(balances.get(&eth()).unwrap().0, prev_eth + 1814048647419868151852693);
    prev_dai -= to_yocto("1");
    prev_eth += 1814048647419868151852693;
    assert_eq!(
        view!(pool.mft_balance_of(":0".to_string(), pool.valid_account_id()))
            .unwrap_json::<U128>()
            .0,
        45457128392697592
    );

    // here, we modify admin_fee to more reasonable rate
    let out_come = call!(
        owner,
        pool.modify_admin_fee(1600, 400)
    );
    out_come.assert_success();
    assert_eq!(get_error_count(&out_come), 0);

    let metadata = get_metadata(&pool);
    assert_eq!(metadata.exchange_fee, 1600);
    assert_eq!(metadata.referral_fee, 400);
    let pool_fee = view!(pool.get_pool_fee(0)).unwrap_json::<u32>();
    assert_eq!(pool_fee, 25);

    call!(
        root,
        pool.swap(
            vec![SwapAction {
                pool_id: 1,
                token_in: usdt(),
                amount_in: Some(U128(to_yocto("1"))),
                token_out: eth(),
                min_amount_out: U128(1)
            }],
            None
        ),
        deposit = 1
    )
    .assert_success();
    let balances = view!(pool.get_deposits(root.valid_account_id()))
        .unwrap_json::<HashMap<AccountId, U128>>();
    assert_eq!(balances.get(&usdt()).unwrap().0, prev_usdt - to_yocto("1"));
    assert_eq!(balances.get(&eth()).unwrap().0, prev_eth + 1870604781997187060478199);
    prev_usdt -= to_yocto("1");
    prev_eth += 1870604781997187060478199;
    assert_eq!(
        view!(pool.mft_balance_of(":1".to_string(), pool.valid_account_id()))
            .unwrap_json::<U128>()
            .0,
        18750732479101047531
    );

    // here, we remove exchange_fee liquidity
    let balances = view!(pool.get_deposits(owner.valid_account_id()))
        .unwrap_json::<HashMap<AccountId, U128>>();
    assert_eq!(balances.get(&usdt()).unwrap_or(&U128(0)).0, 0);
    assert_eq!(balances.get(&eth()).unwrap_or(&U128(0)).0, 0);
    assert_eq!(balances.get(&dai()).unwrap_or(&U128(0)).0, 0);
    
    let out_come = call!(
        owner,
        pool.remove_exchange_fee_liquidity(0, U128(45457128392697592), vec![U128(1), U128(1)]),
        deposit = 1
    );
    out_come.assert_success();
    assert_eq!(get_error_count(&out_come), 0);
    assert_eq!(
        view!(pool.mft_balance_of(":0".to_string(), pool.valid_account_id()))
            .unwrap_json::<U128>()
            .0,
        0
    );
    let balances = view!(pool.get_deposits(owner.valid_account_id()))
        .unwrap_json::<HashMap<AccountId, U128>>();
    assert_eq!(balances.get(&usdt()).unwrap_or(&U128(0)).0, 0);
    assert_eq!(balances.get(&eth()).unwrap_or(&U128(0)).0, 826681087999039131);
    assert_eq!(balances.get(&dai()).unwrap_or(&U128(0)).0, 500028389589818806);

    let out_come = call!(
        owner,
        pool.remove_exchange_fee_liquidity(1, U128(18750732479101047531), vec![U128(1), U128(1)]),
        deposit = 1
    );
    out_come.assert_success();
    assert_eq!(get_error_count(&out_come), 0);
    assert_eq!(
        view!(pool.mft_balance_of(":0".to_string(), pool.valid_account_id()))
            .unwrap_json::<U128>()
            .0,
        0
    );
    let balances = view!(pool.get_deposits(owner.valid_account_id()))
        .unwrap_json::<HashMap<AccountId, U128>>();
    assert_eq!(balances.get(&usdt()).unwrap_or(&U128(0)).0, 200005312946332869948);
    assert_eq!(balances.get(&eth()).unwrap_or(&U128(0)).0, 352453461935982284469);
    assert_eq!(balances.get(&dai()).unwrap_or(&U128(0)).0, 500028389589818806);
}