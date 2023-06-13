use crate::io::{self, Cursor};
use crate::messages::utility::{read_from_varint, read_hash, to_compact_size_bytes, StreamRead};

use crate::utility::double_hash;
use crate::utxo::{Utxo, UtxoSet};
use bitcoin_hashes::{ripemd160, sha256, Hash};
use std::collections::HashMap;
use std::io::{Error, ErrorKind, Read};

fn read_coinbase_script(cursor: &mut Cursor<&[u8]>, count: usize) -> io::Result<Vec<u8>> {
    let mut array = vec![0_u8; count];
    cursor.read_exact(&mut array)?;
    Ok(array)
}

#[derive(Debug, Clone)]
pub struct TxInput {
    pub previous_output: Outpoint,
    pub script_bytes: u64,
    pub script_sig: Vec<u8>,
    pub sequence: u32,
}

impl TxInput {
    pub fn from_bytes(cursor: &mut Cursor<&[u8]>) -> Result<Self, Error> {
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

        Ok(tx_input)
    }

    pub fn vec_from_bytes(cursor: &mut Cursor<&[u8]>, count: usize) -> Result<Vec<Self>, Error> {
        let mut tx_inputs = vec![];

        for _ in 0..count {
            let tx_input = TxInput::from_bytes(cursor)?;
            tx_inputs.push(tx_input);
        }
        Ok(tx_inputs)
    }

    pub fn _serialize(&self) -> Vec<u8> {
        let mut bytes = vec![];
        bytes.extend_from_slice(&self.previous_output._hash);
        bytes.extend_from_slice(&self.previous_output._index.to_le_bytes());

        // this is needed in case the script bytes is 0
        match self.script_bytes {
            0 => {
                bytes.extend_from_slice(&[0u8]);
            }
            _ => {
                bytes.extend_from_slice(&to_compact_size_bytes(self.script_bytes));
            }
        }

        bytes.extend_from_slice(&self.script_sig);
        bytes.extend_from_slice(&self.sequence.to_le_bytes());
        bytes
    }

    pub fn serialize_vec(tx_inputs: &Vec<Self>) -> Vec<u8> {
        let mut bytes = vec![];
        for tx_input in tx_inputs {
            bytes.extend_from_slice(&tx_input._serialize());
        }
        bytes
    }
}

#[derive(Debug, Clone)]
pub struct Outpoint {
    pub _hash: [u8; 32],
    pub _index: u32,
}

impl Outpoint {
    pub fn from_bytes(cursor: &mut Cursor<&[u8]>) -> Result<Self, Error> {
        let _hash = read_hash(cursor)?;
        let _index = u32::from_le_stream(cursor)?;
        let outpoint = Outpoint { _hash, _index };
        Ok(outpoint)
    }
}

#[derive(Debug, Clone)]
pub enum TxInputType {
    CoinBaseInput(CoinBaseInput),
    TxInput(Vec<TxInput>),
}

