use near_contract_standards::fungible_token::{
    FungibleToken, 
    metadata::{
        FungibleTokenMetadata, FungibleTokenMetadataProvider, FT_METADATA_SPEC,
    }
};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{env, log, near_bindgen, Balance, PanicOnDefault, AccountId, PromiseOrValue};
use near_sdk::json_types::{ValidAccountId, U128};

near_sdk::setup_alloc!();

#[near_bindgen]
#[derive(BorshSerialize, BorshDeserialize, PanicOnDefault)]
pub struct Contract {
    price: Balance,
    token: FungibleToken,
    name: String,
    symbol: String,
    icon: Option<String>,
    decimals: u8,
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new(name: String, symbol: String, decimals: u8, price: U128) -> Self {
        Self {
            token: FungibleToken::new(b"t".to_vec()),
            name,
            symbol,
            icon: None,
            decimals,
            price: price.0
        }
    }

    pub fn set_price(&mut self, price: U128){
        self.price = price.0;
        log!("{} set price to {}", env::predecessor_account_id(), price.0);
    }

    pub fn get_st_near_price(&self) -> U128 {
        U128(self.price)
    }

    pub fn ft_price(&self) -> U128 {
        U128(self.price)
    }

    pub fn get_nearx_price(&self) -> U128 {
        U128(self.price)
    }

    pub fn mint(&mut self, account_id: ValidAccountId, amount: U128) {
        self.token
            .internal_deposit(account_id.as_ref(), amount.into());
    }

    pub fn burn(&mut self, account_id: ValidAccountId, amount: U128) {
        self.token
            .internal_withdraw(account_id.as_ref(), amount.into());
    }
}

near_contract_standards::impl_fungible_token_core!(Contract, token);
near_contract_standards::impl_fungible_token_storage!(Contract, token);

#[near_bindgen]
impl FungibleTokenMetadataProvider for Contract {
    fn ft_metadata(&self) -> FungibleTokenMetadata {
        FungibleTokenMetadata {
            spec: FT_METADATA_SPEC.to_string(),
            name: self.name.clone(),
            symbol: self.symbol.clone(),
            icon: self.icon.clone(),
            reference: None,
            reference_hash: None,
            decimals: self.decimals,
        }
    }
}

#[cfg(test)]
mod tests {
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::{env, testing_env, MockedBlockchain};

    use super::*;

    #[test]
    fn test_basics() {
        let mut context = VMContextBuilder::new();
        testing_env!(context.build());
        let mut contract = Contract::new(String::from("TBD"), String::from("TBD"), 24, U128(10u128.pow(24 as u32)));

        testing_env!(context
            .attached_deposit(125 * env::storage_byte_cost())
            .build());
        contract.storage_deposit(Some(accounts(0)), None);
        testing_env!(context
            .attached_deposit(0)
            .predecessor_account_id(accounts(0))
            .build());
        contract.mint(accounts(0), 1_000_000.into());
        assert_eq!(contract.ft_balance_of(accounts(0)), 1_000_000.into());

        testing_env!(context
            .attached_deposit(125 * env::storage_byte_cost())
            .build());
        contract.storage_deposit(Some(accounts(1)), None);
        testing_env!(context
            .attached_deposit(1)
            .predecessor_account_id(accounts(0))
            .build());
        contract.ft_transfer(accounts(1), 1_000.into(), None);
        assert_eq!(contract.ft_balance_of(accounts(1)), 1_000.into());

        contract.burn(accounts(1), 500.into());
        assert_eq!(contract.ft_balance_of(accounts(1)), 500.into());

        assert_eq!(contract.ft_price().0, 10u128.pow(24 as u32));
        assert_eq!(contract.get_st_near_price().0, 10u128.pow(24 as u32));
        assert_eq!(contract.get_nearx_price().0, 10u128.pow(24 as u32));

        contract.set_price(U128(2 * 10u128.pow(24 as u32)));

        assert_eq!(contract.ft_price().0, 2 * 10u128.pow(24 as u32));
        assert_eq!(contract.get_st_near_price().0, 2 * 10u128.pow(24 as u32));
        assert_eq!(contract.get_nearx_price().0, 2 * 10u128.pow(24 as u32));
    }
}