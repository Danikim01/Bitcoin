use super::Message;
use crate::interface::GtkMessage;
use crate::io::{self, Cursor};
use crate::messages::MerkleTree;
use crate::messages::{utility::*, BlockHeader, HashId, Hashable, Serialize};
use crate::raw_transaction::{RawTransaction, TransactionOrigin};
use crate::utility::double_hash;
use crate::utility::to_io_err;
use crate::utxo::UtxoSet;
use crate::wallet::Wallet;
use bitcoin_hashes::{sha256, Hash};
use gtk::glib::SyncSender;
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::{Read, Write};

pub type BlockSet = HashMap<HashId, Block>;

/// A struct that represents a block with a header and  a list of transactions.
#[derive(Debug, Clone)]
pub struct Block {
    pub header: BlockHeader,
    pub txn_count: usize,
    pub txns: Vec<RawTransaction>,
}

impl Block {
    pub fn new(header: BlockHeader, txn_count: usize, txns: Vec<RawTransaction>) -> Self {
        Self {
            header,
            txn_count,
            txns,
        }
    }

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

        match self.header.merkle_root_hash == HashId::new(root_hash.to_byte_array()) {
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

    // MAYBE REMOVE THIS? if you are seeing this outside of multi-wallet branch, remove it
    fn update_ui(// ui_sender: Option<&SyncSender<GtkMessage>>,
        // active_addr: Option<&str>,
        // txn: &RawTransaction,
        // timestamp: u32,
        // utxo_set: &mut UtxoSet,
    ) -> io::Result<()> {
        // if let Some(addr) = active_addr {
        //     if txn.address_is_involved(addr) {
        //         let transaction_info: TransactionDisplayInfo =
        //             txn.transaction_info_for(addr, timestamp, utxo_set);
        //         if let Some(ui_sender) = ui_sender {
        //             ui_sender
        //                 .send(GtkMessage::UpdateOverviewTransactions((
        //                     transaction_info,
        //                     TransactionOrigin::Block,
        //                 )))
        //                 .map_err(to_io_err)?
        //         }
        //     }
        // }
        Ok(())
    }

    /// Validates the block by checking the proof of work, merkle root.
    pub fn validate(&self) -> io::Result<()> {
        self.header.validate_proof_of_work()?;
        self.validate_merkle_root()?;
        Ok(())
    }

    fn update_wallets(
        &self,
        utxo_set: &mut UtxoSet,
        txn: &RawTransaction,
        wallets: &mut HashMap<String, Wallet>,
    ) -> io::Result<()> {
        for wallet in wallets.values_mut() {
            if txn.address_is_involved(vec![&wallet.address]) {
                let txn_info =
                    txn.transaction_info_for(&wallet.address, self.header.timestamp, utxo_set);
                wallet.update_history(txn_info);
            }
        }
        Ok(())
    }

    /// Adds to the utxo set
    pub fn expand_utxo(
        &self,
        utxo_set: &mut UtxoSet,
        ui_sender: Option<&SyncSender<GtkMessage>>,
        wallets: &mut HashMap<String, Wallet>,
        active_addr: Option<&str>,
    ) -> io::Result<()> {
        for txn in self.txns.iter() {
            txn.generate_utxo(utxo_set, TransactionOrigin::Block, ui_sender, active_addr)?;
            // let _ = Self::update_ui(ui_sender, active_addr, txn, self.header.timestamp, utxo_set); // disable this after wallets impl
            self.update_wallets(utxo_set, txn, wallets)?;
        }
        Ok(())
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
                    // read buffer of block size
                    let mut block_bytes = vec![0; block_size as usize];
                    cursor.read_exact(&mut block_bytes)?;
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
}

// https://developer.bitcoin.org/reference/block_chain.html#serialized-blocks
impl Serialize for Block {
    fn serialize(&self) -> io::Result<Vec<u8>> {
        let mut bytes: Vec<u8> = vec![];
        let header_bytes = self.header.serialize();
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
        let header = BlockHeader::from_bytes(&mut cursor)?;
        let txn_count = read_from_varint(&mut cursor)?;
        let mut txns = vec![];
        let coinbase_transaction = RawTransaction::coinbase_from_bytes(&mut cursor)?;
        txns.push(coinbase_transaction);
        let other_txns = RawTransaction::vec_from_bytes(&mut cursor, txn_count as usize)?;
        txns.extend(other_txns);
        let block = Block {
            header,
            txn_count: txn_count as usize,
            txns,
        };
        Ok(Message::Block(block))
    }
}

impl Hashable for Block {
    fn hash(&self) -> HashId {
        self.header.hash()
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
            if let Message::Block(block) = message {
                block.validate()?;
                assert_eq!(block.txn_count, block.txns.len());
            };
        }

        Ok(())
    }
}
