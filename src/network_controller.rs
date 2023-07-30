use crate::config::Config;
use crate::interface::components::overview_panel::TransactionDisplayInfo;
use crate::interface::{GtkMessage, ModelRequest};
use crate::messages::block_header::HeaderSet;
use crate::messages::constants::config::{QUIET, VERBOSE};
use crate::messages::merkle_tree::MerkleProof;
use crate::messages::{
    Block, BlockHeader, GetData, GetHeader, HashId, Hashable, Headers, InvType, Inventory,
    MerkleTree, Message, Serialize,
};

use crate::node_controller::NodeController;
use crate::raw_transaction::{RawTransaction, TransactionOrigin};
use crate::utility::{decode_hex, double_hash, reverse_hex_str, to_io_err};
use crate::utxo::UtxoSet;
use crate::wallet::Wallet;
use bitcoin_hashes::{sha256, Hash};
use chrono::Utc;
use gtk::glib::SyncSender;
use std::collections::{hash_map::Entry::Occupied, hash_map::Entry::Vacant, HashMap};
use std::io;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, TcpListener};
use std::sync::{
    mpsc::{self, Receiver},
    Arc, RwLock, RwLockReadGuard,
};
use std::thread::{self, JoinHandle};

use crate::interface::components::table::{
    table_data_from_blocks, table_data_from_headers, table_data_from_tx, GtkTable, GtkTableData,
};
use crate::interface::{components::send_panel::TransactionInfo, update_ui_progress_bar};
use crate::node::Node;

pub type BlockSet = HashMap<HashId, Block>;

/// Structs of the network controller (main controller of the program)
pub struct NetworkController {
    headers: HeaderSet,
    tallest_header: BlockHeader,
    tallest_block: BlockHeader,
    valid_blocks: BlockSet,   // valid blocks downloaded so far
    blocks_on_hold: BlockSet, // downloaded blocks for which we don't have the previous block
    pending_blocks: HashMap<HashId, Vec<HashId>>, // blocks which haven't arrived, and the blocks which come immediately after them
    utxo_set: UtxoSet,
    nodes: NodeController,
    ui_sender: SyncSender<GtkMessage>,
    active_wallet: String,
    wallets: HashMap<String, Wallet>, // key is address of the wallet
    tx_read: HashMap<HashId, ()>,
}

impl NetworkController {
    /// Creates a new network controller from the given sender and writer
    pub fn new(
        ui_sender: SyncSender<GtkMessage>,
        writer_end: mpsc::SyncSender<(SocketAddr, Message)>,
        config: Config,
    ) -> Result<Self, io::Error> {
        let genesis_header = BlockHeader::genesis(config.get_genesis());
        let (active_wallet, wallets) = Wallet::init_all(&config, Some(&ui_sender))?;
        Ok(Self {
            headers: HeaderSet::with(genesis_header.hash, genesis_header),
            tallest_header: genesis_header,
            tallest_block: genesis_header,
            valid_blocks: BlockSet::new(),
            blocks_on_hold: BlockSet::new(),
            pending_blocks: HashMap::new(),
            utxo_set: UtxoSet::new(),
            nodes: NodeController::connect_to_peers(writer_end, ui_sender.clone(), config)?,
            active_wallet,
            wallets,
            ui_sender,
            tx_read: HashMap::new(),
        })
    }

    fn update_ui_poi_result(&self, proof: MerkleProof, root_from_proof: sha256::Hash) {
        let root_from_proof_str = format!("{:?}", root_from_proof);

        let result_str = format!(
            "{:?}\n\nMerkle root generated from poi: {:?}",
            proof,
            &reverse_hex_str(&root_from_proof_str)[..root_from_proof_str.len() - 2]
        );

        _ = self.ui_sender.send(GtkMessage::UpdatePoiResult(result_str));
    }

    fn handle_getheaders_message(&self, getheaders_message: GetHeader) -> Option<Headers> {
        let last_known_hash = match getheaders_message.block_header_hashes.first() {
            Some(hash) => *hash,
            None => return None,
        };

        let max_blocks = 2000;
        let mut next_block_header = match self.headers.get_next_header(&last_known_hash) {
            Some(header) => *header,
            None => return None,
        };
        let mut headers: Vec<BlockHeader> = vec![next_block_header];
        for _ in 1..max_blocks {
            next_block_header = match self.headers.get_next_header(&next_block_header.hash) {
                Some(header) => *header,
                None => break,
            };
            headers.push(next_block_header);
            if next_block_header.hash == getheaders_message.stop_hash {
                break;
            }
        }
        Some(Headers::new(headers.len(), headers))
    }

