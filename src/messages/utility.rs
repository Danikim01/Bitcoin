use std::io;
use std::io::{Cursor, Read};

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
            buf.extend_from_slice(&(value as u64).to_le_bytes());
        }
    }
    buf
}

pub fn read_i32(cursor: &mut Cursor<&[u8]>) -> Result<i32, io::Error> {
    let mut buf = [0u8; 4];
    cursor.read_exact(&mut buf)?;
    Ok(i32::from_le_bytes(buf))
}

pub fn read_u32(cursor: &mut Cursor<&[u8]>) -> Result<u32, io::Error> {
    let mut buf = [0u8; 4];
    cursor.read_exact(&mut buf)?;
    Ok(u32::from_le_bytes(buf))
}

pub fn read_u8(cursor: &mut Cursor<&[u8]>) -> Result<u8, io::Error> {
    let mut buf = [0u8; 1];
    cursor.read_exact(&mut buf)?;
    Ok(u8::from_le_bytes(buf))
}

pub fn read_hash(cursor: &mut Cursor<&[u8]>) -> Result<[u8; 32], io::Error> {
    let mut hash = [0u8; 32];
    cursor.read_exact(&mut hash)?;
    Ok(hash)
}

pub fn read_from_varint(cursor: &mut Cursor<&[u8]>) -> Result<u64, io::Error> {
    let first_byte = read_u8(cursor)?;
    match first_byte {
        0xff => {
            let mut buf = [0u8; 8];
            cursor.read_exact(&mut buf)?;
            let value = u64::from_le_bytes(buf);
            Ok(value as u64)
        }
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