impl TxInputType {
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            TxInputType::CoinBaseInput(coinbase_input) => coinbase_input._serialize(),
            TxInputType::TxInput(tx_inputs) => TxInput::serialize_vec(tx_inputs),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CoinBaseInput {
    _hash: [u8; 32],
    _index: u32,
    _script_bytes: u64,
    _height: u32,
    _coinbase_script: Vec<u8>,
    _sequence: u32,
}

fn read_height(cursor: &mut Cursor<&[u8]>) -> io::Result<u32> {
    let val = u8::from_le_stream(cursor)?;
    if val != 0x03 {
        let err_str = format!("Height unsupported: {}", val);
        println!("la altura es {}", val);
        return Err(Error::new(ErrorKind::Unsupported, err_str.as_str()));
    }
    let mut array = [0u8; 4];
    array[0] = u8::from_le_stream(cursor)?;
    array[1] = u8::from_le_stream(cursor)?;
    array[2] = u8::from_le_stream(cursor)?;
    // let mut array = vec![0_u8; val as usize];
    // cursor.read_exact(&mut array)?;
    Ok(u32::from_le_bytes(array))
}

fn serialize_height(height: u32) -> Vec<u8> {
    let mut bytes = vec![];
    bytes.push(0x03);
    bytes.extend_from_slice(&height.to_le_bytes()[0..3]);
    bytes
}

impl CoinBaseInput {
    pub fn from_bytes(cursor: &mut Cursor<&[u8]>) -> io::Result<Self> {
        let _hash = read_hash(cursor)?;
        let _index = u32::from_le_stream(cursor)?;
        let _script_bytes = read_from_varint(cursor)?;
        let _height = match read_height(cursor) {
            Ok(height) => height,
            Err(err) => {
                println!("Invalid height, script bytes was set to {}", _script_bytes);
                Err(err)?
            }
        };

        let _coinbase_script = read_coinbase_script(cursor, (_script_bytes - 4) as usize)?;
        let _sequence = u32::from_le_stream(cursor)?;

        let coinbase_input = CoinBaseInput {
            _hash,
            _index,
            _script_bytes,
            _height,
            _coinbase_script,
            _sequence,
        };

        Ok(coinbase_input)
    }

    pub fn _serialize(&self) -> Vec<u8> {
        let mut bytes = vec![];
        bytes.extend_from_slice(&self._hash);
        bytes.extend_from_slice(&self._index.to_le_bytes());
        bytes.extend_from_slice(&to_compact_size_bytes(self._script_bytes));
        // bytes.extend_from_slice(remove_right_zero_bytes(&self._height.to_le_bytes()));
        bytes.extend_from_slice(&serialize_height(self._height));
        bytes.extend_from_slice(&self._coinbase_script);
        bytes.extend_from_slice(&self._sequence.to_le_bytes());
        bytes
    }
}

#[derive(Debug, Clone)]
pub struct PkScriptData {
    pub pk_hash: [u8; 20],
}

impl PkScriptData {
    pub fn from_pk_script_bytes(pk_script_bytes: &[u8]) -> Result<Self, Error> {
        let first_hash = sha256::Hash::hash(pk_script_bytes);
        let second_hash = ripemd160::Hash::hash(&first_hash[..]);

        let mut bytes = [0u8; 20];
        bytes.copy_from_slice(&second_hash[..]);
        Ok(PkScriptData { pk_hash: bytes })
    }
}

#[derive(Debug, Clone)]
pub struct TxOutput {
    pub value: i64,
    pub pk_script_bytes: u64,
    pub pk_script: Vec<u8>,
}

impl TxOutput {
    pub fn from_bytes(cursor: &mut Cursor<&[u8]>) -> Result<Self, Error> {
        let value = i64::from_le_stream(cursor)?; // this is actually a float?
        let pk_script_bytes = read_from_varint(cursor)?;
        let pk_script = read_coinbase_script(cursor, pk_script_bytes as usize)?;

        let _pk_script_data = PkScriptData::from_pk_script_bytes(&pk_script)?;

        let tx_output = TxOutput {
            value,
            pk_script_bytes,
            pk_script,
        };

        Ok(tx_output)
    }

    pub fn vec_from_bytes(cursor: &mut Cursor<&[u8]>, n: usize) -> Result<Vec<Self>, Error> {
        let mut tx_outputs = vec![];

        for _ in 0..n {
            let tx_output = TxOutput::from_bytes(cursor)?;
            tx_outputs.push(tx_output);
        }

        Ok(tx_outputs)
    }

    pub fn _serialize(&self) -> Vec<u8> {
        let mut bytes = vec![];
        bytes.extend_from_slice(&self.value.to_le_bytes());

        bytes.extend_from_slice(&to_compact_size_bytes(self.pk_script_bytes));

        bytes.extend_from_slice(&self.pk_script);
        bytes
    }