    fn update_ui_progress(&self, msg: Option<&str>, progress: f64) {
        _ = update_ui_progress_bar(&self.ui_sender, msg, progress);
    }

    fn update_ui_table(&self, table: GtkTable, data: GtkTableData) -> io::Result<()> {
        self.ui_sender
            .send(GtkMessage::UpdateTable((table, data)))
            .map_err(to_io_err)?;

        Ok(())
    }

    fn update_ui_balance(&self) -> io::Result<()> {
        let (balance, pending) = self.read_active_wallet_balance()?;
        self.ui_sender
            .send(GtkMessage::UpdateBalance((balance, pending)))
            .map_err(to_io_err)
    }

    fn notify_ui_message(&self, t: gtk::MessageType, title: &str, msg: &str) -> io::Result<()> {
        self.ui_sender
            .send(GtkMessage::CreateNotification((
                t,
                title.to_string(),
                msg.to_string(),
            )))
            .map_err(to_io_err)
    }

    fn read_active_wallet_balance(&self) -> io::Result<(u64, u64)> {
        let balance = self.utxo_set.get_wallet_balance(&self.active_wallet);
        let pending_balance = self
            .utxo_set
            .get_pending_wallet_balance(&self.active_wallet);

        Ok((balance, pending_balance))
    }

    fn get_best_headers(&self, amount: usize) -> Vec<&BlockHeader> {
        let mut best_headers = vec![];
        let mut current_header = &self.tallest_header;
        for _ in 0..amount {
            best_headers.push(current_header);
            current_header = match self.headers.get(&current_header.prev_block_hash) {
                Some(header) => header,
                None => break,
            }
        }
        best_headers.reverse();
        best_headers
    }

    fn get_best_blocks(&self, amount: usize) -> Vec<&Block> {
        let mut best_blocks = vec![];
        let mut current_block = match self.valid_blocks.get(&self.tallest_block.hash) {
            Some(block) => block,
            None => return best_blocks, // there's no downloaded blocks so far
        };
        for _ in 0..amount {
            best_blocks.push(current_block);
            current_block = match self.valid_blocks.get(&current_block.header.prev_block_hash) {
                Some(header) => header,
                None => break,
            }
        }
        best_blocks.reverse();
        best_blocks
    }

    fn read_backup_block(&mut self, block: Block, config: &Config) {
        if self
            .valid_blocks
            .contains_key(&block.header.prev_block_hash)
        {
            let hash = block.hash();
            self.blocks_on_hold.insert(hash, block);
            self.add_to_valid_blocks(hash, config);
        } else {
            self.put_block_on_hold(block);
        }
    }

    fn _add_to_valid_blocks(&mut self, mut block: Block, config: &Config) {
        _ = block.expand_utxo(
            &mut self.utxo_set,
            Some(&self.ui_sender),
            &mut self.wallets,
            Some(&self.active_wallet),
        );

        _ = self.update_ui_balance();

        // get real height of the block
        block.header.height = match self.valid_blocks.get(&block.header.prev_block_hash) {
            Some(prev_block) => prev_block.header.height + 1,
            _ => 0, // this will never happen
        };

        // update progress bar
        let pseudo_genesis_timestamp = config.get_start_timestamp();
        let progress = (block.header.timestamp - pseudo_genesis_timestamp) as f64
            / (Utc::now().timestamp() - pseudo_genesis_timestamp as i64) as f64;
        let msg = format!("Received block {}", block.header.height);
        _ = update_ui_progress_bar(&self.ui_sender, Some(&msg), progress);

        if block.header.height > self.tallest_block.height {
            self.tallest_block = block.header;
        }
        self.valid_blocks.insert(block.hash(), block);
    }

    fn add_to_valid_blocks(&mut self, block_id: HashId, config: &Config) {
        // if there where blocks on hold waiting for this one, validate them
        let mut blocks_not_on_hold: Vec<HashId> = vec![block_id];
        while let Some(block_id) = blocks_not_on_hold.pop() {
            if let Some(block) = self.blocks_on_hold.remove(&block_id) {
                self._add_to_valid_blocks(block, config);
                if let Some(mut unblocked_blocks) = self.pending_blocks.remove(&block_id) {
                    blocks_not_on_hold.append(&mut unblocked_blocks);
                }
            }
        }
    }

    fn put_block_on_hold(&mut self, block: Block) {
        // add to pending blocks the previous block, mark this block as blocked by the previous one
        match self.pending_blocks.entry(block.header.prev_block_hash) {
            Vacant(entry) => {
                entry.insert(vec![block.hash()]);
            }
            Occupied(mut entry) => entry.get_mut().push(block.hash()),
        }
        self.blocks_on_hold.insert(block.hash(), block);
    }

