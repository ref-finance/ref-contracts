use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{near_bindgen, Balance, PanicOnDefault};
use near_sdk::json_types::U128;

#[near_bindgen]
#[derive(BorshSerialize, BorshDeserialize, PanicOnDefault)]
pub struct Contract {
    price: Balance
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new(price: U128) -> Self {
        Self {
            price: price.0
        }
    }

    pub fn set_price(&mut self, price: U128){
        self.price = price.0;
    }

    pub fn get_st_near_price(&self) -> U128 {
        U128(self.price)
    }

    pub fn ft_price(&self) -> U128 {
        U128(self.price)
    }
}

#[cfg(test)]
mod tests {
    use near_sdk::test_utils::{VMContextBuilder};
    use near_sdk::{testing_env, MockedBlockchain};

    use super::*;

    #[test]
    fn test_basics() {
        let context = VMContextBuilder::new();
        testing_env!(context.build());
        let mut contract = Contract::new(U128(10u128.pow(24 as u32)));

        assert_eq!(contract.ft_price().0, 10u128.pow(24 as u32));
        assert_eq!(contract.get_st_near_price().0, 10u128.pow(24 as u32));

        contract.set_price(U128(2 * 10u128.pow(24 as u32)));

        assert_eq!(contract.ft_price().0, 2 * 10u128.pow(24 as u32));
        assert_eq!(contract.get_st_near_price().0, 2 * 10u128.pow(24 as u32));
    }
}