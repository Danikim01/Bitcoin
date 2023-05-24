use crate::messages::{HashId, Hashable};
use std::collections::HashMap;
use std::fmt::Display;
use std::io;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use bitcoin_hashes::{sha256, Hash};

pub fn to_io_err<E>(error: E) -> io::Error
where
    E: Display,
{
    io::Error::new(io::ErrorKind::Other, error.to_string())
}

pub fn into_hashmap<T>(elements: Vec<T>) -> HashMap<HashId, T>
where
    T: Hashable,
{
    let hashmap: HashMap<HashId, T> = elements
        .into_iter()
        .map(|element| (element.hash(), element))
        .collect();
    hashmap
}

pub fn actual_timestamp_or_default() -> i64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration,
        Err(..) => Duration::default(),
    }
    .as_secs() as i64
}

pub fn double_hash(bytes: &[u8]) -> sha256::Hash {
    let hash = sha256::Hash::hash(bytes);
    sha256::Hash::hash(&hash[..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_double_hash() {
        let bytes = b"hello world";
        let hash = double_hash(bytes);

        let expected = sha256::Hash::hash(&sha256::Hash::hash(bytes)[..]);
        assert_eq!(hash, expected);
    }
}