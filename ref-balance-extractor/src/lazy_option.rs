use crate::*;

const ERR_VALUE_DESERIALIZATION: &str = "Cannot deserialize value with Borsh";

/// An persistent lazy option, that stores a value in the storage.
#[derive(BorshSerialize, BorshDeserialize)]
pub struct LazyOption<T> {
    storage_key: Vec<u8>,

    #[borsh_skip]
    data: Option<T>,
}

impl<T> LazyOption<T>
where
    T: BorshSerialize + BorshDeserialize,
{
    fn deserialize_value(raw_value: &[u8]) -> T {
        match T::try_from_slice(&raw_value) {
            Ok(x) => x,
            Err(_) => panic!("{}", ERR_VALUE_DESERIALIZATION),
        }
    }

    pub fn parse(&mut self, state: &mut State) {
        if let Some(raw_value) = state.remove(&self.storage_key) {
            self.data = Some(Self::deserialize_value(&raw_value));
        }
    }
}
