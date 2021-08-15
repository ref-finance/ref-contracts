//! A persistent map without iterators. Unlike `near_sdk::collections::UnorderedMap` this map
//! doesn't store keys and values separately in vectors, so it can't iterate over keys. But it
//! makes this map more efficient in the number of reads and writes.
use crate::*;
use std::collections::HashMap;
use std::hash::Hash;

const ERR_VALUE_DESERIALIZATION: &str = "Cannot deserialize value with Borsh";

/// An non-iterable implementation of a map that stores its content directly on the trie.
#[derive(BorshSerialize, BorshDeserialize)]
pub struct LookupMap<K, V> {
    pub key_prefix: Vec<u8>,

    #[borsh_skip]
    pub data: HashMap<K, V>,
}

impl<K, V> LookupMap<K, V>
where
    K: BorshSerialize + BorshDeserialize + std::cmp::Eq + Hash,
    V: BorshSerialize + BorshDeserialize,
{
    fn deserialize_value(raw_value: &[u8]) -> V {
        match V::try_from_slice(&raw_value) {
            Ok(x) => x,
            Err(_) => panic!("{}", ERR_VALUE_DESERIALIZATION),
        }
    }

    pub fn parse(&mut self, state: &mut State) {
        let mut keys = vec![];
        let kp = &self.key_prefix;
        let len = kp.len();
        for (k, v) in state.range(kp.clone()..) {
            if k.len() < len || &k[..len] != &kp[..] {
                break;
            }
            let raw_key = &k[len..];
            // NOTE: Since the shares contains conflicting prefixes we can't always deserialize the key, so we should just skip it.
            let key = match K::try_from_slice(&raw_key) {
                Ok(key) => key,
                Err(_err) => {
                    // skipping
                    continue;
                }
            };
            let value: V = Self::deserialize_value(v);
            self.data.insert(key, value);
            keys.push(k.clone());
        }
        for k in keys {
            state.remove(&k);
        }
    }
}
