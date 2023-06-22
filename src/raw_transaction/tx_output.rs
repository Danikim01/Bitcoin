use crate::messages::utility::StreamRead;
use crate::raw_transaction::{read_coinbase_script, read_from_varint, to_compact_size_bytes};
use crate::utxo::p2pkh_to_address;
use bitcoin_hashes::{ripemd160, sha256, Hash};
use std::io::{self, Cursor, Error, Read};

/// Struct that represents the pk_script data of a transaction output
#[derive(Debug, Clone)]
pub struct PkScriptData {
    pub pk_hash: [u8; 20],
}

impl PkScriptData {
    /// Creates a new `PkScriptData` from a pk_script
    pub fn from_pk_script_bytes(pk_script_bytes: &[u8]) -> Result<Self, Error> {
        let first_hash = sha256::Hash::hash(pk_script_bytes);
        let second_hash = ripemd160::Hash::hash(&first_hash[..]);

        let mut bytes = [0u8; 20];
        bytes.copy_from_slice(&second_hash[..]);
        Ok(PkScriptData { pk_hash: bytes })
    }
}

/// Struct that represents a transaction output (value and pk_script)
#[derive(Debug, Clone)]
pub struct TxOutput {
    pub value: u64,
    pub pk_script_bytes: u64,
    pub pk_script: Vec<u8>,
}

impl TxOutput {
    /// Returns the destined address of the transaction output
    pub fn get_address(&self) -> io::Result<String> {
        let script_bytes = self.pk_script.clone();
        let mut cursor: Cursor<&[u8]> = Cursor::new(&script_bytes);

        // consume cursor until reading 0x14
        let buf = &mut [0; 1];
        while buf[0] != 0x14 {
            cursor.read_exact(buf)?;
        }

        let mut pk_hash = [0; 20];
        cursor.read_exact(&mut pk_hash)?;

        Ok(p2pkh_to_address(pk_hash))
    }

    /// Checks if the transaction output is destined to the given address
    pub fn destined_to(&self, address: &str) -> bool {
        match self.get_address() {
            Ok(a) => a == address,
            Err(_) => false,
        }
    }
    
    /// Deserialize a `TxOutput` from a byte array.
    pub fn from_bytes(cursor: &mut Cursor<&[u8]>) -> Result<Self, Error> {
        let value = u64::from_le_stream(cursor)?; // this is actually a float?
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
    
    /// Deserialize a vector of `TxOutput` from a byte array.
    pub fn vec_from_bytes(cursor: &mut Cursor<&[u8]>, n: usize) -> Result<Vec<Self>, Error> {
        let mut tx_outputs = vec![];

        for _ in 0..n {
            let tx_output = TxOutput::from_bytes(cursor)?;
            tx_outputs.push(tx_output);
        }

        Ok(tx_outputs)
    }

    /// Serialize a `TxOutput` into a byte array.
    pub fn _serialize(&self) -> Vec<u8> {
        let mut bytes = vec![];
        bytes.extend_from_slice(&self.value.to_le_bytes());

        bytes.extend_from_slice(&to_compact_size_bytes(self.pk_script_bytes));

        bytes.extend_from_slice(&self.pk_script);
        bytes
    }

    /// Serialize a vector of `TxOutput` into a byte array.
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

#[cfg(test)]
mod tests {
    use crate::utility::_decode_hex;

    use super::*;

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
    fn test_deseareal_and_sereal() {
        let bytes =
            _decode_hex("b7051e00000000001976a914c9bc003bf72ebdc53a9572f7ea792ef49a2858d788ac")
                .unwrap();
        let txou = TxOutput::from_bytes(&mut Cursor::new(&bytes)).unwrap();
        let serialized_txou = txou._serialize();
        assert_eq!(bytes, serialized_txou);
    }

    #[test]
    fn test_txout_destined_to() {
        let bytes =
            _decode_hex("96051a00000000001976a914c9bc003bf72ebdc53a9572f7ea792ef49a2858d788ac")
                .unwrap();
        let txou = TxOutput::from_bytes(&mut Cursor::new(&bytes)).unwrap();
        let address = "myudL9LPYaJUDXWXGz5WC6RCdcTKCAWMUX";
        assert!(txou.destined_to(address));
        assert!(!txou.destined_to("foo"));
    }
}
