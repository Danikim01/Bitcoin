use crate::io::{self, Cursor};
use crate::messages::utility::{
    read_from_varint, read_hash, to_compact_size_bytes, to_varint, StreamRead,
};
use crate::messages::{HashId, Serialize};

use crate::utility::{double_hash, to_io_err};
use crate::utxo::{Utxo, UtxoSet, WalletUtxo};
use bitcoin_hashes::Hash;
use std::{
    io::{Error, Read},
    str::FromStr,
};

pub mod tx_input;
use tx_input::{CoinBaseInput, Outpoint, TxInput, TxInputType};
pub mod tx_output;
use tx_output::TxOutput;

use super::messages::Message as Msg;

use secp256k1::{Message, PublicKey, Secp256k1, SecretKey};
use crate::network_controller::{TransactionDisplayInfo, TransactionRole};

const SIGHASH_ALL: u32 = 1;

fn read_coinbase_script(cursor: &mut Cursor<&[u8]>, count: usize) -> io::Result<Vec<u8>> {
    let mut array = vec![0_u8; count];
    cursor.read_exact(&mut array)?;
    Ok(array)
}

fn der_sign_with_priv_key(z: &[u8], private_key: &str) -> io::Result<Vec<u8>> {
    let message = &z;

    let secp: Secp256k1<secp256k1::All> = Secp256k1::gen_new();
    let message_slice: &[u8] = message;
    let message_slice = Message::from_slice(message_slice).map_err(to_io_err)?;
    let private_key = SecretKey::from_str(private_key).map_err(to_io_err)?;
    let signature = secp.sign_ecdsa(&message_slice, &private_key);

    // Convert the DER-encoded signature to bytes
    Ok(signature.serialize_der().to_vec())
}