    fn request_blocks_evenly(&mut self, headers: &mut Headers, config: &Config) -> io::Result<()> {
        let chunks = headers.block_headers.chunks(20); // request 20 blocks at a time
        for chunk in chunks {
            let get_data = GetData::from_inv(chunk.len(), chunk.to_vec());
            self.nodes.send_to_all(&get_data.serialize()?, config)?;
        }
        config.log("Requesting blocks, sent GetData message.", VERBOSE);
        Ok(())
    }

    /// requests block for headers after given timestamp
    fn request_blocks(&mut self, mut headers: Headers, config: &Config) -> io::Result<()> {
        if headers.count == 0 {
            return Ok(());
        }

        self.request_blocks_evenly(&mut headers, config)
    }

    fn get_downloadable_bck_headers(&mut self, headers: Headers) -> Headers {
        // since every block needs to come after a valid block, create a "pseudo genesis" validated block
        if headers.block_headers.is_empty() {
            return headers;
        }

        // since every block needs to come after a valid block, create a "pseudo genesis" validated block
        let first_downloadable_header = headers.block_headers[0];
        if let Some(previous_header) = self.headers.get(&first_downloadable_header.prev_block_hash)
        {
            // this never fails
            let pseudo_genesis_block = Block::new(*previous_header, 0, vec![]);
            self.valid_blocks
                .insert(pseudo_genesis_block.hash(), pseudo_genesis_block);
        }
        headers
    }

    fn read_backup_headers(&mut self, mut headers: Headers, config: &Config) -> Headers {
        // save new headers to hashmap and backup file
        let mut new_headers = vec![];
        for mut header in headers.block_headers {
            match self.headers.get(&header.prev_block_hash) {
                Some(parent_header) => {
                    header.height = parent_header.height + 1;
                }
                None => continue, // ignore header if prev_header is unknown
            }
            self.headers.insert(header.hash(), header);
            //self.set_next_block_hash_for_blockheaders();
            new_headers.push(header);
            if header.height > self.tallest_header.height {
                self.tallest_header = header;
                self.update_best_header_chain();
            }
        }

        config.log(
            &format!(
                "Read backup headers. New header count: {:?}",
                self.headers.len() - 1
            ),
            VERBOSE,
        );
        // request blocks mined after given date
        headers = Headers::new(new_headers.len(), new_headers);
        headers.trim_timestamp(config.get_start_timestamp());

        self.get_downloadable_bck_headers(headers)
    }

    fn try_request_trimmed_blocks(
        &mut self,
        mut headers: Headers,
        config: &Config,
    ) -> io::Result<()> {
        headers.trim_timestamp(config.get_start_timestamp());
        if headers.block_headers.is_empty() {
            return Ok(());
        }

        // since every block needs to come after a valid block, create a "pseudo genesis" validated block
        let first_downloadable_header = headers.block_headers[0];
        if let Some(previous_header) = self.headers.get(&first_downloadable_header.prev_block_hash)
        {
            // this never fails
            let pseudo_genesis_block = Block::new(*previous_header, 0, vec![]);
            self.valid_blocks
                .insert(pseudo_genesis_block.hash(), pseudo_genesis_block);
        }
        self.request_blocks(headers, config)
    }

    fn request_headers(&mut self, header_hash: HashId, config: &Config) -> io::Result<()> {
        let getheader_message = GetHeader::from_last_header(header_hash);
        self.nodes
            .send_to_all(&getheader_message.serialize()?, config)?;
        Ok(())
    }

    fn read_pending_tx(&mut self, transaction: RawTransaction) -> io::Result<()> {
        let tx_hash: HashId = transaction.get_hash();
        if self.tx_read.contains_key(&tx_hash) {
            return Ok(());
        }

        transaction.generate_utxo(
            &mut self.utxo_set,
            TransactionOrigin::Pending,
            Some(&self.ui_sender),
            Some(&self.active_wallet),
        )?;

        let data = table_data_from_tx(&transaction);
        self.update_ui_table(GtkTable::Transactions, data)?;

        // have to check for each wallet separately
        for w in self.wallets.iter_mut() {
            let (address, wallet) = w;
            if transaction.address_is_involved(address) {
                let tx_info = transaction.transaction_info_for_pending(
                    address,
                    Utc::now().timestamp() as u32,
                    &mut self.utxo_set,
                );
                wallet.update_history(tx_info);
            }
        }

        self.tx_read.insert(tx_hash, ());
        Ok(())
    }

