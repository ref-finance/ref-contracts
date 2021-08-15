pub(crate) fn append(id: &[u8], chr: u8) -> Vec<u8> {
    append_slice(id, &[chr])
}

pub(crate) fn append_slice(id: &[u8], extra: &[u8]) -> Vec<u8> {
    [id, extra].concat()
}