fn pub_key_from_priv_key(private_key: &str) -> io::Result<Vec<u8>> {
    let secp: Secp256k1<secp256k1::All> = Secp256k1::gen_new();
    let private_key = SecretKey::from_str(private_key).map_err(to_io_err)?;
    let public_key = PublicKey::from_secret_key(&secp, &private_key);
    Ok(public_key.serialize().to_vec())
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

#[derive(Debug, Clone, PartialEq)]
pub enum TransactionOrigin {
    Block,
    Pending,
}

impl RawTransaction {
    fn sig_txin(&self, s: &mut Vec<u8>, prev_pk_script: Vec<u8>, index: usize) {
        if let TxInputType::TxInput(tx_ins) = &self.tx_in {
            for (i, tx_in) in tx_ins.iter().enumerate() {
                if i == index {
                    let pubkey_script_bytes = prev_pk_script.clone();
                    let tin = TxInput {
                        previous_output: Outpoint {
                            hash: tx_in.previous_output.hash,
                            index: tx_in.previous_output.index,
                        },
                        script_bytes: pubkey_script_bytes.len() as u64,
                        script_sig: pubkey_script_bytes,
                        sequence: tx_in.sequence,
                    };
                    s.extend(tin._serialize());
                } else {
                    let tin = TxInput {
                        previous_output: Outpoint {
                            hash: tx_in.previous_output.hash,
                            index: tx_in.previous_output.index,
                        },
                        script_bytes: 0,
                        script_sig: vec![],
                        sequence: tx_in.sequence,
                    };
                    s.extend(tin._serialize());
                }
            }
        }
    }

    fn sig_hash(&self, prev_pk_script: Vec<u8>, index: usize) -> io::Result<[u8; 32]> {
        let mut s = Vec::new();
        s.extend(self.version.to_le_bytes());

        s.extend(to_varint(self.tx_in_count));
        self.sig_txin(&mut s, prev_pk_script, index);

        s.extend(to_varint(self.tx_out_count));
        for tx_out in self.tx_out.iter() {
            s.extend(tx_out._serialize());
        }
        s.extend(self.lock_time.to_le_bytes());
        s.extend(SIGHASH_ALL.to_le_bytes());

        let h256 = double_hash(&s);
        let bytes = h256.to_byte_array();
        Ok(bytes)
    }

    pub fn sign_input(
        &mut self,
        secret_key: &str,
        prev_pk_script: Vec<u8>,
        index: usize,
    ) -> io::Result<()> {
        let z = self.sig_hash(prev_pk_script, index)?;
        let der = der_sign_with_priv_key(&z, secret_key)?;
        let pub_key = pub_key_from_priv_key(secret_key)?;

        let der_len = (der.len() + 1) as u8;
        let pub_key_len = pub_key.len() as u8;
        let script_sig = [&[der_len], &der[..], &[0x01], &[pub_key_len], &pub_key[..]].concat();

        // change script sig of index
        match self.tx_in {
            TxInputType::CoinBaseInput(_) => {} // we should never get here
            TxInputType::TxInput(ref mut inputs) => {
                inputs[index].script_bytes = script_sig.len() as u64;
                inputs[index].script_sig = script_sig;
            }
        }

        Ok(())
    }

    pub fn is_from_address(&self, address: &str) -> bool{
        match &self.tx_in {
            TxInputType::CoinBaseInput(_) => {}
            TxInputType::TxInput(tx_ins) => {
                for txin in tx_ins {
                    if txin.destined_from(address) {
                        return true;
                    }
                }
            }
        }
        false
    }


    pub fn is_destined_to_address(&self, address: &str) -> bool{
        for txout in &self.tx_out {
            if txout.destined_to(address) {
                return true;
            }
        }
        false
    }

    pub fn address_is_involved(&self, address: &str) -> bool {
        self.is_from_address(address) || self.is_destined_to_address(address)
    }

    pub fn transaction_info_for(&self, address: &str, utxo_set: &UtxoSet) -> TransactionDisplayInfo{
        /*
        if tx.is_from_address("myudL9LPYaJUDXWXGz5WC6RCdcTKCAWMUX"){
            let spent_value = tx.tx_in.get_spent_value("myudL9LPYaJUDXWXGz5WC6RCdcTKCAWMUX", self.utxo_set);
            let change_value = tx.tx_out.get_change_value_for("myudL9LPYaJUDXWXGz5WC6RCdcTKCAWMUX");
        } else if tx.is_destined_to_address("myudL9LPYaJUDXWXGz5WC6RCdcTKCAWMUX".to_string()){

            let change_value = tx.tx_out.get_change_value_for("myudL9LPYaJUDXWXGz5WC6RCdcTKCAWMUX".to_string());
        }
        */
         return TransactionDisplayInfo{
             role: TransactionRole::Receiver,
             date: "".to_string(),
             amount: 0,
             hash: HashId::new([0_u8;32]),
         }
    }
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

    fn generate_utxo_in(
        &self,
        utxo_set: &mut UtxoSet,
        origin: TransactionOrigin,
    ) -> io::Result<()> {
        if let TxInputType::TxInput(ref inputs) = self.tx_in {
            for input in inputs {
                let address = match input.get_address() {
                    Ok(a) => a,
                    _ => "no_address".to_string(),
                };

                let utxo_id = input.previous_output.hash;
                let index = input.previous_output.index;
                match utxo_set.set.get_mut(&address) {
                    Some(wallet) => {
                        wallet.add_spent(utxo_id, index, origin.clone());
                    }
                    None => {
                        let mut wallet = WalletUtxo::new();
                        wallet.add_spent(utxo_id, index, origin.clone());
                        utxo_set.set.insert(address, wallet);
                    }
                }
            }
        }

        Ok(())
    }

    fn generate_utxo_out(
        &self,
        utxo_set: &mut UtxoSet,
        origin: TransactionOrigin,
    ) -> io::Result<()> {
        let new_utxo_id = double_hash(&self.serialize()).to_byte_array();
        let new_utxo = Utxo::from_raw_transaction(self)?;
        for utxo_transaction in &new_utxo.transactions {
            let address = match utxo_transaction.get_address() {
                Ok(a) => a,
                _ => "no_address".to_string(),
            };

            match utxo_set.set.get_mut(&address) {
                Some(wallet) => {
                    wallet.add_utxo(new_utxo_id, utxo_transaction.clone(), origin.clone())
                }
                None => {
                    let mut wallet = WalletUtxo::new();
                    wallet.add_utxo(new_utxo_id, utxo_transaction.clone(), origin.clone());
                    utxo_set.set.insert(address, wallet);
                }
            }
        }

        Ok(())
    }

    pub fn generate_utxo(
        &self,
        utxo_set: &mut UtxoSet,
        origin: TransactionOrigin,
    ) -> io::Result<()> {
        self.generate_utxo_in(utxo_set, origin.clone())?;
        self.generate_utxo_out(utxo_set, origin)?;

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

impl Serialize for RawTransaction {
    fn deserialize(bytes: &[u8]) -> Result<Msg, std::io::Error> {
        let mut cursor = Cursor::new(bytes);
        let raw_transaction = RawTransaction::from_bytes(&mut cursor)?;
        Ok(Msg::Transaction(raw_transaction))
    }

    fn serialize(&self) -> io::Result<Vec<u8>> {
        Ok(self.serialize())
    }
}

#[cfg(test)]
mod tests {
    use crate::utility::_decode_hex;

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

    #[test]
    fn test_raw_transaction_deserial_and_serial() {
        let bytes = _decode_hex("01000000011acd5fe758ab56da34a0973c9c5dda0b63dcd79fe5860950813a366db1c92585010000006a4730440220046dc82c7c2e72665938c0aa7e10a135496d2467c2d1d105daa4ed1bab436898022064d9e36334d87c56454f7447c9da2c2eeb56cb77d3e9431feeac45649a23d9b901210387d7265c4973b153830aa72486d2488f964d194d2de869236fb87cc907d83971ffffffff0240420f00000000001976a9149144fda38182db2d26e5de88456accf241c898eb88aca0860100000000001976a9144a82aaa02eba3c31cd86ee83345c4f91986743fe88ac00000000").unwrap();
        let raw_transaction = RawTransaction::from_bytes(&mut Cursor::new(&bytes)).unwrap();
        let serialized_raw_transaction = raw_transaction.serialize();
        assert_eq!(bytes, serialized_raw_transaction);
    }

    #[test]
    fn test_raw_transaction_address_is_envolved() {
        let transaction_bytes = _decode_hex("0100000001881468a1a95473ed788c8a13bcdb7e524eac4f1088b1e2606ffb95492e239b10000000006a473044022021dc538aab629f2be56304937e796884356d1e79499150f5df03e8b8a545d17702205b76bda9c238035c907cbf6a39fa723d65f800ebb8082bdbb62d016d7937d990012102a953c8d6e15c569ea2192933593518566ca7f49b59b91561c01e30d55b0e1922ffffffff0210270000000000001976a9144a82aaa02eba3c31cd86ee83345c4f91986743fe88ac96051a00000000001976a914c9bc003bf72ebdc53a9572f7ea792ef49a2858d788ac00000000");
        let transaction =
            RawTransaction::from_bytes(&mut Cursor::new(&transaction_bytes.unwrap())).unwrap();
        let address = "myudL9LPYaJUDXWXGz5WC6RCdcTKCAWMUX";
        assert!(transaction.address_is_involved(address));
        assert!(!transaction.address_is_involved("foo"));
    }
}