    /// Generates a transaction and broadcasts it to all peers given the transaction details
    pub fn generate_transaction(
        &mut self,
        details: TransactionInfo,
        config: &Config,
    ) -> io::Result<()> {
        let wallet = match self.wallets.get_mut(&self.active_wallet) {
            Some(w) => w,
            None => return Err(io::Error::new(io::ErrorKind::Other, "Wallet not found")),
        };

        let tx = wallet.generate_transaction(&mut self.utxo_set, details);

        match tx {
            Ok(tx) => {
                let tx_hash = double_hash(&tx.serialize());
                let bytes = tx.build_message()?;
                self.nodes.send_to_all(&bytes, config)?;

                self.read_pending_tx(tx)?;
                self.notify_ui_message(
                    gtk::MessageType::Info,
                    "Transaction broadcasted",
                    &format!("Transaction hash: {}", HashId::from_hash(tx_hash)),
                )
            }
            Err(e) => self.notify_ui_message(
                gtk::MessageType::Error,
                "Failed broadcasting transaction",
                &format!("{}", e),
            ),
        }
    }

    /// Gets the proof of inclusion for a transaction given the block hash and transaction hash
    pub fn get_proof_of_inclusion(&self, block_hash: String, tx_hash: String) -> io::Result<()> {
        let block_hashid: HashId = match block_hash.parse() {
            Ok(hash) => hash,
            Err(_) => {
                return self.notify_ui_message(
                    gtk::MessageType::Error,
                    "Invalid block hash",
                    "Invalid block hash.",
                )
            }
        };
        let block = match self.valid_blocks.get(&block_hashid) {
            Some(block) => block,
            None => {
                return self.notify_ui_message(
                    gtk::MessageType::Error,
                    "Block not found",
                    "Block not found in blockchain.",
                )
            }
        };

        let block_tx_hashes = block.hash_transactions();
        let merkle_tree = MerkleTree::generate_from_hashes(block_tx_hashes);
        let dhx = decode_hex(&reverse_hex_str(&tx_hash)).map_err(to_io_err)?;
        let tx_hashed = sha256::Hash::from_slice(&dhx).map_err(to_io_err)?;
        let proof = merkle_tree.generate_proof(tx_hashed)?;
        let root_from_proof = proof.generate_merkle_root();

        self.update_ui_poi_result(proof, root_from_proof);

        Ok(())
    }

    fn update_best_header_chain(&mut self) {
        let mut current_header_hash = self.tallest_header.hash;
        let mut prev_header_hash = self.tallest_header.prev_block_hash;
        loop {
            let previous_header = match self.headers.get_mut(&prev_header_hash) {
                Some(previous_header) => previous_header,
                None => return, // this will only happen when the current header is the genesis
            };

            // update previous in loop, until the previous' next is the current
            match previous_header.next_block_hash {
                Some(next_hash) if next_hash == current_header_hash => break,
                _ => previous_header.next_block_hash = Some(current_header_hash),
            }

            // set values for next iteration
            current_header_hash = prev_header_hash;
            let current_header = match self.headers.get(&prev_header_hash) {
                Some(header) => header,
                None => return, // will never happen
            };
            prev_header_hash = current_header.prev_block_hash;
        }
    }

    /// Starts the sync process by requesting headers from all peers from the last known header (or genesis block) to the current time
    /// If a backup file is found, it will read the blocks and headers from the backup file
    pub fn start_sync(&mut self, config: &Config) -> io::Result<()> {
        let mut downloadable_headers = Headers::default();
        // attempt to read headers from backup file
        self.update_ui_progress(Some("Reading backup files..."), 0.0);
        if let Ok(headers) = Headers::from_file(config.get_headers_file()) {
            self.update_ui_progress(Some("Reading headers from backup file..."), 0.0);
            downloadable_headers = self.read_backup_headers(headers, config);
            update_ui_progress_bar(
                &self.ui_sender,
                Some("Finished reading headers from backup file."),
                1.0,
            )?;
        }

        // attempt to read blocks from backup file
        if let Ok(blocks) = Block::all_from_file(config.get_blocks_file()) {
            self.update_ui_progress(Some("Found blocks backup file, reading blocks..."), 0.0);
            for (_, block) in blocks.into_iter() {
                self.read_backup_block(block, config);
            }
            update_ui_progress_bar(
                &self.ui_sender,
                Some("Finished reading blocks from backup file."),
                1.0,
            )?;
        }
        // Finally, catch up to blockchain doing IBD
        let mut missing_blocks: Vec<BlockHeader> = vec![];
        for header in downloadable_headers.block_headers {
            if !self.valid_blocks.contains_key(&header.hash())
                && !self.blocks_on_hold.contains_key(&header.hash())
            {
                missing_blocks.push(header);
            }
        }
        self.request_blocks(Headers::new(missing_blocks.len(), missing_blocks), config)?;
        self.request_headers(self.tallest_header.hash(), config)?;
        Ok(())
    }
}

