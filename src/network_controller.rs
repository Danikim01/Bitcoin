use crate::config::Config;
use crate::interface::{GtkMessage, ModelRequest};
use crate::messages::constants::{
    commands::TX,
    config::{MAGIC, QUIET, VERBOSE},
};
use crate::messages::{
    Block, BlockHeader, GetData, GetHeader, HashId, Hashable, Headers, InvType, Inventory, Message,
    MessageHeader, Serialize,
};
use crate::node_controller::NodeController;
use crate::raw_transaction::{RawTransaction, TransactionOrigin};
use crate::utility::{double_hash, to_io_err};
use crate::utxo::UtxoSet;
use crate::wallet::Wallet;
use chrono::Utc;
use gtk::glib::Sender;
use std::collections::{hash_map::Entry::Occupied, hash_map::Entry::Vacant, HashMap};
use std::io;
use std::net::SocketAddr;
use std::sync::{
    mpsc::{self, Receiver},
    Arc, RwLock,
};
use std::thread::{self, JoinHandle};

use crate::interface::components::table::{
    table_data_from_blocks, table_data_from_headers, table_data_from_tx, GtkTable, GtkTableData,
};
use crate::interface::{
    components::{overview_panel::TransactionDisplayInfo, send_panel::TransactionInfo},
    update_ui_progress_bar,
};
pub type BlockSet = HashMap<HashId, Block>;

/// Structs of the network controller (main controller of the program)
pub struct NetworkController {
    headers: HashMap<HashId, BlockHeader>,
    tallest_header: BlockHeader,
    valid_blocks: BlockSet,   // valid blocks downloaded so far
    blocks_on_hold: BlockSet, // downloaded blocks for which we don't have the previous block
    pending_blocks: HashMap<HashId, Vec<HashId>>, // blocks which haven't arrived, and the blocks which come immediately after them
    utxo_set: UtxoSet,
    nodes: NodeController,
    ui_sender: Sender<GtkMessage>,
    wallet: Wallet,
    tx_read: HashMap<HashId, ()>,
}

impl NetworkController {
    /// Creates a new network controller from the given sender and writer
    pub fn new(
        ui_sender: Sender<GtkMessage>,
        writer_end: mpsc::Sender<(SocketAddr, Message)>,
        config: Config,
    ) -> Result<Self, io::Error> {
        let genesis_header = BlockHeader::genesis(config.get_genesis());
        let wallet = match config.get_wallet() {
            Some(w) => w,
            None => {
                let new_wallet = Wallet::new();
                //utils::create_notification_window(gtk::MessageType::Info, title, message);
                eprintln!("Since a secret key was not provided through the configuration file, a new wallet has been created. {{Secret key: {}, Address: {}}}", new_wallet.secret_key.display_secret(), new_wallet.address);
                new_wallet
            }
        };
        Wallet::display_in_ui(&wallet, Some(&ui_sender));
        Ok(Self {
            headers: HashMap::from([(genesis_header.hash(), genesis_header)]),
            tallest_header: genesis_header,
            valid_blocks: BlockSet::new(),
            blocks_on_hold: BlockSet::new(),
            pending_blocks: HashMap::new(),
            utxo_set: UtxoSet::new(),
            nodes: NodeController::connect_to_peers(writer_end, ui_sender.clone(), config)?,
            wallet,
            ui_sender,
            tx_read: HashMap::new(),
        })
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
        let (balance, pending) = self.read_wallet_balance()?;
        self.ui_sender
            .send(GtkMessage::UpdateBalance((balance, pending)))
            .map_err(to_io_err)
    }

