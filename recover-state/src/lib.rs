use near_sdk::base64;
use near_sdk::{env, near_bindgen};

#[allow(dead_code)]
#[near_bindgen]
struct Contract {}

#[no_mangle]
pub extern "C" fn restore() {
    env::setup_panic_hook();
    env::set_blockchain_interface(Box::new(near_blockchain::NearBlockchain {}));

    let values: Vec<(&str, &str)> = vec![("AAA=", "A==")];
    for (key, value) in values {
        env::storage_write(
            &base64::decode(key).unwrap(),
            &base64::decode(value).unwrap(),
        );
        env::storage_get_evicted().map(|old| {
            env::log(
                format!("Key {} replace {} with {}", key, base64::encode(old), value).as_bytes(),
            );
        });
    }
}