/// OuterNetworkController is a wrapper around the inner NetworkController in order to allow for safe multithreading
pub struct OuterNetworkController {
    inner: Arc<RwLock<NetworkController>>,
    ui_sender: SyncSender<GtkMessage>,
    writer_chanel: mpsc::SyncSender<(SocketAddr, Message)>,
}

impl OuterNetworkController {
    /// Creates a new OuterNetworkController given a ui_sender and a writer
    pub fn new(
        ui_sender: SyncSender<GtkMessage>,
        writer_end: mpsc::SyncSender<(SocketAddr, Message)>,
        config: Config,
    ) -> Result<Self, io::Error> {
        let inner = Arc::new(RwLock::new(NetworkController::new(
            ui_sender.clone(),
            writer_end.clone(),
            config,
        )?));
        Ok(Self {
            inner,
            ui_sender,
            writer_chanel: writer_end,
        })
    }

    fn update_ui_headers_periodically(
        inner: &RwLockReadGuard<'_, NetworkController>,
        ui_sender: &SyncSender<GtkMessage>,
        tallest_header_hash: &mut HashId,
        amount: usize,
    ) {
        let headers: Vec<&BlockHeader> = inner.get_best_headers(amount);
        if inner.tallest_header.hash() != *tallest_header_hash {
            *tallest_header_hash = inner.tallest_header.hash();
            let data = table_data_from_headers(headers.clone());
            _ = ui_sender
                .send(GtkMessage::UpdateTable((GtkTable::Headers, data)))
                .map_err(to_io_err);
        }
    }

    fn update_ui_blocks_periodically(
        inner: &RwLockReadGuard<'_, NetworkController>,
        ui_sender: &SyncSender<GtkMessage>,
        tallest_block_hash: &mut HashId,
        amount: usize,
    ) {
        if inner.tallest_block.hash != *tallest_block_hash {
            *tallest_block_hash = inner.tallest_block.hash;
            let blocks = inner.get_best_blocks(amount);
            let data = table_data_from_blocks(blocks);
            _ = ui_sender.send(GtkMessage::UpdateTable((GtkTable::Blocks, data)));
        }
    }

    fn update_ui_overview_tx_periodically(
        inner: &RwLockReadGuard<'_, NetworkController>,
        ui_sender: &SyncSender<GtkMessage>,
        txs_on_overview: &mut Vec<TransactionDisplayInfo>,
    ) {
        let curr_active_wallet = inner.active_wallet.clone();
        _ = inner.update_ui_balance();
        if let Some(wallet) = inner.wallets.get(&curr_active_wallet) {
            let transactions = wallet.get_last_n_transactions(20);
            if transactions != *txs_on_overview {
                *txs_on_overview = transactions.clone();
                _ = ui_sender.send(GtkMessage::UpdateOverviewTransactions(transactions));
            }
        }
    }

    fn update_ui_data_periodically(&self) -> io::Result<()> {
        let inner = self.inner.clone();
        let ui_sender: SyncSender<GtkMessage> = self.ui_sender.clone();
        thread::spawn(move || -> io::Result<()> {
            let mut tallest_header_hash = HashId::default();
            let mut tallest_block_hash = HashId::default();
            let mut txs_on_overview: Vec<TransactionDisplayInfo> = Vec::new();
            loop {
                thread::sleep(std::time::Duration::from_secs(10));
                let inner: RwLockReadGuard<'_, NetworkController> =
                    inner.read().map_err(to_io_err)?;
                Self::update_ui_headers_periodically(
                    &inner,
                    &ui_sender,
                    &mut tallest_header_hash,
                    100,
                );
                Self::update_ui_blocks_periodically(
                    &inner,
                    &ui_sender,
                    &mut tallest_block_hash,
                    100,
                );
                Self::update_ui_overview_tx_periodically(&inner, &ui_sender, &mut txs_on_overview)
            }
        });
        Ok(())
    }

    fn handle_ui_change_active_wallet(
        t_inner: Arc<RwLock<NetworkController>>,
        wallet: String,
    ) -> io::Result<()> {
        let mut inner_lock = t_inner.write().map_err(to_io_err)?;
        inner_lock.active_wallet = wallet;

        let curr_active_wallet = inner_lock.active_wallet.clone();
        inner_lock.update_ui_balance()?;

        if let Some(wallet) = inner_lock.wallets.get(&curr_active_wallet) {
            let transactions = wallet.get_last_n_transactions(20);
            _ = inner_lock
                .ui_sender
                .send(GtkMessage::UpdateOverviewTransactions(transactions));
        }

        Ok(())
    }

