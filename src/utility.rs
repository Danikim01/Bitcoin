use bitcoin_hashes::{sha256, Hash};
use std::fmt::Display;
use std::fmt::Write;
use std::io;
use std::num::ParseIntError;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Displays a error
pub fn to_io_err<E>(error: E) -> io::Error
where
    E: Display,
{
    io::Error::new(io::ErrorKind::Other, error.to_string())
}

/// Get the actual timestamp or default
pub fn actual_timestamp_or_default() -> i64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration,
        Err(..) => Duration::default(),
    }
    .as_secs() as i64
}

/// Aplies two times the sha256 hash function to a byte array
pub fn double_hash(bytes: &[u8]) -> sha256::Hash {
    let hash = sha256::Hash::hash(bytes);
    sha256::Hash::hash(&hash[..])
}

pub fn reverse_hex_str(hex: &str) -> String {
    let mut reversed_hex = String::new();
    let chars = hex.chars().collect::<Vec<char>>();
    for i in (0..chars.len()).step_by(2) {
        let mut byte = String::new();
        byte.push(chars[i]);
        byte.push(chars[i + 1]);
        reversed_hex = format!("{}{}", byte, reversed_hex);
    }
    reversed_hex
}

pub fn decode_hex(s: &str) -> Result<Vec<u8>, ParseIntError> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
        .collect()
}

pub fn _encode_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        write!(&mut s, "{:02x}", b).unwrap();
    }
    s
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
