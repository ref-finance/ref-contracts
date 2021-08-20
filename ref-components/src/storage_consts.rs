use near_sdk::StorageUsage;

/// Max account length is 64 + 4 bytes for serialization.
pub const MAX_ACCOUNT_ID_BYTES: StorageUsage = 68;

pub const U128_BYTES: StorageUsage = 16;

pub const U64_BYTES: StorageUsage = 8;

pub const U32_BYTES: StorageUsage = 4;

// struct X<T> {
//     x: PhantomData<T>,
// }
//
// impl<T> X<T> {
//     fn bytes(value: &T) -> StorageUsage {
//         unimplemented!()
//     }
// }
//
// impl<u128> X<u128> {
//     fn bytes(value: &T) -> StorageUsage {
//         unimplemented!()
//     }
// }

// pub fn bytes<T>(value: &T) -> StorageUsage {
//     unimplemented!()
// }
//
// pub fn bytes(value: &u128) -> StorageUsage {
//     MAX_ACCOUNT_ID_BYTES
// }
//
// pub fn hashmap_bytes<K, V>(data: &HashMap<K, V>) -> StorageUsage {
//     U32_BYTES + data.len() * (bytes(K) + bytes(V))
// }