    fn handle_ui_generate_transaction(
        t_inner: Arc<RwLock<NetworkController>>,
        transaction_info: TransactionInfo,
        config: Config,
    ) -> io::Result<()> {
        let mut inner_lock = t_inner.write().map_err(to_io_err)?;
        inner_lock.generate_transaction(transaction_info, &config)
    }

    fn handle_ui_get_poi(
        t_inner: Arc<RwLock<NetworkController>>,
        block_hash: String,
        tx_hash: String,
    ) -> io::Result<()> {
        let inner_lock = t_inner.read().map_err(to_io_err)?;
        inner_lock.get_proof_of_inclusion(block_hash, tx_hash)
    }

    fn recv_ui_messages(
        &self,
        ui_receiver: Receiver<ModelRequest>,
        config: Config,
    ) -> io::Result<()> {
        let inner = self.inner.clone();
        thread::spawn(move || -> io::Result<()> {
            loop {
                let t_inner: Arc<RwLock<NetworkController>> = inner.clone();
                match ui_receiver.recv().map_err(to_io_err)? {
                    ModelRequest::GenerateTransaction(transaction_info) => {
                        Self::handle_ui_generate_transaction(
                            t_inner,
                            transaction_info,
                            config.clone(),
                        )
                    }
                    ModelRequest::ChangeActiveWallet(wallet) => {
                        Self::handle_ui_change_active_wallet(t_inner, wallet)
                    }
                    ModelRequest::GetPoi(block_hash, tx_hash) => {
                        _ = Self::handle_ui_get_poi(t_inner, block_hash, tx_hash);
                        Ok(())
                    }
                }?;
            }
        });
        Ok(())
    }

    fn handle_node_block_message(
        t_inner: Arc<RwLock<NetworkController>>,
        mut block: Block,
        config: &Config,
    ) -> io::Result<()> {
        let inner_read = t_inner.read().map_err(to_io_err)?;
        if inner_read.valid_blocks.contains_key(&block.hash())
            || inner_read.blocks_on_hold.contains_key(&block.hash())
        {
            return Ok(());
        }
        if block.validate().is_err() {
            return Ok(());
        }
        block.save_to_file(config.get_blocks_file())?;
        drop(inner_read);

        let mut inner_write = t_inner.write().map_err(to_io_err)?;
        if let Some(previous_block) = inner_write.valid_blocks.get(&block.header.prev_block_hash) {
            block.header.height = previous_block.header.height + 1;
            if let Vacant(entry) = inner_write.headers.entry(block.hash()) {
                entry.insert(block.header);
                if block.header.height > inner_write.tallest_header.height {
                    inner_write.tallest_header = block.header;
                    inner_write.update_best_header_chain();
                }
            }
            let block_hash = block.hash();
            // add to on-hold and then validate as many on-hold blocks as possible
            inner_write.blocks_on_hold.insert(block_hash, block);
            inner_write.add_to_valid_blocks(block_hash, config);
        } else {
            inner_write.put_block_on_hold(block);
        }
        Ok(())
    }

    fn handle_headers_message_info(
        config: &Config,
        inner_read: RwLockReadGuard<'_, NetworkController>,
        ui_sender: &SyncSender<GtkMessage>,
    ) -> io::Result<()> {
        config.log(
            &format!(
                "Read headers. New header count: {:?}",
                inner_read.headers.len()
            ),
            VERBOSE,
        );

        let msg = format!(
            "Read headers. New header count: {:?}",
            inner_read.headers.len()
        );
        let most_recent_timestamp = inner_read.tallest_header.timestamp as i64;

        let genesis_block_timestamp = 1231006500; // 2009-01-03T18:15Z
        let progress = (most_recent_timestamp - genesis_block_timestamp) as f64
            / (Utc::now().timestamp() - genesis_block_timestamp) as f64;
        _ = update_ui_progress_bar(ui_sender, Some(&msg), progress);
        Ok(())
    }

    fn try_request_trimmed_blocks(
        t_inner: Arc<RwLock<NetworkController>>,
        new_headers: Vec<BlockHeader>,
        config: &Config,
    ) -> io::Result<()> {
        let headers = Headers::new(new_headers.len(), new_headers);
        let mut inner_write = t_inner.write().map_err(to_io_err)?;
        inner_write.try_request_trimmed_blocks(headers, config)
    }