    fn update_ui_overview(&mut self, transaction: &RawTransaction) -> io::Result<()> {
        let transaction_info: TransactionDisplayInfo = transaction.transaction_info_for(
            &self.wallet.address,
            Utc::now().timestamp() as u32,
            &mut self.utxo_set,
        );
        self.ui_sender
            .send(GtkMessage::UpdateOverviewTransactions((
                transaction_info,
                TransactionOrigin::Pending,
            )))
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

    fn read_wallet_balance(&self) -> io::Result<(u64, u64)> {
        let balance = self.utxo_set.get_wallet_balance(&self.wallet.address);
        let pending_balance = self
            .utxo_set
            .get_pending_wallet_balance(&self.wallet.address);

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

    fn get_best_blocks(&self, headers: Vec<&BlockHeader>) -> Vec<&Block> {
        let mut best_blocks = vec![];
        for header in headers {
            if let Some(block) = self.valid_blocks.get(&header.hash()) {
                best_blocks.push(block);
            }
        }
        best_blocks
    }

    fn read_backup_block(&mut self, block: Block) {
        if self
            .valid_blocks
            .contains_key(&block.header.prev_block_hash)
        {
            let hash = block.hash();
            self.blocks_on_hold.insert(hash, block);
            self.add_to_valid_blocks(hash);
        } else {
            self.put_block_on_hold(block);
        }
    }

    fn read_incoming_block(&mut self, mut block: Block, config: &Config) -> io::Result<()> {
        if self.validate_block(&block).is_err() {
            return Ok(()); // ignore invalid or duplicate blocks
        }

        block.save_to_file(config.get_blocks_file())?;
        if let Some(previous_block) = self.valid_blocks.get(&block.header.prev_block_hash) {
            block.header.height = previous_block.header.height + 1;
            if let Vacant(entry) = self.headers.entry(block.hash()) {
                entry.insert(block.header);
            }
            let hash = block.hash();
            self.blocks_on_hold.insert(hash, block);
            self.add_to_valid_blocks(hash);
        } else {
            self.put_block_on_hold(block);
        }
        Ok(())
    }

    fn validate_block(&mut self, block: &Block) -> io::Result<()> {
        if self.valid_blocks.contains_key(&block.hash())
            || self.blocks_on_hold.contains_key(&block.hash())
        {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "This block has already been received before",
            ));
        }
        block.validate()?;
        Ok(())
    }

    fn _add_to_valid_blocks(&mut self, mut block: Block) {
        let _ = block.expand_utxo(
            &mut self.utxo_set,
            Some(&self.ui_sender),
            Some(&self.wallet.address),
        );

        let _ = self.update_ui_balance();

        // get real height of the block
        block.header.height = match self.headers.get(&block.hash()) {
            Some(header) => header.height,
            None => 0,
        };

        // update progress bar
        let mut progress =
            ((block.header.height as f64 / self.tallest_header.height as f64) * 100.0) % 1.0;
        if progress == 0.0 {
            progress = 1.0;
        }
        _ = update_ui_progress_bar(&self.ui_sender, None, progress);

        self.valid_blocks.insert(block.hash(), block);
    }

