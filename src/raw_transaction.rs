use crate::io::Cursor;
use crate::messages::utility::*;
use std::io::ErrorKind;
use std::io::Read;
use std::vec;

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
        let index = u32::from_le_stream(cursor)?;
        println!("the index is {}", &index);
        let script_bytes = read_from_varint(cursor)?;
        println!("the script bytes are {}", &script_bytes);
        let height = read_height(cursor)?;
        println!("the height is {:?}", &height);
        let coinbase_script = read_coinbase_script(cursor, script_bytes as usize)?;
        println!("the coinbase script is {:?}", &coinbase_script);
        let sequence = u32::from_le_stream(cursor)?;
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
        let index = u32::from_le_stream(cursor)?;
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
        let sequence = u32::from_le_stream(cursor)?;

        Ok(TxInput {
            previous_output,
            script_bytes: script_bytes as usize,
            signature_script,
            sequence,
        })
    }

    pub fn vec_from_bytes(
        cursor: &mut Cursor<&[u8]>,
        count: usize,
    ) -> Result<Vec<Self>, std::io::Error> {
        let mut tx_inputs: Vec<Self> = Vec::new();

        for _ in 0..count {
            let tx_input = Self::from_bytes(cursor)?;
            tx_inputs.push(tx_input);
        }

        Ok(tx_inputs)
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
        let value = i64::from_le_stream(cursor)?;
        let pk_script_bytes = read_from_varint(cursor)?;
        let pk_script = read_coinbase_script(cursor, pk_script_bytes as usize)?;

        Ok(TxOutput {
            value,
            pk_script_bytes: pk_script_bytes as usize,
            pk_script,
        })
    }

    pub fn vec_from_bytes(
        cursor: &mut Cursor<&[u8]>,
        count: usize,
    ) -> Result<Vec<Self>, std::io::Error> {
        let mut tx_outputs: Vec<Self> = Vec::new();

        for _ in 0..count {
            let tx_output = Self::from_bytes(cursor)?;
            tx_outputs.push(tx_output);
        }

        Ok(tx_outputs)
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
        let version = i32::from_le_stream(cursor)?;
        let tx_in_count = read_from_varint(cursor)? as usize;
        let tx_in = TxInput::vec_from_bytes(cursor, tx_in_count)?;
        let tx_out_count = read_from_varint(cursor)? as usize;
        let tx_out = TxOutput::vec_from_bytes(cursor, tx_out_count)?;
        let lock_time = u32::from_le_stream(cursor)?;

        let raw_transaction = RawTransaction {
            version,
            tx_in_count,
            tx_in,
            tx_out_count,
            tx_out,
            lock_time,
        };
        Ok(raw_transaction)
    }
}

fn read_coinbase_script(cursor: &mut Cursor<&[u8]>, n: usize) -> Result<Vec<u8>, std::io::Error> {
    let mut array = vec![0_u8; n];
    cursor.read_exact(&mut array)?;
    Ok(array)
}

fn read_height(cursor: &mut Cursor<&[u8]>) -> Result<u32, std::io::Error> {
    let height_bytes = u8::from_le_stream(cursor)?;
    if height_bytes != 0x03 && height_bytes != 0x04 {
        println!("uipsee");
        return Err(std::io::Error::new(
            ErrorKind::Unsupported,
            "Height unsupported",
        ));
    }

    let mut array = [0u8; 4]; // 00[0][1][2] or [0][1][2]00?

    array[0] = u8::from_le_stream(cursor)?;
    array[1] = u8::from_le_stream(cursor)?;
    array[2] = u8::from_le_stream(cursor)?;
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
