//! A set implemented on a trie. Unlike `std::collections::HashSet` the elements in this set are not
//! hashed but are instead serialized.
use crate::*;
use std::collections::HashSet;
use std::hash::Hash;
use std::mem::size_of;

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
    /// Returns the number of elements in the set, also referred to as its size.
    pub fn len(&self) -> u64 {
        self.elements.len()
    }

    /// Returns `true` if the set contains no elements.
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    /// Create new map with zero elements. Use `id` as a unique identifier.
    pub fn new(id: Vec<u8>) -> Self {
        let element_index_prefix = append(&id, b'i');
        let elements_prefix = append(&id, b'e');

        Self {
            element_index_prefix,
            elements: Vector::new(elements_prefix),
            data: HashSet::new(),
        }
    }

    pub fn serialize_index(index: u64) -> [u8; size_of::<u64>()] {
        index.to_le_bytes()
    }

    pub fn deserialize_index(raw_index: &[u8]) -> u64 {
        let mut result = [0u8; size_of::<u64>()];
        result.copy_from_slice(raw_index);
        u64::from_le_bytes(result)
    }

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