    fn add_to_valid_blocks(&mut self, block_id: HashId) {
        // if there where blocks on hold waiting for this one, validate them
        let mut blocks_not_on_hold: Vec<HashId> = vec![block_id];
        while let Some(block_id) = blocks_not_on_hold.pop() {
            if let Some(block) = self.blocks_on_hold.remove(&block_id) {
                self._add_to_valid_blocks(block);
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
            if let Vacant(entry) = self.headers.entry(header.hash()) {
                new_headers.push(header);
                entry.insert(header);
                if header.height > self.tallest_header.height {
                    self.tallest_header = header
                }
            }
        }

        config.log(
            &format!(
                "Read backup headers. New header count: {:?}",
                self.headers.len()
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

        // since every block needs to come after a valid block, create a "pseudo genesis" validated block
        if headers.block_headers.is_empty() {
            return Ok(());
        }

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

    fn read_headers(&mut self, headers: Headers, config: &Config) -> io::Result<()> {
        let prev_header_count = self.headers.len();
        // save new headers to hashmap and backup file
        let mut new_headers: Vec<BlockHeader> = vec![];
        for mut header in headers.block_headers {
            match self.headers.get(&header.prev_block_hash) {
                Some(parent_header) => {
                    header.height = parent_header.height + 1;
                }
                None => continue, // ignore header if prev_header is unknown
            }
            let hash = header.hash();
            if let Vacant(entry) = self.headers.entry(hash) {
                header.save_to_file(config.get_headers_file())?;
                new_headers.push(header);
                entry.insert(header);
                if header.height > self.tallest_header.height {
                    self.tallest_header = header
                }
            }
        }
        if prev_header_count == self.headers.len() {
            return Ok(());
        }
        config.log(
            &format!("Read headers. New header count: {:?}", self.headers.len()),
            VERBOSE,
        );
        let msg = format!("Read headers. New header count: {:?}", self.headers.len());
        self.update_ui_progress(Some(&msg), 0.0);
        // request blocks mined after given date
        let headers = Headers::new(new_headers.len(), new_headers);
        self.try_request_trimmed_blocks(headers, config)
    }

    fn request_headers(&mut self, header_hash: HashId, config: &Config) -> io::Result<()> {
        let getheader_message = GetHeader::from_last_header(header_hash);
        self.nodes
            .send_to_all(&getheader_message.serialize()?, config)?;
        Ok(())
    }

    /// read inv message from peer, if it contains tx invs, request txs to same peer
    fn read_inventories(
        &mut self,
        peer: SocketAddr,
        inventories: Vec<Inventory>,
        config: &Config,
    ) -> io::Result<()> {
        let mut filtered_inv: Vec<Inventory> = Vec::new();
        for inventory in inventories {
            if inventory.inv_type == InvType::MSGTx || inventory.inv_type == InvType::MSGBlock {
                filtered_inv.push(inventory);
            }
        }

        if filtered_inv.is_empty() {
            return Ok(());
        }

        let getdata_message = GetData::new(filtered_inv.len(), filtered_inv);
        // ignore inv and error if target node is not reachable
        let _ = self
            .nodes
            .send_to_specific(&peer, &getdata_message.serialize()?, config);
        Ok(())
    }

    fn read_pending_tx(&mut self, transaction: RawTransaction) -> io::Result<()> {
        // get data from tx and update ui
        let tx_hash: HashId = transaction.get_hash();
        if self.tx_read.contains_key(&tx_hash) {
            return Ok(());
        }

        if transaction.address_is_involved(&self.wallet.address) {
            transaction.generate_utxo(
                &mut self.utxo_set,
                TransactionOrigin::Pending,
                Some(&self.ui_sender),
                Some(&self.wallet.address),
            )?;

            // get wallet balance and update UI
            self.update_ui_balance()?;

            // add transaction to overview
            self.update_ui_overview(&transaction)?;
        }

        // if self.tx_read
        let data = table_data_from_tx(&transaction);
        self.update_ui_table(GtkTable::Transactions, data)?;

        self.tx_read.insert(tx_hash, ());
        Ok(())
    }

    /// Generates a transaction and broadcasts it to all peers given the transaction details
    pub fn generate_transaction(
        &mut self,
        details: TransactionInfo,
        config: &Config,
    ) -> io::Result<()> {
        let tx = self
            .wallet
            .generate_transaction(&mut self.utxo_set, details);

        match tx {
            Ok(tx) => {
                let tx_hash = double_hash(&tx.serialize());

                let payload = tx.serialize();
                let mut bytes = MessageHeader::new(
                    MAGIC,
                    TX.to_string(),
                    payload.len() as u32,
                    [tx_hash[0], tx_hash[1], tx_hash[2], tx_hash[3]],
                )
                .serialize()?;

                bytes.extend(payload);

                // send bytes to all
                self.nodes.send_to_all(&bytes, config)?;

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

    /// Starts the sync process by requesting headers from all peers from the last known header (or genesis block) to the current time
    /// If a backup file is found, it will read the blocks and headers from the backup file
    pub fn start_sync(&mut self, config: &Config) -> io::Result<()> {
        let mut downloadable_headers = Headers::default();
        // attempt to read headers from backup file
        if let Ok(headers) = Headers::from_file(config.get_headers_file()) {
            self.update_ui_progress(Some("Reading headers from backup file..."), 0.0);
            downloadable_headers = self.read_backup_headers(headers, config);
            update_ui_progress_bar(&self.ui_sender, Some("Read headers from backup file."), 1.0)?;
        }

        // attempt to read blocks from backup file
        if let Ok(blocks) = Block::all_from_file(config.get_blocks_file()) {
            self.update_ui_progress(Some("Found blocks backup file, reading blocks..."), 0.0);
            for (_, block) in blocks.into_iter() {
                self.read_backup_block(block);
            }
            update_ui_progress_bar(&self.ui_sender, Some("Read blocks from backup file."), 1.0)?;
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

/// NetworkController is a wrapper around the inner NetworkController in order to allow for safe multithreading
pub struct OuterNetworkController {
    inner: Arc<RwLock<NetworkController>>,
}

impl OuterNetworkController {
    /// Creates a new OuterNetworkController given a ui_sender and a writer
    pub fn new(
        ui_sender: Sender<GtkMessage>,
        writer_end: mpsc::Sender<(SocketAddr, Message)>,
        config: Config,
    ) -> Result<Self, io::Error> {
        let inner = Arc::new(RwLock::new(NetworkController::new(
            ui_sender, writer_end, config,
        )?));
        Ok(Self { inner })
    }

    fn handle_ui_generate_transaction(
        t_inner: Arc<RwLock<NetworkController>>,
        transaction_info: TransactionInfo,
        config: Config,
    ) -> io::Result<()> {
        let mut inner_lock = t_inner.write().map_err(to_io_err)?;
        inner_lock.generate_transaction(transaction_info, &config)
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
                }?;
            }
        });
        Ok(())
    }

    fn handle_node_block_message(
        t_inner: Arc<RwLock<NetworkController>>,
        block: Block,
        config: &Config,
    ) -> io::Result<()> {
        t_inner
            .write()
            .map_err(to_io_err)?
            .read_incoming_block(block, config)
    }

    fn handle_node_headers_message(
        t_inner: Arc<RwLock<NetworkController>>,
        headers: Headers,
        config: &Config,
    ) -> io::Result<()> {
        t_inner
            .write()
            .map_err(to_io_err)?
            .read_headers(headers, config)
    }

    fn handle_node_inv_message(
        t_inner: Arc<RwLock<NetworkController>>,
        peer_addr: SocketAddr,
        inventories: Vec<Inventory>,
        config: &Config,
    ) -> io::Result<()> {
        t_inner
            .write()
            .map_err(to_io_err)?
            .read_inventories(peer_addr, inventories, config)
    }

    fn handle_node_tx_message(
        t_inner: Arc<RwLock<NetworkController>>,
        tx: RawTransaction,
    ) -> io::Result<()> {
        t_inner.write().map_err(to_io_err)?.read_pending_tx(tx)
    }

    fn recv_node_messages(
        &self,
        node_receiver: mpsc::Receiver<(SocketAddr, Message)>,
        config: Config,
    ) -> io::Result<JoinHandle<io::Result<()>>> {
        let inner = self.inner.clone();
        let handle = thread::spawn(move || -> io::Result<()> {
            loop {
                let t_inner: Arc<RwLock<NetworkController>> = inner.clone();
                if let Err(result) = match node_receiver.recv().map_err(to_io_err)? {
                    (_, Message::Headers(headers)) => {
                        Self::handle_node_headers_message(t_inner, headers, &config)
                    }
                    (_, Message::Block(block)) => {
                        Self::handle_node_block_message(t_inner, block, &config)
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

    fn update_ui_data_periodically(&self) -> io::Result<()> {
        let inner = self.inner.clone();
        thread::spawn(move || -> io::Result<()> {
            let mut tallest_header_hash = HashId::default();
            loop {
                thread::sleep(std::time::Duration::from_secs(3));
                let controller = inner.read().map_err(to_io_err)?;
                if controller.tallest_header.hash() != tallest_header_hash {
                    tallest_header_hash = controller.tallest_header.hash();
                    // update ui with last 100 headers
                    let headers: Vec<&BlockHeader> = controller.get_best_headers(100);
                    let data = table_data_from_headers(headers.clone());
                    controller.update_ui_table(GtkTable::Headers, data)?;

                    // update ui with last 100 blocks
                    let blocks = controller.get_best_blocks(headers);
                    let data = table_data_from_blocks(blocks);
                    controller.update_ui_table(GtkTable::Blocks, data)?;
                }
            }
        });
        Ok(())
    }

    fn sync(&self, config: Config) -> io::Result<()> {
        let inner = self.inner.clone();
        thread::spawn(move || -> io::Result<()> {
            inner.write().map_err(to_io_err)?.start_sync(&config)
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
        self.sync(config)?;
        self.update_ui_data_periodically()
    }
}
