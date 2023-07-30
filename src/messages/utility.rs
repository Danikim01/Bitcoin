use super::HashId;
use chrono::{DateTime, NaiveDateTime, Utc};
use std::io;
use std::io::{Cursor, Read};

/// Convert a `u64` to a `Vec<u8>` with the varint format (https://btcinformation.org/en/developer-reference#compactsize-unsigned-integers)
//ver: https://btcinformation.org/en/developer-reference#compactsize-unsigned-integers
pub fn to_varint(value: u64) -> Vec<u8> {
    let mut buf = Vec::new();
    match value {
        0..=252 => {
            buf.push(value as u8);
        }
        253..=0xffff => {
            buf.push(0xfd);
            buf.extend_from_slice(&(value as u16).to_le_bytes());
        }
        0x10000..=0xffffffff => {
            buf.push(0xfe);
            buf.extend_from_slice(&(value as u32).to_le_bytes());
        }
        _ => {
            buf.push(0xff);
            buf.extend_from_slice(&value.to_le_bytes());
        }
    }
    buf
}

pub trait StreamRead {
    fn from_le_stream(cursor: &mut Cursor<&[u8]>) -> Result<Self, io::Error>
    where
        Self: Sized;
    fn from_be_stream(cursor: &mut Cursor<&[u8]>) -> Result<Self, io::Error>
    where
        Self: Sized;
}

/// Implement StreamRead for all the integer types in order to read them from a stream
// source: https://www.reddit.com/r/rust/comments/g0inzh/is_there_a_trait_for_from_le_bytes_from_be_bytes/
macro_rules! impl_StreamRead_for_ints (( $($int:ident),* ) => {
    $(
        impl StreamRead for $int {
            fn from_le_stream(cursor: &mut Cursor<&[u8]>) -> Result<Self, io::Error> {
                let mut buf = [0u8; std::mem::size_of::<Self>()];
                cursor.read_exact(&mut buf)?;
                Ok(Self::from_le_bytes(buf))
            }
            fn from_be_stream(cursor: &mut Cursor<&[u8]>) -> Result<Self, io::Error> {
                let mut buf = [0u8; std::mem::size_of::<Self>()];
                cursor.read_exact(&mut buf)?;
                Ok(Self::from_be_bytes(buf))
            }
        }
    )*
});

impl_StreamRead_for_ints!(u8, u16, u32, u64, i32, i64, u128);

pub fn read_hash(cursor: &mut Cursor<&[u8]>) -> io::Result<HashId> {
    let mut hash = [0u8; 32];
    cursor.read_exact(&mut hash)?;
    Ok(HashId::new(hash))
}

pub fn date_from_timestamp(timestamp: u32) -> String {
    let naive: NaiveDateTime = match NaiveDateTime::from_timestamp_opt(timestamp as i64, 0) {
        Some(naive) => naive,
        None => Utc::now().naive_utc(),
    };
    let datetime: DateTime<Utc> = DateTime::from_utc(naive, Utc);
    datetime.format("%Y-%m-%d %H:%M:%S").to_string()
}

pub fn read_from_varint(cursor: &mut Cursor<&[u8]>) -> io::Result<u64> {
    let first_byte = u8::from_le_stream(cursor)?;

    match first_byte {
        0xff => Ok(u64::from_le_stream(cursor)?),
        0xfe => {
            let mut buf = [0u8; 4];
            cursor.read_exact(&mut buf)?;
            let value = u32::from_le_bytes(buf);
            Ok(value as u64)
        }
        0xfd => {
            let mut buf = [0u8; 2];
            cursor.read_exact(&mut buf)?;
            let value = u16::from_le_bytes(buf);
            Ok(value as u64)
        }
        _ => Ok(first_byte as u64),
    }
}

// https://developer.bitcoin.org/reference/transactions.html#compact_size-unsigned-integers
pub fn to_compact_size_bytes(compact_size: u64) -> Vec<u8> {
    let mut bytes: Vec<u8> = vec![];
    if compact_size <= 0xfc {
        bytes.extend(compact_size.to_le_bytes()[..1].iter());
    } else if compact_size <= 0xffff {
        bytes.push(0xfd);
        bytes.extend(compact_size.to_le_bytes()[..2].iter());
    } else if compact_size <= 0xffffffff {
        bytes.push(0xfe);
        bytes.extend(compact_size.to_le_bytes()[..4].iter());
    } else {
        bytes.push(0xff);
        bytes.extend(compact_size.to_le_bytes()[..8].iter());
    }

    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_varint() {
        assert_eq!(to_varint(0), vec![0]);
        assert_eq!(to_varint(100), vec![100]);
        assert_eq!(to_varint(500), vec![0xfd, 0xf4, 0x01]);
        assert_eq!(to_varint(1000000), vec![0xfe, 0x40, 0x42, 0x0f, 0x00]);
    }

    #[test]
    fn test_endian_read() {
        let bytes = [0x01, 0x02, 0x03, 0x04];
        let slice: &[u8] = &bytes;
        let mut cursor = Cursor::new(slice);
        let value = u32::from_le_stream(&mut cursor).unwrap();
        assert_eq!(value, 67305985);

        let _bytes = [0x04, 0x03, 0x02, 0x01];
        let _slice: &[u8] = &_bytes;
        let mut cursor = Cursor::new(_slice);
        let value = u32::from_be_stream(&mut cursor).unwrap();
        assert_eq!(value, 67305985);
    }

    #[test]
    fn test_read_hash() {
        let data: [u8; 32] = [
            0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, 0xFE, 0xDC, 0xBA, 0x98, 0x76, 0x54,
            0x32, 0x10, 0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, 0xFE, 0xDC, 0xBA, 0x98,
            0x76, 0x54, 0x32, 0x10,
        ];
        let slice: &[u8] = &data;

        let mut cursor = Cursor::new(slice);
        let result = read_hash(&mut cursor);

        assert_eq!(result.is_ok(), true);
        assert_eq!(result.unwrap(), HashId::new(data));
    }
}
