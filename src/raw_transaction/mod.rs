use crate::io::{self, Cursor};
use crate::messages::utility::{read_from_varint, read_hash, to_compact_size_bytes, StreamRead};

use crate::utility::double_hash;
use crate::utxo::{Utxo, UtxoSet};
use bitcoin_hashes::Hash;
use std::collections::HashMap;
use std::io::{Error, Read};

pub mod tx_input;
use tx_input::{TxInput, TxInputType, CoinBaseInput};
pub mod tx_output;
use tx_output::TxOutput;

fn read_coinbase_script(cursor: &mut Cursor<&[u8]>, count: usize) -> io::Result<Vec<u8>> {
    let mut array = vec![0_u8; count];
    cursor.read_exact(&mut array)?;
    Ok(array)
}

pub fn generate_txid_vout_bytes(txid: [u8; 32], vout: [u8; 4]) -> [u8; 36] {
    let mut bytes: [u8; 36] = [0; 36];
    bytes[..32].copy_from_slice(&txid);
    bytes[32..].copy_from_slice(&vout);
    bytes
}

#[derive(Debug, Clone)]
pub struct RawTransaction {
    pub version: u32,
    pub tx_in_count: u64,
    pub tx_in: TxInputType,
    pub tx_out_count: u64,
    pub tx_out: Vec<TxOutput>,
    pub lock_time: u32,
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

    fn get_spent_utxos(&self) -> Vec<[u8; 36]> {
        let mut spent_utxos: Vec<[u8; 36]> = vec![];

        match &self.tx_in {
            TxInputType::CoinBaseInput(_) => {}
            TxInputType::TxInput(inputs) => {
                for input in inputs {
                    let txid: [u8; 32] = input.previous_output.hash;
                    let vout: [u8; 4] = input.previous_output.index.to_le_bytes(); // may need to reverse this
                    spent_utxos.push(generate_txid_vout_bytes(txid, vout));
                }
            }
        }

        spent_utxos
    }

    pub fn generate_utxo(&self, utxo_set: &mut UtxoSet) -> io::Result<()> {
        let new_id = double_hash(&self.serialize()).to_byte_array();

        // add spent utxos
        let spent_utxos: Vec<[u8; 36]> = self.get_spent_utxos();
        utxo_set.spent.extend(spent_utxos);

        // add generated utxos
        let new_utxo = Utxo::from_raw_transaction(self)?;
        for transaction in &new_utxo.transactions {
            let tx_address = match transaction.get_address() {
                Ok(val) => val,
                Err(_) => "no_address".to_string(),
            };

            match utxo_set.set.get_mut(&tx_address) {
                Some(val) => {
                    val.insert(new_id, transaction.clone());
                }
                None => {
                    let mut map = HashMap::new();
                    map.insert(new_id, transaction.clone());
                    utxo_set.set.insert(tx_address, map);
                }
            }
        }

        Ok(())
    }

    pub fn _validate(&self, utxo_set: &mut UtxoSet) -> io::Result<()> {
        // check the inputs and mark them as spent
        // self.validate_inputs(utxo_set)?; // unused function as of now

        // generate new utxos from the outputs

        self.generate_utxo(utxo_set)?;

        Ok(())
    }

    fn read_witnesses(cursor: &mut Cursor<&[u8]>, tx_in_count: u64) -> io::Result<()> {
        let mut witnesses = Vec::new();
        for _ in 0..tx_in_count {
            let witness_len = read_from_varint(cursor)?;
            for _ in 0..witness_len {
                let length = read_from_varint(cursor)?;
                let mut witness_data = vec![0u8; length as usize];
                cursor.read_exact(&mut witness_data)?;
                witnesses.push(witness_data);
            }
        }
        Ok(())
    }

    pub fn from_bytes(cursor: &mut Cursor<&[u8]>) -> Result<Self, Error> {
        let version = u32::from_le_stream(cursor)?;

        let mut has_witness = false;

        let mut tx_in_count = read_from_varint(cursor)?;
        if tx_in_count == 0 {
            let _flag: u8 = u8::from_le_stream(cursor)?;
            tx_in_count = read_from_varint(cursor)?;
            has_witness = true;
        }

        let tx_in = TxInputType::TxInput(TxInput::vec_from_bytes(cursor, tx_in_count as usize)?);

        let tx_out_count = read_from_varint(cursor)?;
        let tx_out = TxOutput::vec_from_bytes(cursor, tx_out_count as usize)?;

        if has_witness {
            Self::read_witnesses(cursor, tx_in_count)?;
        }

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
            let raw_transaction = RawTransaction::from_bytes(cursor)?;
            raw_transactions.push(raw_transaction);
        }

        Ok(raw_transactions)
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut transaction_bytes = vec![];
        transaction_bytes.extend(self.version.to_le_bytes());
        transaction_bytes.extend(&to_compact_size_bytes(self.tx_in_count));
        transaction_bytes.extend(self.tx_in.to_bytes());
        transaction_bytes.extend(&to_compact_size_bytes(self.tx_out_count));
        transaction_bytes.extend(TxOutput::serialize_vec(&self.tx_out));
        transaction_bytes.extend(self.lock_time.to_le_bytes());
        transaction_bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_compactsize_serialization_u16() {
        let bytes: &[u8] = &[
            0xfd, // format
            0x03, 0x02, // number 515
        ];

        let mut cursor = Cursor::new(bytes);

        let compact_size = read_from_varint(&mut cursor).unwrap();
        assert_eq!(compact_size, 515);

        let serialized_compactsize = to_compact_size_bytes(compact_size);
        assert_eq!(serialized_compactsize, bytes);
    }

