use crate::io::{self, Cursor};
use crate::messages::constants::commands::TX;
use crate::messages::constants::config::MAGIC;
use crate::messages::utility::{
    date_from_timestamp, read_from_varint, read_hash, to_compact_size_bytes, to_varint, StreamRead,
};
use crate::messages::{HashId, MessageHeader, Serialize};

use crate::utility::{double_hash, to_io_err};
use crate::utxo::{Utxo, UtxoSet, UtxoTransaction, WalletUtxo};
use bitcoin_hashes::Hash;
use std::io::{Error, Read};

use gtk::glib::SyncSender;
pub mod tx_input;
use tx_input::{CoinBaseInput, Outpoint, TxInput, TxInputType};
pub mod tx_output;
use crate::interface::components::overview_panel::{TransactionDisplayInfo, TransactionRole};
use crate::interface::GtkMessage;
use tx_output::TxOutput;

use super::messages::Message as Msg;

use secp256k1::{All, Message, PublicKey, Secp256k1, SecretKey};

const SIGHASH_ALL: u32 = 1;

fn read_coinbase_script(cursor: &mut Cursor<&[u8]>, count: usize) -> io::Result<Vec<u8>> {
    let mut array = vec![0_u8; count];
    cursor.read_exact(&mut array)?;
    Ok(array)
}

fn der_sign_with_priv_key(z: &[u8], private_key: &SecretKey) -> io::Result<Vec<u8>> {
    let message = &z;

    let secp: Secp256k1<secp256k1::All> = Secp256k1::gen_new();
    let message_slice: &[u8] = message;
    let message_slice = Message::from_slice(message_slice).map_err(to_io_err)?;
    let signature = secp.sign_ecdsa(&message_slice, private_key);

    // Convert the DER-encoded signature to bytes
    Ok(signature.serialize_der().to_vec())
}

/// A struct that represents a raw transaction (includes version, inputs, outputs, and locktime)
#[derive(Debug, Clone)]
pub struct RawTransaction {
    pub version: u32,
    pub tx_in_count: u64,
    pub tx_in: TxInputType,
    pub tx_out_count: u64,
    pub tx_out: Vec<TxOutput>,
    pub lock_time: u32,
}

