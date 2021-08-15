//! A set implemented on a trie. Unlike `std::collections::HashSet` the elements in this set are not
//! hashed but are instead serialized.
use crate::*;
use std::collections::HashSet;
use std::hash::Hash;

const ERR_ELEMENT_SERIALIZATION: &str = "Cannot serialize element with Borsh";

/// An iterable implementation of a set that stores its content directly on the trie.
#[derive(BorshSerialize, BorshDeserialize)]
pub struct UnorderedSet<T> {
    pub element_index_prefix: Vec<u8>,
    pub elements: Vector<T>,

    #[borsh_skip]
    pub data: HashSet<T>,
}

impl<T> UnorderedSet<T> {
    pub fn raw_element_to_index_lookup(&self, element_raw: &[u8]) -> Vec<u8> {
        append_slice(&self.element_index_prefix, element_raw)
    }
}

impl<T> UnorderedSet<T>
where
    T: BorshSerialize + BorshDeserialize + Clone + std::cmp::Eq + Hash,
{
    fn serialize_element(element: &T) -> Vec<u8> {
        match element.try_to_vec() {
            Ok(x) => x,
            Err(_) => panic!("{}", ERR_ELEMENT_SERIALIZATION),
        }
    }

    pub fn parse(&mut self, state: &mut State) {
        self.elements.parse(state);
        for element in self.elements.data.iter() {
            let raw_key = self.raw_element_to_index_lookup(&Self::serialize_element(&element));
            let _index = state.remove(&raw_key).expect("Index expected");
            self.data.insert(element.clone());
        }
    }
}
