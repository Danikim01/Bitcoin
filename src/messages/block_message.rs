use super::Message;
use crate::interface::GtkMessage;
use crate::io::{self, Cursor};
use crate::merkle_tree::MerkleTree;
use crate::messages::{utility::*, BlockHeader, HashId, Hashable, Serialize};
use crate::network_controller::TransactionDisplayInfo;
use crate::raw_transaction::{RawTransaction, TransactionOrigin};
use crate::utility::double_hash;
use crate::utility::to_io_err;
use crate::utxo::UtxoSet;
use bitcoin_hashes::{sha256, Hash};
use chrono::Utc;
use gtk::glib::Sender;
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::{Read, Write};

pub type BlockSet = HashMap<HashId, Block>;

/// A struct that represents a block with a header and  a list of transactions.
#[derive(Debug, Clone)]
pub struct Block {
    pub block_header: BlockHeader,
    pub txn_count: usize,
    pub txns: Vec<RawTransaction>,
}

// https://developer.bitcoin.org/reference/block_chain.html#serialized-blocks
impl Serialize for Block {
    fn serialize(&self) -> io::Result<Vec<u8>> {
        let mut bytes: Vec<u8> = vec![];

        let header_bytes = self.block_header.serialize();
        bytes.extend(header_bytes);
        let txn_count_bytes = to_compact_size_bytes(self.txn_count as u64);
        bytes.extend(txn_count_bytes);

        for txn in self.txns.iter() {
            let txn_bytes = txn.serialize();
            bytes.extend(txn_bytes);
        }

        Ok(bytes)
    }

    fn deserialize(bytes: &[u8]) -> Result<Message, io::Error> {
        let mut cursor = Cursor::new(bytes);

        let block_header = BlockHeader::from_bytes(&mut cursor)?;
        let txn_count = read_from_varint(&mut cursor)?;

        let mut txns = vec![];

        let coinbase_transaction = RawTransaction::coinbase_from_bytes(&mut cursor)?;
        txns.push(coinbase_transaction);

        let other_txns = RawTransaction::vec_from_bytes(&mut cursor, txn_count as usize)?;
        txns.extend(other_txns);

        let block = Block {
            block_header,
            txn_count: txn_count as usize,
            txns,
        };

        Ok(Message::Block(block))
    }
}

impl Hashable for Block {
    fn hash(&self) -> HashId {
        self.block_header.hash()
    }
}

impl Block {
    fn hash_transactions(&self) -> Vec<sha256::Hash> {
        let mut txn_hashes: Vec<sha256::Hash> = vec![];
        self.txns.iter().for_each(|txn| {
            let txn_bytes = txn.serialize();
            let txn_hash = double_hash(&txn_bytes);
            txn_hashes.push(txn_hash);
        });
        txn_hashes
    }

