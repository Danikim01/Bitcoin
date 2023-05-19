use crate::io::Cursor;
use crate::messages::utility::*;
use std::io::ErrorKind;
use std::io::Read;

#[derive(Debug)]
pub struct CoinBaseInput {
    hash: [u8; 32],
    index: u32,
    script_bytes: u64,
    height: u32,
    coinbase_script: Vec<u8>,
    sequence: u32,
}

impl CoinBaseInput {
    pub fn new(
        hash: [u8; 32],
        index: u32,
        script_bytes: u64,
        height: u32,
        coinbase_script: Vec<u8>,
        sequence: u32,
    ) -> Self {
        Self {
            hash,
            index,
            script_bytes,
            height,
            coinbase_script,
            sequence,
        }
    }

    pub fn from_bytes(cursor: &mut Cursor<&[u8]>) -> Result<CoinBaseInput, std::io::Error> {
        let hash = read_hash(cursor)?;
        println!("the hash is {:?}", &hash);
        let index = read_u32(cursor)?;
        println!("the index is {}", &index);
        let script_bytes = read_from_varint(cursor)?;
        println!("the script bytes are {}", &script_bytes);
        let height = read_height(cursor)?;
        println!("the height is {:?}", &height);
        let coinbase_script = read_coinbase_script(cursor, script_bytes as usize)?;
        println!("the coinbase script is {:?}", &coinbase_script);
        let sequence = read_u32(cursor)?;
        println!("the sequence is {}", &sequence);
        Ok(CoinBaseInput::new(
            hash,
            index,
            script_bytes,
            height,
            coinbase_script,
            sequence,
        ))
    }
}

#[derive(Debug)]
pub struct Outpoint {
    pub hash: [u8; 32],
    pub index: u32,
}

impl Outpoint {
    pub fn from_bytes(cursor: &mut Cursor<&[u8]>) -> Result<Self, std::io::Error> {
        let hash = read_hash(cursor)?;
        let index = read_u32(cursor)?;
        Ok(Outpoint { hash, index })
    }
}

#[derive(Debug)]
pub struct TxInput {
    pub previous_output: Outpoint,
    pub script_bytes: usize,
    pub signature_script: Vec<u8>,
    pub sequence: u32,
}

impl TxInput {
    pub fn from_bytes(cursor: &mut Cursor<&[u8]>) -> Result<Self, std::io::Error> {
        let previous_output = Outpoint::from_bytes(cursor)?;
        let script_bytes = read_from_varint(cursor)?;
        let signature_script = read_coinbase_script(cursor, script_bytes as usize)?;
        let sequence = read_u32(cursor)?;
        
        Ok(TxInput {
            previous_output,
            script_bytes: script_bytes as usize,
            signature_script,
            sequence,
        })
    }
}

#[derive(Debug)]
pub struct TxOutput {
    pub value: i64,
    pub pk_script_bytes: usize,
    pub pk_script: Vec<u8>,
}

impl TxOutput {
    pub fn from_bytes(cursor: &mut Cursor<&[u8]>) -> Result<Self, std::io::Error> {
        let value = read_i64(cursor)?;
        let pk_script_bytes = read_from_varint(cursor)?;
        let pk_script = read_coinbase_script(cursor, pk_script_bytes as usize)?;

        Ok(TxOutput {
            value,
            pk_script_bytes: pk_script_bytes as usize,
            pk_script,
        })
    }
}

#[derive(Debug)]
pub struct RawTransaction {
    pub version: i32, //Puede ser 1 o 2
    pub tx_in_count: usize,
    pub tx_in: Vec<TxInput>,
    pub tx_out_count: usize,
    pub tx_out: Vec<TxOutput>,
    pub lock_time: u32,
}

//Ver https://developer.bitcoin.org/reference/transactions.html#raw-transaction-format
impl RawTransaction {
    pub fn from_bytes(cursor: &mut Cursor<&[u8]>) -> Result<Self, std::io::Error> {
        // read version - 4 bytes
        let version = read_i32(cursor)?;

        // read tx_in_count - compactSize uint
        let tx_in_count = read_from_varint(cursor)? as usize;

        // read tx_in - vec of txIn
        let tx_in = TxInput::from_bytes(cursor)?;

        // read tx_out_count - compactSize uint
        let tx_out_count = read_from_varint(cursor)? as usize;

        // read tx_out - vec of txOut
        let tx_out = TxOutput::from_bytes(cursor)?;

        // read lock_time - 4 bytes
        let lock_time = read_u32(cursor)?;

        let raw_transaction = RawTransaction {
            version,
            tx_in_count,
            tx_in: vec![], // should be a vector
            tx_out_count,
            tx_out: vec![], // should be a vector
            lock_time,
        };
        Ok(raw_transaction) // should return self
    }
}

fn read_coinbase_script(cursor: &mut Cursor<&[u8]>, n: usize) -> Result<Vec<u8>, std::io::Error> {
    let mut array = vec![0_u8; n];
    cursor.read_exact(&mut array);
    Ok(array)
}

fn read_height(cursor: &mut Cursor<&[u8]>) -> Result<u32, std::io::Error> {
    let height_bytes = read_u8(cursor)?;
    if height_bytes != 0x03 && height_bytes != 0x04 {
        println!("uipsee");
        return Err(std::io::Error::new(
            ErrorKind::Unsupported,
            "Height unsupported",
        ));
    }

    let mut array = [0u8; 4]; // 00[0][1][2] or [0][1][2]00?

    array[0] = read_u8(cursor)?;
    array[1] = read_u8(cursor)?;
    array[2] = read_u8(cursor)?;
    Ok(u32::from_le_bytes(array))
}

// tests
#[cfg(test)]
mod tests {

    #[test]
    fn foo_tests() {
        assert_eq!(1, 1);
    }
}