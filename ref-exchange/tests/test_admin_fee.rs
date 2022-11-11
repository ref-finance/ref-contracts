use near_sdk::json_types::U128;
use near_sdk::AccountId;
use std::collections::HashMap;
use near_sdk_sim::{
    call, view, to_yocto,
};

use ref_exchange::{PoolInfo, SwapAction};

use crate::common::utils::*;
pub mod common;




#[test]
fn modify_admin_fee() {
    let (root, owner, pool, _, _, _) = setup_pool_with_liquidity();
    // let new_user = root.create_user("new_user".to_string(), to_yocto("100"));

    // pool 0, 10 dai -> 20 eth; pool 1, 20 eth -> 10 usdt

    // make sure the exchange's initial admin fee is 4 & 1 bps
    let metadata = get_metadata(&pool);
    assert_eq!(metadata.exchange_fee, 4);
    assert_eq!(metadata.referral_fee, 1);
    let pool_fee = view!(pool.get_pool_fee(0)).unwrap_json::<u32>();
    assert_eq!(pool_fee, 25);

    // make sure pool info, especially total_fee and share_total_supply
    assert_eq!(
        view!(pool.get_pool(0)).unwrap_json::<PoolInfo>(),
        PoolInfo {
            pool_kind: "SIMPLE_POOL".to_string(),
            amp: 0,
            token_account_ids: vec![dai(), eth()],
            amounts: vec![to_yocto("10").into(), to_yocto("20").into()],
            total_fee: 25,
            shares_total_supply: to_yocto("1").into(),
        }
    );

    // for a new pool, there is no lp token for the exchange
    assert_eq!(
        view!(pool.mft_balance_of(":0".to_string(), pool.valid_account_id()))
            .unwrap_json::<U128>()
            .0,
        to_yocto("0")
    );

    let mut prev_dai = to_yocto("85");
    let mut prev_eth = to_yocto("70");
    let mut prev_usdt = to_yocto("90");

    // swap in 1 dai to get eth
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
    // the exchange got some lp tokens as 4 bps in 25 bps. which is 45457128392697592
    // On v1.7.0, exchange got all admin fee if there is no referral, which is 5 bps in 25 bps, that is 56827868570166683
    assert_eq!(
        view!(pool.mft_balance_of(":0".to_string(), pool.valid_account_id()))
            .unwrap_json::<U128>()
            .0,
        56827868570166683
    );

    // here, we modify admin_fee to more reasonable rate, 1600 bps in 25 bps
    // which is 4 bps (exchange fee) in total, 
    // and 1 bps (referal fee) in total.
    let out_come = call!(
        owner,
        pool.modify_admin_fee(2000),
        deposit = 1
    );
    out_come.assert_success();
    assert_eq!(get_error_count(&out_come), 0);

    // make sure the modification succeed
    let metadata = get_metadata(&pool);
    assert_eq!(metadata.exchange_fee, 2000);
    assert_eq!(metadata.referral_fee, 0);
    let pool_fee = view!(pool.get_pool_fee(0)).unwrap_json::<u32>();
    assert_eq!(pool_fee, 25);

    // swap in 1 usdt to get eth
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
    assert_eq!(balances.get(&eth()).unwrap().0, prev_eth + 1814048647419868151852693);
    prev_usdt -= to_yocto("1");
    prev_eth += 1814048647419868151852693;
    assert_eq!(
        view!(pool.mft_balance_of(":1".to_string(), pool.valid_account_id()))
            .unwrap_json::<U128>()
            .0,
        22731147428066673554
    );

    // here, we remove exchange_fee liquidity
    let balances = view!(pool.get_deposits(owner.valid_account_id()))
        .unwrap_json::<HashMap<AccountId, U128>>();
    assert_eq!(balances.get(&usdt()).unwrap_or(&U128(0)).0, 0);
    assert_eq!(balances.get(&eth()).unwrap_or(&U128(0)).0, 0);
    assert_eq!(balances.get(&dai()).unwrap_or(&U128(0)).0, 0);
    
    // only owner can call, and withdraw liquidity to owner's inner account
    let out_come = call!(
        owner,
        pool.remove_exchange_fee_liquidity(0, U128(56827868570166683), vec![U128(1), U128(1)]),
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
    assert_eq!(balances.get(&eth()).unwrap_or(&U128(0)).0, 1033468794558039923);
    assert_eq!(balances.get(&dai()).unwrap_or(&U128(0)).0, 625106518748362423);

    let out_come = call!(
        owner,
        pool.remove_exchange_fee_liquidity(1, U128(22731147428066673554), vec![U128(1), U128(1)]),
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
    assert_eq!(balances.get(&usdt()).unwrap_or(&U128(0)).0, 250036938082231399513);
    assert_eq!(balances.get(&eth()).unwrap_or(&U128(0)).0, 414411613550153567028);
    assert_eq!(balances.get(&dai()).unwrap_or(&U128(0)).0, 625106518748362423);

    assert_eq!(prev_dai, to_yocto("84"));
    assert_eq!(prev_eth, 73628097294839736303705386);
    assert_eq!(prev_usdt, to_yocto("89"));
}


#[test]
fn referral_fee() {
    // pool 0, 10 dai -> 20 eth; pool 1, 20 eth -> 10 usdt
    let (root, owner, pool, _, _, _) = setup_pool_with_liquidity();
    let referral1 = root.create_user("referral1".to_string(), to_yocto("100"));
    // let new_user = root.create_user("new_user".to_string(), to_yocto("100"));

    call!(
        owner,
        pool.modify_admin_fee(2000),
        deposit=1
    ).assert_success();

    call!(
        owner,
        pool.insert_referral(referral1.valid_account_id(), 2000),
        deposit=1
    ).assert_success();

    assert!(!mft_has_registered(&pool, ":1", referral1.valid_account_id()));

    call!(
        referral1,
        pool.mft_register(":1".to_string(), referral1.valid_account_id()),
        deposit=to_yocto("1")
    ).assert_success();

    assert!(mft_has_registered(&pool, ":1", referral1.valid_account_id()));

    // swap in 1 usdt to get eth
    let out_come = call!(
        root,
        pool.swap(
            vec![SwapAction {
                pool_id: 1,
                token_in: usdt(),
                amount_in: Some(U128(to_yocto("1"))),
                token_out: eth(),
                min_amount_out: U128(1)
            }],
            Some(referral1.valid_account_id())
        ),
        deposit = 1
    );
    out_come.assert_success();
    println!("{:#?}", get_logs(&out_come));
    // "Swapped 1000000000000000000000000 usdt for 1814048647419868151852693 eth002",
    // "Exchange swap got 18184917942453338844 shares, Referral referral1 got 4546229485613334710 shares",
    assert_eq!(mft_balance_of(&pool, ":1", &pool.account_id()), 18184917942453338844);
    assert_eq!(mft_balance_of(&pool, ":1", &referral1.account_id()), 4546229485613334710);
}