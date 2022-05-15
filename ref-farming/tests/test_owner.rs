use near_sdk_sim::{call, init_simulator, to_yocto};
use near_sdk::json_types::{U128};
use ref_farming::{HRSimpleFarmTerms};

use crate::common::utils::*;
use crate::common::init::deploy_farming;
use crate::common::views::*;

mod common;

#[test]
fn owner_interfaces() {
    let root = init_simulator(None);
    let owner = root.create_user("owner".to_string(), to_yocto("100"));
    let farming = deploy_farming(&root, farming_id(), owner.account_id());

    // create farm
    let out_come = call!(
        owner,
        farming.create_simple_farm(HRSimpleFarmTerms{
            seed_id: "swap@0".to_string(),
            reward_token: owner.valid_account_id(),
            start_at: 0,
            reward_per_session: to_yocto("1").into(),
            session_interval: 60,
        }, None),
        deposit = to_yocto("1")
    );
    out_come.assert_success();
    let seeds = show_seedsinfo(&farming, false);
    assert_eq!(seeds.get("swap@0").unwrap().min_deposit.0, 1000000000000000000);

    let out_come = call!(
        owner,
        farming.modify_seed_min_deposit("swap@0".to_string(), U128(1000000000000000)),
        deposit = 0
    );
    out_come.assert_success();

    let seeds = show_seedsinfo(&farming, false);
    assert_eq!(seeds.get("swap@0").unwrap().min_deposit.0, 1000000000000000);
}