    fn validate_merkle_root(&self) -> io::Result<()> {
        // hash all transactions in the block
        let txn_hashes = self.hash_transactions();

        // build merkle tree from transaction hashes
        let merkle_tree = MerkleTree::generate_from_hashes(txn_hashes); // clone txn_hashes if merkle proofing
        let root_hash = merkle_tree.get_root();

        match self.block_header.merkle_root_hash == HashId::new(root_hash.to_byte_array()) {
            true => {
                // println!("Merkle root is valid!");
                Ok(())
            }
            false => {
                println!("\x1b[93mMerkle root is invalid!\x1b[0m");
                Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Merkle root hash mismatch",
                ))
            }
        }
    }

    fn update_ui(
        ui_sender: Option<&Sender<GtkMessage>>,
        active_addr: Option<&str>,
        txn: &RawTransaction,
    ) -> io::Result<()> {
        if let Some(addr) = active_addr {
            if txn.address_is_involved(addr) {
                let transaction_info: TransactionDisplayInfo = txn.transaction_info_for(addr);
                if let Some(ui_sender) = ui_sender {
                    ui_sender
                        .send(GtkMessage::UpdateOverviewTransactions((
                            transaction_info,
                            TransactionOrigin::Block,
                        )))
                        .map_err(to_io_err)?
                }
            }
        }
        Ok(())
    }

    /// Validates the block by checking the proof of work, merkle root, and generates utxos from the transactions outputs.
    /// Adds to the utxo set only if the block is valid.
    pub fn validate(
        &self,
        utxo_set: &mut UtxoSet,
        ui_sender: Option<&Sender<GtkMessage>>,
        active_addr: Option<&str>,
    ) -> io::Result<()> {
        let mut utxo_set_snapshot = utxo_set.clone();

        for txn in self.txns.iter() {
            txn.generate_utxo(
                &mut utxo_set_snapshot,
                TransactionOrigin::Block,
                ui_sender,
                active_addr,
            )?;
        }

        self.block_header.validate_proof_of_work()?;
        self.validate_merkle_root()?;

        *utxo_set = utxo_set_snapshot;

        Ok(())
    }

    /// Validates the block by checking the proof of work, merkle root, and generates utxos from the transactions outputs.
    pub fn validate_from_backup(
        &self,
        utxo_set: &mut UtxoSet,
        ui_sender: Option<&Sender<GtkMessage>>,
        active_addr: Option<&str>,
    ) -> io::Result<()> {
        for txn in self.txns.iter() {
            txn.generate_utxo(utxo_set, TransactionOrigin::Block, ui_sender, active_addr)?;
            Self::update_ui(ui_sender, active_addr, txn)?;
        }

        self.block_header.validate_proof_of_work()?;
        self.validate_merkle_root()?;

        Ok(())
    }

    /// Reads all transactions made from the given address and returns them in a vector of TransactionDisplayInfo.
    pub fn read_transactions_from(&self, address: &str) -> Vec<TransactionDisplayInfo> {
        let mut transactions = vec![];

        for tx in &self.txns {
            if tx.address_is_involved("myudL9LPYaJUDXWXGz5WC6RCdcTKCAWMUX") {
                let transaction_info: TransactionDisplayInfo =
                    tx.transaction_info_for("myudL9LPYaJUDXWXGz5WC6RCdcTKCAWMUX");
                transactions.push(transaction_info);
            }
        }

        return transactions;
    }

    /// Reads all transactions in the file and returns them in a BlockSet.
    pub fn all_from_file(file_name: &str) -> io::Result<BlockSet> {
        let mut block_set: BlockSet = HashMap::new();

        match std::fs::read(file_name) {
            Ok(bytes) => {
                // create cursor to read bytes
                let mut cursor: Cursor<&[u8]> = Cursor::new(&bytes);
                let file_size = bytes.len() as u64;

                while cursor.position() < file_size {
                    // read block size
                    let block_size = read_from_varint(&mut cursor)?;
                    // create buffer of block size
                    let mut block_bytes = vec![0; block_size as usize];

                    // read block bytes
                    cursor.read_exact(&mut block_bytes)?;

                    // deserialize block
                    let block_msg = Block::deserialize(&block_bytes)?;

                    if let Message::Block(block) = block_msg {
                        block_set.insert(block.hash(), block);
                    }
                }
            }
            Err(e) => return Err(e),
        }

        Ok(block_set)
    }

    /// Serialize the block and then save it to the given file.
    pub fn save_to_file(&self, file_name: &str) -> io::Result<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .open(file_name)
            .map_err(to_io_err)?;

        let bytes = self.serialize()?;
        let msg_len = to_compact_size_bytes(bytes.len() as u64);
        let data = [msg_len, bytes].concat();

        file.write_all(&data)?;
        Ok(())
    }

    /// Returns the age of the block in days.
    pub fn get_days_old(&self) -> u64 {
        let current_time = Utc::now().timestamp();
        let block_time = self.block_header.timestamp as i64;
        let age = (current_time - block_time) / 86400;
        age as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::Block;
    use std::fs;

    #[test]
    fn test_read_serialized_block_from_bytes() -> io::Result<()> {
        // Needed to avoid github actions error
        let bytes = match fs::read("./tmp/block_message_payload.dat") {
            Ok(bytes) => bytes,
            Err(e) => {
                println!("Error reading file: {}", e);
                vec![]
            }
        };

        if !bytes.is_empty() {
            let message = Block::deserialize(&bytes).unwrap();
            let mut utxo_set: UtxoSet = UtxoSet::new();
            if let Message::Block(block) = message {
                block.validate(&mut utxo_set, None, None)?;
                assert_eq!(block.txn_count, block.txns.len());
            };
        }

        Ok(())
    }
}