/// Enum that represents the state of a transaction (pending or in a block)
#[derive(Debug, Clone, Copy, PartialEq)]
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

    /// Signs the input at the given index with the given private key
    pub fn sign_input(
        &mut self,
        secp: &Secp256k1<All>,
        secret_key: &SecretKey,
        prev_pk_script: Vec<u8>,
        index: usize,
    ) -> io::Result<()> {
        let z = self.sig_hash(prev_pk_script, index)?;
        let der = der_sign_with_priv_key(&z, secret_key)?;
        let pub_key = PublicKey::from_secret_key(secp, secret_key)
            .serialize()
            .to_vec();

        let der_len = (der.len() + 1) as u8;
        let pub_key_len = pub_key.len() as u8;
        let script_sig = [&[der_len], &der[..], &[0x01], &[pub_key_len], &pub_key].concat();

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

    /// Checks if any of the inputs is from the given address
    pub fn is_from_address(&self, address: &str) -> bool {
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

    /// Checks if any of the outputs is destined to the given address
    pub fn is_destined_to_address(&self, address: &str) -> bool {
        for txout in &self.tx_out {
            if txout.destined_to(address) {
                return true;
            }
        }
        false
    }

    /// Checks if the given address is involved in the transaction (either as input or output)
    pub fn address_is_involved(&self, address: &str) -> bool {
        if self.is_from_address(address) || self.is_destined_to_address(address) {
            return true;
        }

        false
    }

    fn get_input_value(&self, address: &str, utxoset: &UtxoSet, txin: &TxInput) -> u64 {
        let mut value = 0_u64;
        if !txin.destined_from(address) {
            return value;
        }

        let tx_previous = &txin.previous_output;
        if let Some(wallet) = utxoset.set.get(address) {
            if let Some(utxo_transaction) = wallet.utxos.get(&(tx_previous.hash, tx_previous.index))
            {
                value += utxo_transaction.value;
            }
        }
        value
    }

    fn get_total_input_value(&self, address: &str, utxoset: &UtxoSet) -> u64 {
        let mut total_value = 0_u64;
        match &self.tx_in {
            TxInputType::CoinBaseInput(_) => {}
            TxInputType::TxInput(tx_ins) => {
                for txin in tx_ins {
                    total_value += self.get_input_value(address, utxoset, txin);
                }
            }
        }
        total_value
    }

    /// Returns the total output value of the transaction (sum of all output values)
    pub fn get_total_output_value(&self) -> u64 {
        let mut total_value = 0_u64;
        for output in &self.tx_out {
            total_value += output.value;
        }

        total_value
    }

    /// Returns the change value for the given address (sum of all output values destined to the address)
    fn get_change_value_for(&self, address: &str) -> u64 {
        let mut total_value = 0_u64;
        for output in &self.tx_out {
            if output.destined_to(address) {
                total_value += output.value;
            }
        }
        total_value
    }

    /// Returns the hash of the transaction
    pub fn get_hash(&self) -> HashId {
        let hash = double_hash(&self.serialize());
        HashId::from_hash(hash)
    }

    /// Returns the transaction info for the given address
    pub fn transaction_info_for_pending(
        &self,
        address: &str,
        timestamp: u32,
        utxo_set: &mut UtxoSet,
    ) -> TransactionDisplayInfo {
        let mut role = TransactionRole::Sender;
        let mut spent_value = 0;

        if self.is_from_address(address) {
            spent_value = self.get_total_input_value(address, utxo_set);
        } else {
            role = TransactionRole::Receiver;
        }

        let change_value = self.get_change_value_for(address);

        TransactionDisplayInfo {
            role,
            origin: TransactionOrigin::Pending,
            date: date_from_timestamp(timestamp),
            amount: change_value as i64 - spent_value as i64,
            hash: self.get_hash(),
        }
    }

    /// Returns the transaction info for the given address
    pub fn transaction_info_for(
        &self,
        address: &str,
        timestamp: u32,
        utxo_set: &mut UtxoSet,
    ) -> TransactionDisplayInfo {
        let mut role = TransactionRole::Sender;
        let mut spent_value = 0;

        if self.is_from_address(address) {
            spent_value = self.get_total_input_value(address, utxo_set);
        } else {
            role = TransactionRole::Receiver;
        }

        let change_value = self.get_change_value_for(address);

        TransactionDisplayInfo {
            role,
            origin: TransactionOrigin::Block,
            date: date_from_timestamp(timestamp),
            amount: change_value as i64 - spent_value as i64,
            hash: self.get_hash(),
        }
    }

    /// Read the coinbase transaction from the given bytes and returns a RawTransaction with only the coinbase input and the outputs
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
                        wallet.add_spent(utxo_id, index, origin);
                    }
                    None => {
                        let mut wallet = WalletUtxo::new();
                        wallet.add_spent(utxo_id, index, origin);
                        utxo_set.set.insert(address, wallet);
                    }
                }
            }
        }

        Ok(())
    }

    fn get_utxo_addr(utxo_tx: &UtxoTransaction) -> String {
        match utxo_tx.get_address() {
            Ok(a) => a,
            _ => "no_address".to_string(),
        }
    }

    fn generate_utxo_out(
        &self,
        utxo_set: &mut UtxoSet,
        origin: TransactionOrigin,
        ui_sender: Option<&SyncSender<GtkMessage>>,
        active_addr: Option<&str>,
    ) -> io::Result<()> {
        let new_utxo_id = HashId::from_hash(double_hash(&self.serialize()));
        let new_utxo = Utxo::from_raw_transaction(self)?;
        for (index, utxo_transaction) in new_utxo.transactions.iter().enumerate() {
            let address = Self::get_utxo_addr(utxo_transaction);
            match utxo_set.set.get_mut(&address) {
                Some(wallet) => wallet.add_utxo(
                    new_utxo_id,
                    utxo_transaction.clone(),
                    origin,
                    index as u32,
                    ui_sender,
                    active_addr,
                )?,
                None => {
                    let mut wallet = WalletUtxo::new();
                    wallet.add_utxo(
                        new_utxo_id,
                        utxo_transaction.clone(),
                        origin,
                        index as u32,
                        None,
                        None,
                    )?;
                    utxo_set.set.insert(address, wallet);
                }
            }
        }

        Ok(())
    }

    /// Generates the UTXO for the given transaction and adds it to the given UTXO set
    pub fn generate_utxo(
        &self,
        utxo_set: &mut UtxoSet,
        origin: TransactionOrigin,
        ui_sender: Option<&SyncSender<GtkMessage>>,
        active_addr: Option<&str>,
    ) -> io::Result<()> {
        self.generate_utxo_in(utxo_set, origin)?;
        self.generate_utxo_out(utxo_set, origin, ui_sender, active_addr)?;

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

    /// Reads the transaction from the given bytes and returns a RawTransaction (supports segwit transactions BIP 144)
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

    /// Reads the given number of transactions from the given bytes and returns a vector of RawTransactions
    pub fn vec_from_bytes(cursor: &mut Cursor<&[u8]>, count: usize) -> Result<Vec<Self>, Error> {
        let mut raw_transactions = vec![];

        for _ in 1..count {
            let raw_transaction = RawTransaction::from_bytes(cursor)?;
            raw_transactions.push(raw_transaction);
        }

        Ok(raw_transactions)
    }

    /// Serializes the transaction into bytes
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

    /// build message to be broadcasted
    pub fn build_message(&self) -> io::Result<Vec<u8>> {
        let tx_hash = double_hash(&self.serialize());

        let payload = self.serialize();
        let mut bytes = MessageHeader::new(
            MAGIC,
            TX.to_string(),
            payload.len() as u32,
            [tx_hash[0], tx_hash[1], tx_hash[2], tx_hash[3]],
        )
        .serialize()?;

        bytes.extend(payload);
        Ok(bytes)
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
    use crate::utility::decode_hex;

    use super::*;
    use crate::utxo::UtxoTransaction;
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
        let bytes = decode_hex("01000000011acd5fe758ab56da34a0973c9c5dda0b63dcd79fe5860950813a366db1c92585010000006a4730440220046dc82c7c2e72665938c0aa7e10a135496d2467c2d1d105daa4ed1bab436898022064d9e36334d87c56454f7447c9da2c2eeb56cb77d3e9431feeac45649a23d9b901210387d7265c4973b153830aa72486d2488f964d194d2de869236fb87cc907d83971ffffffff0240420f00000000001976a9149144fda38182db2d26e5de88456accf241c898eb88aca0860100000000001976a9144a82aaa02eba3c31cd86ee83345c4f91986743fe88ac00000000").unwrap();
        let raw_transaction = RawTransaction::from_bytes(&mut Cursor::new(&bytes)).unwrap();
        let serialized_raw_transaction = raw_transaction.serialize();
        assert_eq!(bytes, serialized_raw_transaction);
    }

    #[test]
    fn test_raw_transaction_address_is_envolved() {
        let transaction_bytes = decode_hex("0100000001881468a1a95473ed788c8a13bcdb7e524eac4f1088b1e2606ffb95492e239b10000000006a473044022021dc538aab629f2be56304937e796884356d1e79499150f5df03e8b8a545d17702205b76bda9c238035c907cbf6a39fa723d65f800ebb8082bdbb62d016d7937d990012102a953c8d6e15c569ea2192933593518566ca7f49b59b91561c01e30d55b0e1922ffffffff0210270000000000001976a9144a82aaa02eba3c31cd86ee83345c4f91986743fe88ac96051a00000000001976a914c9bc003bf72ebdc53a9572f7ea792ef49a2858d788ac00000000");
        let transaction =
            RawTransaction::from_bytes(&mut Cursor::new(&transaction_bytes.unwrap())).unwrap();
        let address = "myudL9LPYaJUDXWXGz5WC6RCdcTKCAWMUX";
        assert!(transaction.address_is_involved(address));
        assert!(!transaction.address_is_involved("foo"));
    }

    #[test]
    fn test_raw_transaction_has_value_negative() {
        let transaction_bytes = decode_hex("01000000011ecd55d9f67f16ffdc7b572a1c8baa2b4acb5c45c672f74e498b792d09f856a4010000006b483045022100bb0a409aa0b0a276b5ec4473f5aa9d526eb2e9835916f6754f7f5a89725b7f0c02204d3b3b3fe8f8af9e8de983301dd6bb5637e03038d94cba670b40b1e9ca221b29012102a953c8d6e15c569ea2192933593518566ca7f49b59b91561c01e30d55b0e1922ffffffff0210270000000000001976a914c9bc003bf72ebdc53a9572f7ea792ef49a2858d788ac54121d00000000001976a914c9bc003bf72ebdc53a9572f7ea792ef49a2858d788ac00000000");
        let transaction =
            RawTransaction::from_bytes(&mut Cursor::new(&transaction_bytes.unwrap())).unwrap();
        let address = "myudL9LPYaJUDXWXGz5WC6RCdcTKCAWMUX";

        let txin = match transaction.tx_in.clone() {
            TxInputType::TxInput(txin) => txin[0].clone(),
            _ => panic!("not a TxInputType::TxInput"),
        };

        let previous_output = (txin.previous_output.hash, txin.previous_output.index);
        let utxo_tx = UtxoTransaction {
            index: txin.previous_output.index,
            value: 1925236,
            lock: vec![],
        };

        let mut wallet = WalletUtxo::new();
        wallet.utxos.insert(previous_output, utxo_tx);

        let mut utxo_set = UtxoSet::new();
        utxo_set
            .set
            .insert("myudL9LPYaJUDXWXGz5WC6RCdcTKCAWMUX".to_string(), wallet);

        let transaction_info = transaction.transaction_info_for(address, 0, &mut utxo_set);

        assert_eq!(transaction_info.amount, -10000);
    }
}