    pub fn serialize_vec(tx_outputs: &Vec<Self>) -> Vec<u8> {
        let mut bytes = vec![];
        for tx_output in tx_outputs {
            bytes.extend_from_slice(&tx_output._serialize());
        }
        bytes
    }

    pub fn _get_pk_script_data(&self) -> Result<PkScriptData, Error> {
        PkScriptData::from_pk_script_bytes(&self.pk_script)
    }
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

    pub fn generate_utxo(&self, utxo_set: &mut UtxoSet) -> io::Result<()> {
        let new_id = double_hash(&self.serialize()).to_byte_array();
        let new_utxo = Utxo::from_raw_transaction(self)?;

        for transaction in &new_utxo.transactions {
            let tx_address = match transaction.get_address() {
                Ok(val) => val,
                Err(_) => "no_address".to_string(),
            };

            match utxo_set.get_mut(&tx_address) {
                Some(val) => {
                    val.insert(new_id, transaction.clone());
                }
                None => {
                    let mut map = HashMap::new();
                    map.insert(new_id, transaction.clone());
                    utxo_set.insert(tx_address, map);
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
    use crate::utility::_decode_hex;

    use super::*;
    use std::fs;

    #[test]
    fn test_coinbase_input_deserialization() {
        let raw_transaction_bytes: &[u8] = &[
            0x01, 0x00, 0x00, 0x00, // version: 1
            0x01, // tx_in_count: 1
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff, // hash
            0x1d, // script bytes - The height and coinbase script has 29 bytes.
            0x03, 0x0f, 0x8d,
            0x13, // height - 0x03-byte little-endian integer: 0x0f8d13 = 1281295
            0x04, 0x9f, 0xaa, 0x80, 0x5a, 0x06, 0x35, 0x38, 0x70, 0x6f, 0x6f, 0x6c, 0x0c, 0x00,
            0x01, 0x00, 0x00, 0xfe, 0x22, 0x03, 0x00, 0x00, 0x00, 0x00,
            0x00, // coinbase script - Arbitrary data entered by the miner
            0xff, 0xff, 0xff, 0xff, // sequence - End of this input
            0x01, // tx_out_count - 1 transaction output
            0x53, 0x41, 0xcb, 0x04, 0x00, 0x00, 0x00,
            0x00, // value - Amount of the first output in little-endian integer
            0x19, // script bytes - The pubkey script has 25 bytes
            0x76, 0xa9, 0x14, 0xf1, 0x12, 0x98, 0xce, 0x77, 0x7c, 0xb5, 0xdb, 0x5c, 0x09, 0x25,
            0x0c, 0xad, 0x4e, 0xb8, 0x56, 0xb1, 0xe3, 0x66, 0xef, 0x88,
            0xac, // pubkey script - Represents the account address of the miner
            0x00, 0x00, 0x00,
            0x00, // lock_time - Block number or timestamp at which this transaction is unlocked
        ];
        let mut cursor = Cursor::new(raw_transaction_bytes);

        let coinbase_transaction = RawTransaction::coinbase_from_bytes(&mut cursor).unwrap();

        assert_eq!(coinbase_transaction.version, 1);
        assert_eq!(coinbase_transaction.tx_in_count, 1);
        if let TxInputType::CoinBaseInput(coinbase_input) = coinbase_transaction.tx_in {
            assert_eq!(coinbase_input._hash, [0u8; 32]);
            assert_eq!(coinbase_input._index, 0xffffffff);
            assert_eq!(coinbase_input._script_bytes, 29);
            assert_eq!(coinbase_input._height, 1281295);
            assert_eq!(
                coinbase_input._coinbase_script,
                [
                    0x04, 0x9f, 0xaa, 0x80, 0x5a, 0x06, 0x35, 0x38, 0x70, 0x6f, 0x6f, 0x6c, 0x0c,
                    0x00, 0x01, 0x00, 0x00, 0xfe, 0x22, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00
                ]
            );
            assert_eq!(coinbase_input._sequence, 0xffffffff);
        } else {
            panic!("Expected CoinBaseInput type for tx_in, found different variant.");
        }
        assert_eq!(coinbase_transaction.tx_out_count, 1);
        assert_eq!(coinbase_transaction.tx_out[0].value, 80429395);
        assert_eq!(coinbase_transaction.tx_out[0].pk_script_bytes, 25);
        assert_eq!(
            coinbase_transaction.tx_out[0].pk_script,
            [
                0x76, 0xa9, 0x14, 0xf1, 0x12, 0x98, 0xce, 0x77, 0x7c, 0xb5, 0xdb, 0x5c, 0x09, 0x25,
                0x0c, 0xad, 0x4e, 0xb8, 0x56, 0xb1, 0xe3, 0x66, 0xef, 0x88, 0xac
            ]
        );
        assert_eq!(coinbase_transaction.lock_time, 0);
    }

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
    fn test_txou_serialization() {
        // txou bytes
        let bytes: &[u8] = &[
            0xf0, 0xca, 0x05, 0x2a, 0x01, 0x00, 0x00, 0x00, // value
            0x19, // pk_script_bytes
            0x76, // OP_DUP
            0xa9, // OP_HASH160
            0x14, // OP_PUSHDATA(20)
            0xcb, 0xc2, 0x0a, 0x76, 0x64, 0xf2, 0xf6, 0x9e, 0x53, 0x55, 0xaa, 0x42, 0x70, 0x45,
            0xbc, 0x15, 0xe7, 0xc6, 0xc7, 0x72, // PubKeyHash
            0x88, // OP_EQUALVERIFY
            0xac, // OP_CHECKSIG
        ];

        // we deserialize the txou
        let mut cursor = Cursor::new(bytes);
        let txou = TxOutput::from_bytes(&mut cursor).unwrap();

        // we serialize the txou
        let serialized_txou = txou._serialize();

        // we compare the deserialized txou with the original one
        assert_eq!(bytes[0..8], serialized_txou[0..8]); // value bytes
        assert_eq!(bytes[8], serialized_txou[8]); // pk_script_bytes
        assert_eq!(bytes[9..], serialized_txou[9..]); // pk_script
    }

    #[test]
    fn test_txin_serialization() {
        // txin bytes
        let bytes: &[u8] = &[
            0xf0, 0xca, 0x05, 0x2a, 0x01, 0x00, 0x00, 0x00, //
            0xf0, 0xca, 0x05, 0x2a, 0x01, 0x00, 0x00, 0x00, //
            0xf0, 0xca, 0x05, 0x2a, 0x01, 0x00, 0x00, 0x00, //
            0xf0, 0xca, 0x05, 0x2a, 0x01, 0x00, 0x00, 0x00, // previous_output
            0x19, // signature_script_bytes
            0x19, 0x76, 0xa9, 0x14, 0xcb, 0xc2, 0x0a, 0x76, //
            0x64, 0xf2, 0xf6, 0x9e, 0x53, 0x55, 0xaa, 0x42, //
            0x70, 0x45, 0xbc, 0x15, 0xe7, 0xc6, 0xc7, 0x72, //
            0x88, // signature_script
            0xff, 0xff, 0xff, 0xff, // sequence
        ];

        // we deserialize the txin
        let mut cursor = Cursor::new(bytes);
        let txin = TxInput::from_bytes(&mut cursor).unwrap();

        // we serialize the txin
        let serialized_txin = txin._serialize();

        // we compare the deserialized txin with the original one
        assert_eq!(bytes[0..32], serialized_txin[0..32]); // previous_output
        assert_eq!(bytes[32], serialized_txin[32]); // signature_script_bytes
        assert_eq!(bytes[33..58], serialized_txin[33..58]); // signature_script
        assert_eq!(bytes[58..61], serialized_txin[58..61]); // sequence
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
