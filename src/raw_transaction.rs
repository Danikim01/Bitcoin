use crate::io::Cursor;
use crate::messages::utility::*;
use std::io::{Error, Read};
use crate::raw_transaction::TxInputType::TxInput as OtherTxInput;

fn read_coinbase_script(cursor: &mut Cursor<&[u8]>, count: usize) -> Result<Vec<u8>, std::io::Error> {
    let mut array = vec![0_u8; count];
    cursor.read_exact(&mut array)?;
    Ok(array)
}

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
    pub fn from_bytes(cursor: &mut Cursor<&[u8]>) -> Result<Self, Error> {
        let hash = read_hash(cursor)?;
        let index = u32::from_le_stream(cursor)?;
        let script_bytes = read_from_varint(cursor)?;
        let height = u32::from_le_stream(cursor)?;
        let coinbase_script = read_coinbase_script(cursor, (script_bytes-4) as usize)?;
        let sequence = u32::from_le_stream(cursor)?;

        let coinbase_input = CoinBaseInput {
            hash,
            index,
            script_bytes,
            height,
            coinbase_script,
            sequence,
        };

        Ok(coinbase_input)
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut bytes = vec![];
        bytes.extend_from_slice(&self.hash);
        bytes.extend_from_slice(&self.index.to_le_bytes());
        bytes.extend_from_slice(&self.script_bytes.to_le_bytes());
        bytes.extend_from_slice(&self.height.to_le_bytes());
        bytes.extend_from_slice(&self.coinbase_script);
        bytes.extend_from_slice(&self.sequence.to_le_bytes());
        bytes
    }

}

#[derive(Debug)]
pub struct Outpoint {
    hash: [u8; 32],
    index: u32,
}

impl Outpoint {
    pub fn from_bytes(cursor: &mut Cursor<&[u8]>) -> Result<Self, Error> {
        let hash = read_hash(cursor)?;
        let index = u32::from_le_stream(cursor)?;
        let outpoint = Outpoint { hash, index };
        Ok(outpoint)
    }
}

#[derive(Debug)]
pub struct TxInput {
    previous_output: Outpoint,
    script_bytes: u64,
    script_sig: Vec<u8>,
    sequence: u32,
}

impl TxInput {
    pub fn vec_from_bytes(cursor: &mut Cursor<&[u8]>, count: usize) -> Result<Vec<Self>, Error> {
        let mut tx_inputs = vec![];

        for _ in 0..count {
            let previous_output = Outpoint::from_bytes(cursor)?;
            let script_bytes = read_from_varint(cursor)?;
            let script_sig = read_coinbase_script(cursor, script_bytes as usize)?;
            let sequence = u32::from_le_stream(cursor)?;

            let tx_input = TxInput {
                previous_output,
                script_bytes,
                script_sig,
                sequence,
            };

            tx_inputs.push(tx_input);
        }
        Ok(tx_inputs)
    }


    pub fn serialize_vec(tx_inputs: &Vec<Self>) -> Vec<u8> {
        let mut bytes = vec![];
        for tx_input in tx_inputs {
            bytes.extend_from_slice(&tx_input.previous_output.hash);
            bytes.extend_from_slice(&tx_input.previous_output.index.to_le_bytes());
            bytes.extend_from_slice(&tx_input.script_bytes.to_le_bytes());
            bytes.extend_from_slice(&tx_input.script_sig);
            bytes.extend_from_slice(&tx_input.sequence.to_le_bytes());
        }
        bytes
    }
}

#[derive(Debug)]
pub struct TxOutput {
    value: i64,
    pk_script_bytes: u64,
    pk_script: Vec<u8>,
}

impl TxOutput {
    pub fn vec_from_bytes(cursor: &mut Cursor<&[u8]>, n: usize) -> Result<Vec<Self>, Error> {
        let mut tx_outputs = vec![];

        for _ in 0..n {
            let value = i64::from_le_stream(cursor)?;
            let pk_script_bytes = read_from_varint(cursor)?;
            let pk_script = read_coinbase_script(cursor, pk_script_bytes as usize)?;

            let tx_output = TxOutput {
                value,
                pk_script_bytes,
                pk_script,
            };

            tx_outputs.push(tx_output);
        }

        Ok(tx_outputs)
    }

    pub fn serialize_vec(tx_outputs: &Vec<Self>) -> Vec<u8> {
        let mut bytes = vec![];
        for tx_output in tx_outputs {
            bytes.extend_from_slice(&tx_output.value.to_le_bytes());
            bytes.extend_from_slice(&tx_output.pk_script_bytes.to_le_bytes());
            bytes.extend_from_slice(&tx_output.pk_script);
        }
        bytes
    }
}

#[derive(Debug)]
enum TxInputType {
    CoinBaseInput(CoinBaseInput),
    TxInput(Vec<TxInput>),
}

impl TxInputType {
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            TxInputType::CoinBaseInput(coinbase_input) => coinbase_input.serialize(),
            TxInputType::TxInput(tx_inputs) => TxInput::serialize_vec(&tx_inputs),
        }
    }
}

#[derive(Debug)]
pub struct RawTransaction {
    version: u32,
    tx_in_count: u64,
    tx_in: TxInputType,
    tx_out_count: u64,
    tx_out: Vec<TxOutput>,
    lock_time: u32,
}

impl RawTransaction {
    pub fn coinbase_from_bytes(cursor: &mut Cursor<&[u8]>) -> Result<Self, Error> {
        let version = u32::from_le_stream(cursor)?;
        let tx_in_count = read_from_varint(cursor)?;
        let tx_in = TxInputType::CoinBaseInput(CoinBaseInput::from_bytes(cursor)?);
        let tx_out_count = read_from_varint(cursor)?;
        let tx_out = TxOutput::vec_from_bytes(cursor, tx_out_count as usize)?;
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

    pub fn vec_from_bytes(cursor: &mut Cursor<&[u8]>, count: usize) -> Result<Vec<Self>, Error> {
        let mut raw_transactions = vec![];

        for _ in 1..count {
            let version = u32::from_le_stream(cursor)?;

            let tx_in_count = read_from_varint(cursor)?;
            let tx_in =
                TxInputType::TxInput(TxInput::vec_from_bytes(cursor, tx_in_count as usize)?);

            let tx_out_count = read_from_varint(cursor)?;
            let tx_out = TxOutput::vec_from_bytes(cursor, tx_out_count as usize)?;

            let lock_time = u32::from_le_stream(cursor)?;

            let raw_transaction = RawTransaction {
                version,
                tx_in_count,
                tx_in,
                tx_out_count,
                tx_out,
                lock_time,
            };

            raw_transactions.push(raw_transaction);
        }

        Ok(raw_transactions)
    }

    pub fn serialize(transaction:&RawTransaction) -> Vec<u8> {
        let mut transaction_bytes = vec![];
        transaction_bytes.extend(&transaction.version.to_le_bytes());
        transaction_bytes.extend(&transaction.tx_in_count.to_le_bytes());
        transaction_bytes.extend(&transaction.tx_in.to_bytes());
        transaction_bytes.extend(&transaction.tx_out_count.to_le_bytes());
        transaction_bytes.extend(TxOutput::serialize_vec(&transaction.tx_out));
        transaction_bytes.extend(&transaction.lock_time.to_le_bytes());
        transaction_bytes
    }
}
