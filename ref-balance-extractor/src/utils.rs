pub(crate) fn append_slice(id: &[u8], extra: &[u8]) -> Vec<u8> {
    [id, extra].concat()
}

uint::construct_uint! {
    /// 256-bit unsigned integer.
    pub struct U256(4);
}