    fn handle_node_headers_message(
        t_inner: Arc<RwLock<NetworkController>>,
        headers: Headers,
        config: &Config,
        ui_sender: &SyncSender<GtkMessage>,
    ) -> io::Result<()> {
        let mut inner_read: RwLockReadGuard<'_, NetworkController> =
            t_inner.read().map_err(to_io_err)?;
        let prev_header_count = inner_read.headers.len();
        // save new headers to hashmap and backup file
        let mut new_headers: Vec<BlockHeader> = vec![];
        for mut header in headers.block_headers {
            if inner_read.headers.contains_key(&header.hash()) {
                continue;
            }
            match inner_read.headers.get(&header.prev_block_hash) {
                Some(parent_header) => {
                    header.height = parent_header.height + 1;
                }
                None => continue, // ignore header if prev_header is unknown
            }
            let hash = header.hash();
            drop(inner_read);
            let mut inner_write = t_inner.write().map_err(to_io_err)?;
            inner_write.headers.insert(hash, header);
            header.save_to_file(config.get_headers_file())?;
            new_headers.push(header);
            if header.height > inner_write.tallest_header.height {
                inner_write.tallest_header = header
            }
            drop(inner_write);
            inner_read = t_inner.read().map_err(to_io_err)?;
        }
        if prev_header_count == inner_read.headers.len() {
            return Ok(());
        }
        _ = Self::handle_headers_message_info(config, inner_read, ui_sender);
        let mut inner_write = t_inner.write().map_err(to_io_err)?;
        inner_write.update_best_header_chain();
        drop(inner_write);

        // request blocks mined after given date
        Self::try_request_trimmed_blocks(t_inner, new_headers, config)
    }

    fn handle_node_inv_message(
        t_inner: Arc<RwLock<NetworkController>>,
        peer_addr: SocketAddr,
        inventories: Vec<Inventory>,
        config: &Config,
    ) -> io::Result<()> {
        let mut inner_write = t_inner.write().map_err(to_io_err)?;
        let mut blocks: Vec<Block> = Vec::new();
        for inventory in inventories {
            if inventory.inv_type == InvType::MSGBlock {
                let block = match inner_write.valid_blocks.get(&inventory.hash) {
                    Some(block) => block.clone(),
                    None => continue,
                };
                blocks.push(block);
            }
        }

        if blocks.is_empty() {
            return Ok(());
        }

        for block in blocks {
            let serialized_block = block.serialize_message()?;
            inner_write
                .nodes
                .send_to_specific(&peer_addr, &serialized_block, config)?;
        }
        Ok(())
    }

    fn handle_node_tx_message(
        t_inner: Arc<RwLock<NetworkController>>,
        tx: RawTransaction,
    ) -> io::Result<()> {
        t_inner.write().map_err(to_io_err)?.read_pending_tx(tx)
    }

    pub fn handle_node_getheaders_message(
        t_inner: Arc<RwLock<NetworkController>>,
        peer_addr: SocketAddr,
        getheaders: GetHeader,
        config: &Config,
    ) -> io::Result<()> {
        let mut inner_write = t_inner.write().map_err(to_io_err)?;
        if let Some(getheaders_message) = inner_write.handle_getheaders_message(getheaders) {
            _ = inner_write.nodes.send_to_specific(
                &peer_addr,
                &getheaders_message.serialize()?,
                config,
            );
        }
        Ok(())
    }

    pub fn handle_node_getdata_message(
        t_inner: Arc<RwLock<NetworkController>>,
        peer_addr: SocketAddr,
        getdata: GetData,
        config: &Config,
    ) -> io::Result<()> {
        OuterNetworkController::handle_node_inv_message(
            t_inner,
            peer_addr,
            getdata.inventory,
            config,
        )
    }

    fn recv_node_messages(
        &self,
        node_receiver: mpsc::Receiver<(SocketAddr, Message)>,
        config: Config,
    ) -> io::Result<JoinHandle<io::Result<()>>> {
        let inner = self.inner.clone();
        let ui_sender = self.ui_sender.clone();
        let handle = thread::spawn(move || -> io::Result<()> {
            loop {
                let t_inner: Arc<RwLock<NetworkController>> = inner.clone();
                if let Err(result) = match node_receiver.recv().map_err(to_io_err)? {
                    (_, Message::Headers(headers)) => {
                        Self::handle_node_headers_message(t_inner, headers, &config, &ui_sender)
                    }
                    (peer_addr, Message::GetHeader(get_headers)) => {
                        Self::handle_node_getheaders_message(
                            t_inner,
                            peer_addr,
                            get_headers,
                            &config,
                        )
                    }
                    (_, Message::Block(block)) => {
                        Self::handle_node_block_message(t_inner, block, &config)
                    }
                    (peer_addr, Message::_GetData(get_data)) => {
                        Self::handle_node_getdata_message(t_inner, peer_addr, get_data, &config)
                    }
                    (peer_addr, Message::Inv(inventories)) => {
                        Self::handle_node_inv_message(t_inner, peer_addr, inventories, &config)
                    }
                    (_, Message::Transaction(tx)) => Self::handle_node_tx_message(t_inner, tx),
                    _ => Ok(()), // unexpected messages were already filtered by node listeners
                } {
                    config.log(&format!("Received unhandled error: {:?}", result), QUIET);
                    return Err(result);
                }
            }
        });
        Ok(handle)
    }

