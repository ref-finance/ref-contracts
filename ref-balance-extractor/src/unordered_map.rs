//! A map implemented on a trie. Unlike `std::collections::HashMap` the keys in this map are not
//! hashed but are instead serialized.
use crate::*;
use std::hash::Hash;

const ERR_KEY_SERIALIZATION: &str = "Cannot serialize key with Borsh";

/// An iterable implementation of a map that stores its content directly on the trie.
#[derive(BorshSerialize, BorshDeserialize)]
pub struct UnorderedMap<K, V> {
    key_index_prefix: Vec<u8>,
    keys: Vector<K>,
    values: Vector<V>,

    #[borsh_skip]
    pub data: HashMap<K, V>,
}

impl<K, V> UnorderedMap<K, V> {
    fn raw_key_to_index_lookup(&self, raw_key: &[u8]) -> Vec<u8> {
        append_slice(&self.key_index_prefix, raw_key)
    }
}

impl<K, V> UnorderedMap<K, V>
where
    K: BorshSerialize + BorshDeserialize + Clone + std::cmp::Eq + Hash,
    V: BorshSerialize + BorshDeserialize + Clone,
{
    fn serialize_key(key: &K) -> Vec<u8> {
        match key.try_to_vec() {
            Ok(x) => x,
            Err(_) => panic!("{}", ERR_KEY_SERIALIZATION),
        }
    }

    pub fn parse(&mut self, state: &mut State) {
        self.keys.parse(state);
        self.values.parse(state);
        for (key, value) in self.keys.data.iter().zip(self.values.data.iter()) {
            let raw_key = self.raw_key_to_index_lookup(&Self::serialize_key(&key));
            let _index = state.remove(&raw_key).expect("Index expected");
            self.data.insert(key.clone(), value.clone());
        }
    }
}