    #[test]
    fn test_coinbase_transaction_serialization() {
        // coinbase bytes
        let bytes: &[u8] = &[
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, // hash
            0xff, 0xff, 0xff, 0xff, // index
            0x29, // script bytes
            0x03, // bytes in block height
            0x4e, 0x01, 0x05, // block height
            0x06, 0x2f, 0x50, 0x32, 0x53, 0x48, 0x2f, 0x04, 0x72, 0xd3, 0x54, 0x54, 0x08, 0x5f,
            0xff, 0xed, 0xf2, 0x40, 0x00, 0x00, 0xf9, 0x0f, 0x54, 0x69, 0x6d, 0x65, 0x20, 0x26,
            0x20, 0x48, 0x65, 0x61, 0x6c, 0x74, 0x68, 0x20, 0x21, // script
            0x00, 0x00, 0x00, 0x00, // sequence
        ];

        // we deserialize the coinbase transaction
        let mut cursor = Cursor::new(bytes);
        let coinbase = CoinBaseInput::from_bytes(&mut cursor).unwrap();

        // we serialize the coinbase transaction
        let serialized_coinbase = coinbase._serialize();

        // we compare the deserialized transaction with the original one
        assert_eq!(bytes[0..32], serialized_coinbase[0..32]); // hash
        assert_eq!(bytes[32..36], serialized_coinbase[32..36]); // index
        assert_eq!(bytes[36], serialized_coinbase[36]); // script bytes
        assert_eq!(bytes[37], serialized_coinbase[37]); // bytes in block height
        assert_eq!(bytes[38..41], serialized_coinbase[38..41]); // block height
        assert_eq!(bytes[41..77], serialized_coinbase[41..77]); // script
        assert_eq!(bytes[77..82], serialized_coinbase[77..82]); // sequence
    }

    #[test]
    fn test_transaction_serialization() {
        // Needed to avoid github actions error
        let bytes = match fs::read("./tmp/block_message_payload.dat") {
            Ok(bytes) => bytes,
            Err(e) => {
                println!("Error reading file: {}", e);
                // empty &[u8] vec
                Vec::new()
            }
        };

        if !bytes.is_empty() {
            // create a cursor over the bytes
            let mut cursor: Cursor<&[u8]> = Cursor::new(&bytes);

            // we skip the first 80 bytes (block header)
            cursor.set_position(80);

            // we read the txn_count
            let txn_count = read_from_varint(&mut cursor).unwrap();
            let mut pos_start = cursor.position() as usize;

            // we read the first transaction manually as it's a coinbase transaction
            let tx_coinbase = RawTransaction::coinbase_from_bytes(&mut cursor).unwrap();
            let mut pos_end = cursor.position() as usize;

            // we serialize the transaction
            let serialized_tx_coinbase = tx_coinbase.serialize();

            assert_eq!(bytes[pos_start..pos_end], serialized_tx_coinbase);

            // we read the rest of the transactions
            for _ in 1..txn_count {
                // save the cursor position
                pos_start = cursor.position() as usize;

                // we deserialize the transaction
                let tx = RawTransaction::from_bytes(&mut cursor).unwrap();

                // we save the cursor position
                pos_end = cursor.position() as usize;

                // we serialize the transaction
                let serialized_tx = tx.serialize();

                // we compare bytes from start to end
                assert_eq!(bytes[pos_start..pos_end], serialized_tx);
                // println!("serialized transaction {} correctly", i);
            }
        }
    }

    #[test]
    fn test_transaction_vector_serialization() {
        // Needed to avoid github actions error
        let bytes = match fs::read("./tmp/block_message_payload.dat") {
            Ok(bytes) => bytes,
            Err(e) => {
                println!("Error reading file: {}", e);
                // empty &[u8] vec
                Vec::new()
            }
        };

        if !bytes.is_empty() {
            // create a cursor over the bytes
            let mut cursor: Cursor<&[u8]> = Cursor::new(&bytes);

            // we skip the first 80 bytes (block header)
            cursor.set_position(80);

            // we read the txn_count
            let txn_count = read_from_varint(&mut cursor).unwrap() as usize;

            // we read the first transaction manually as it's a coinbase transaction
            let tx_coinbase = RawTransaction::coinbase_from_bytes(&mut cursor).unwrap();

            // we read the rest of the transactions
            let txns = RawTransaction::vec_from_bytes(&mut cursor, txn_count).unwrap();

            // we serialize all transactions
            let mut serialized_txn_vec = Vec::new();

            let serialized_tx_coinbase = tx_coinbase.serialize();
            serialized_txn_vec.push(serialized_tx_coinbase);

            for tx in txns {
                let serialized_tx = tx.serialize();
                serialized_txn_vec.push(serialized_tx);
            }

            // we compare the bytes
            assert_eq!(bytes[81..], serialized_txn_vec.concat());
        }
    }
}
