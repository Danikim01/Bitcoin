use crate::messages::utility::StreamRead;
use crate::raw_transaction::{
    read_coinbase_script, read_from_varint, read_hash, to_compact_size_bytes,
};
use crate::utxo::p2pkh_to_address;
use bitcoin_hashes::{hash160, Hash};
use std::io;
use std::io::{Cursor, Error, ErrorKind, Read};

/// Store a tx input (previous output, script sig, sequence)
#[derive(Debug, Clone)]
pub struct TxInput {
    pub previous_output: Outpoint,
    pub script_bytes: u64,
    pub script_sig: Vec<u8>,
    pub sequence: u32,
}

impl TxInput {
    /// Read the address from the script sig
    pub fn get_address(&self) -> io::Result<String> {
        let script_bytes = self.script_sig.clone();
        let mut cursor: Cursor<&[u8]> = Cursor::new(&script_bytes);

        // get sig length
        let sig_length = u8::from_le_stream(&mut cursor)?;

        // skip sig
        cursor.set_position(cursor.position() + sig_length as u64);

        // read pubkey length
        let pubkey_length = u8::from_le_stream(&mut cursor)?;

        // read pubkey
        let mut pubkey = vec![0u8; pubkey_length as usize];
        cursor.read_exact(&mut pubkey)?;

        // get address
        let h160 = hash160::Hash::hash(&pubkey).to_byte_array();
        Ok(p2pkh_to_address(h160))
    }

    /// Check if the input is destined to the given address    
    pub fn destined_from(&self, address: &str) -> bool {
        match self.get_address() {
            Ok(addr) => addr == address,
            Err(..) => false,
        }
    }

    /// Deserialize a tx input from a byte Cursor
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

    /// Deserialize a vector of tx inputs from a byte Cursor
    pub fn vec_from_bytes(cursor: &mut Cursor<&[u8]>, count: usize) -> Result<Vec<Self>, Error> {
        let mut tx_inputs = vec![];

        for _ in 0..count {
            let tx_input = TxInput::from_bytes(cursor)?;
            tx_inputs.push(tx_input);
        }
        Ok(tx_inputs)
    }

    /// Serialize a tx input to bytes
    pub fn _serialize(&self) -> Vec<u8> {
        let mut bytes = vec![];
        bytes.extend_from_slice(&self.previous_output.hash);
        bytes.extend_from_slice(&self.previous_output.index.to_le_bytes());

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

    /// Serialize a vector of tx inputs to bytes
    pub fn serialize_vec(tx_inputs: &Vec<Self>) -> Vec<u8> {
        let mut bytes = vec![];
        for tx_input in tx_inputs {
            bytes.extend_from_slice(&tx_input._serialize());
        }
        bytes
    }
}

/// Represent outpoint (hash of previous utxo, index of previous utxo)
#[derive(Debug, Clone)]
pub struct Outpoint {
    pub hash: [u8; 32],
    pub index: u32,
}

impl Outpoint {
    /// Deserialize an outpoint from a byte Cursor
    pub fn from_bytes(cursor: &mut Cursor<&[u8]>) -> Result<Self, Error> {
        let hash = read_hash(cursor)?;
        let index = u32::from_le_stream(cursor)?;
        let outpoint = Outpoint { hash, index };
        Ok(outpoint)
    }
}

/// Represent a tx input type (coinbase or tx input vector)
#[derive(Debug, Clone)]
pub enum TxInputType {
    CoinBaseInput(CoinBaseInput),
    TxInput(Vec<TxInput>),
}

impl TxInputType {
    /// Deserialize a tx input type from a byte Cursor
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            TxInputType::CoinBaseInput(coinbase_input) => coinbase_input._serialize(),
            TxInputType::TxInput(tx_inputs) => TxInput::serialize_vec(tx_inputs),
        }
    }
}

/// Represent a coinbase input
#[derive(Debug, Clone)]
pub struct CoinBaseInput {
    pub _hash: [u8; 32],
    pub _index: u32,
    pub _script_bytes: u64,
    pub _height: u32,
    pub _coinbase_script: Vec<u8>,
    pub _sequence: u32,
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
    /// Deserialize a coinbase input from a byte Cursor
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

    /// Serialize a coinbase input to bytes
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

#[cfg(test)]
mod tests {

    use super::*;
    use crate::{raw_transaction::RawTransaction, utility::_decode_hex};

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
    fn test_txin_destined_from() {
        let txin_bytes = _decode_hex("881468a1a95473ed788c8a13bcdb7e524eac4f1088b1e2606ffb95492e239b10000000006a473044022021dc538aab629f2be56304937e796884356d1e79499150f5df03e8b8a545d17702205b76bda9c238035c907cbf6a39fa723d65f800ebb8082bdbb62d016d7937d990012102a953c8d6e15c569ea2192933593518566ca7f49b59b91561c01e30d55b0e1922ffffffff").unwrap();
        let txin = TxInput::from_bytes(&mut Cursor::new(&txin_bytes)).unwrap();
        let address = "myudL9LPYaJUDXWXGz5WC6RCdcTKCAWMUX";
        assert!(txin.destined_from(address));
        assert!(!txin.destined_from("foo"));
    }
}