    fn listen_for_nodes(&self, config: Config) -> io::Result<()> {
        let inner = self.inner.clone();
        let listener = match TcpListener::bind(SocketAddrV4::new(
            Ipv4Addr::LOCALHOST,
            config.get_listening_port(),
        )) {
            Ok(listener) => listener,
            Err(e) => {
                eprintln!("Ignoring Error: {:?}", e);
                return Ok(());
            }
        };

        let ui_sender = self.ui_sender.clone();
        let writer_channel = self.writer_chanel.clone();

        thread::spawn(move || -> io::Result<()> {
            for stream in listener.incoming() {
                match stream {
                    Ok(mut stream) => {
                        let ui_sender = ui_sender.clone();
                        Node::inverse_handshake(&mut stream).unwrap();
                        let node =
                            Node::spawn(stream, writer_channel.clone(), ui_sender, config.clone())?;
                        // Network Controller should add the new node
                        inner.write().map_err(to_io_err)?.nodes.add_node(node);
                    }
                    Err(e) => {
                        println!("Error: {}", e);
                    }
                }
            }

            Ok(())
        });
        Ok(())
    }

    fn sync(&self, config: Config) -> io::Result<()> {
        let inner = self.inner.clone();
        // let writer_chanel = self.writer_chanel.clone();
        self.listen_for_nodes(config.clone())?;
        thread::spawn(move || -> io::Result<()> {
            inner.write().map_err(to_io_err)?.start_sync(&config)?;
            Ok(())
        });
        Ok(())
    }

    /// Starts the sync process and requests headers periodically.
    pub fn start_sync(
        &self,
        node_receiver: mpsc::Receiver<(SocketAddr, Message)>,
        ui_receiver: Receiver<ModelRequest>,
        config: Config,
    ) -> io::Result<()> {
        self.recv_ui_messages(ui_receiver, config.clone())?;
        self.recv_node_messages(node_receiver, config.clone())?;
        self.update_ui_data_periodically()?;
        self.sync(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::constants::config::PORT;
    use crate::messages::version_message::Version;
    use crate::messages::{MessageHeader, VerAck};
    use gtk::glib;
    use std::io::Write;
    use std::net::TcpStream;
    use std::path::PathBuf;
    use std::sync::mpsc::SyncSender;

    #[test]
    #[ignore]
    fn test_handle_incoming_nodes() {
        let (ui_sender, _) = glib::MainContext::sync_channel(glib::PRIORITY_HIGH, 100);
        let (writer_end, _) = std::sync::mpsc::sync_channel::<(SocketAddr, Message)>(100);
        let writer_end: SyncSender<(SocketAddr, Message)> = writer_end;

        let config_file = "node.conf";
        let config_path: PathBuf = config_file.into(); // Convert &str to PathBuf

        let config = Config::from_file(config_path).unwrap();
        let outer_controller =
            OuterNetworkController::new(ui_sender, writer_end, config.clone()).unwrap();
        outer_controller.listen_for_nodes(config).unwrap();

        let mut socket = TcpStream::connect(SocketAddrV4::new(Ipv4Addr::LOCALHOST, PORT)).unwrap();

        //Envio un version
        let msg_version = Version::default_for_trans_addr(socket.peer_addr().unwrap());
        let payload = msg_version.serialize().unwrap();
        socket.write_all(&payload).unwrap();
        socket.flush().unwrap();

        //Leo el header del version message
        let version_header = MessageHeader::from_stream(&mut socket).unwrap();
        //Leo el payload del version message
        let _payload_data = version_header.read_payload(&mut socket).unwrap();

        //Recibo un verack message
        let _verack = VerAck::from_stream(&mut socket).unwrap();

        //Envio un verack message
        let payload = VerAck::new().serialize().unwrap();
        socket.write_all(&payload).unwrap();
        socket.flush().unwrap();
    }
}
