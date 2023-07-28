use crate::config::Config;
use crate::interface::{GtkMessage, ModelRequest};
use crate::messages::block_header::HeaderSet;
use crate::messages::constants::{
    commands::TX,
    config::{MAGIC, QUIET, VERBOSE},
};
use crate::messages::{
    Block, BlockHeader, GetData, GetHeader, HashId, Hashable, Headers, InvType, Inventory, Message,
    MessageHeader, Serialize,
};

use crate::messages::invblock_message::{InventoryBlock, InventoryVector};
use crate::node_controller::NodeController;
use crate::raw_transaction::{RawTransaction, TransactionOrigin};
use crate::utility::{double_hash, to_io_err};
use crate::utxo::UtxoSet;
use crate::wallet::Wallet;
use bitcoin_hashes::Hash;
use chrono::Utc;
use gtk::glib::SyncSender;
use std::collections::{hash_map::Entry::Occupied, hash_map::Entry::Vacant, HashMap};
use std::io;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::{
    mpsc::{self, Receiver},
    Arc, RwLock, RwLockReadGuard,
};
use std::thread::{self, JoinHandle};

use crate::interface::components::table::{
    table_data_from_blocks, table_data_from_headers, table_data_from_tx, GtkTable, GtkTableData,
};
use crate::interface::{
    components::{overview_panel::TransactionDisplayInfo, send_panel::TransactionInfo},
    update_ui_progress_bar,
};
use crate::messages::constants::config::{LOCALHOST, LOCALSERVER, PORT};
use crate::messages::InvType::MSGBlock;
use crate::node::Node;

pub type BlockSet = HashMap<HashId, Block>;

/// Structs of the network controller (main controller of the program)
pub struct NetworkController {
    headers: HeaderSet,
    tallest_header: BlockHeader,
    valid_blocks: BlockSet,   // valid blocks downloaded so far
    blocks_on_hold: BlockSet, // downloaded blocks for which we don't have the previous block
    pending_blocks: HashMap<HashId, Vec<HashId>>, // blocks which haven't arrived, and the blocks which come immediately after them
    utxo_set: UtxoSet,
    nodes: NodeController,
    ui_sender: SyncSender<GtkMessage>,
    wallet: Wallet,
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
            headers: HeaderSet::with(genesis_header.hash(), genesis_header),
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

    fn handle_getheaders_message(&self, getheaders_message: GetHeader) -> Option<Headers> {
        let mut headers: Vec<BlockHeader> = Vec::new();
        let last_known_hash = match getheaders_message.block_header_hashes.first() {
            Some(hash) => hash.clone(),
            None => return None, // No hay hashes disponibles, devolver None
        };
        // if next block header is None, return empty Headers message
        if self.headers.get_next_header(&last_known_hash).is_none() {
            println!("[handle_getheaders_message] next header is none");
            return Some(Headers {
                count: 0,
                block_headers: headers,
            });
        }

        let stop_hash = HashId { hash: [0; 32] };
        let mut count = 0;

        // Si el stop_hash es igual a ceros, limitamos a 2000 bloques, de lo contrario, agregamos solo el siguiente bloque
        let max_blocks = if getheaders_message.stop_hash == stop_hash {
            2000
        } else {
            let mut next_block_header = self.headers.get_next_header(&last_known_hash)?;
            while next_block_header.hash != getheaders_message.stop_hash {
                headers.push(next_block_header.clone());
                next_block_header = self.headers.get_next_header(&next_block_header.hash())?;
                count += 1;
                if count >= 2000 {
                    break;
                }
            }
            return Some(Headers {
                count: headers.len(),
                block_headers: headers,
            });
        };

        let mut next_block_header = self.headers.get_next_header(&last_known_hash)?;

        while count < max_blocks {
            headers.push(next_block_header.clone());
            next_block_header = match self.headers.get_next_header(&next_block_header.hash()) {
                Some(header) => header,
                None => break, // Manejar el caso en el que no se encuentre el siguiente encabezado
            };
            count += 1;
        }

        Some(Headers {
            count: headers.len(),
            block_headers: headers,
        })
    }

    fn handle_getdata_message(&self, getdata_message: GetData) -> Option<InventoryVector> {
        let mut blocks: Vec<Block> = Vec::new();
        for inventory in getdata_message.get_inventory() {
            match inventory.inv_type {
                InvType::_MSGCompactBlock => {
                    let block = match self.valid_blocks.get(&inventory.hash) {
                        Some(block) => block.clone(),
                        None => continue,
                    };
                    blocks.push(block);
                }
                _ => continue,
            }
        }

        Some(InventoryVector::from_inv(blocks.len(), blocks))
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

    fn _add_to_valid_blocks(&mut self, mut block: Block) {
        let _ = block.expand_utxo(
            &mut self.utxo_set,
            Some(&self.ui_sender),
            Some(&self.wallet.address),
        );

        let _ = self.update_ui_balance();

        // get real height of the block
        block.header.height = match self.valid_blocks.get(&block.header.prev_block_hash) {
            Some(prev_block) => prev_block.header.height + 1,
            _ => self.tallest_header.height + 1,
        };

        // update progress bar
        let mut progress =
            ((block.header.height as f64 / self.tallest_header.height as f64) * 100.0) % 1.0;
        if progress == 0.0 {
            progress = 1.0;
        }
        let msg = format!("Reading block {}", block.header.height);
        _ = update_ui_progress_bar(&self.ui_sender, Some(&msg), progress);

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
            self.headers.insert(header.hash(), header);
            new_headers.push(header);
            if header.height > self.tallest_header.height {
                self.tallest_header = header
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

    fn request_headers(&mut self, header_hash: HashId, config: &Config) -> io::Result<()> {
        let getheader_message = GetHeader::from_last_header(header_hash);
        self.nodes
            .send_to_all(&getheader_message.serialize()?, config)?;
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

    // DELETE ME
    // pub fn listen_for_nodes(
    //     &self,
    //     writer_channel: std::sync::mpsc::SyncSender<(SocketAddr, Message)>,
    //     config: Config,
    // ) -> io::Result<()> {
    //     let listener = TcpListener::bind(LOCALSERVER)?;
    //     let ui_sender = self.ui_sender.clone();
    //     thread::spawn(move || -> io::Result<()> {
    //         println!("Listening on port {}", PORT);
    //         println!("Listener: {:?}", listener);
    //         for stream in listener.incoming() {
    //             println!("Incoming connection");
    //             println!("Stream: {:?}", stream);
    //             match stream {
    //                 Ok(mut stream) => {
    //                     let ui_sender = ui_sender.clone();
    //                     println!("New connectionn: {}", stream.peer_addr()?);
    //                     Node::inverse_handshake(&mut stream).unwrap();

    //                     let node =
    //                         Node::spawn(stream, writer_channel.clone(), ui_sender, config.clone())?;
    //                     // Network Controller should add the new node
    //                 }
    //                 Err(e) => {
    //                     println!("Error: {}", e);
    //                 }
    //             }
    //         }

    //         Ok(())
    //     });
    //     Ok(())
    // }
}

/// NetworkController is a wrapper around the inner NetworkController in order to allow for safe multithreading
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

    fn update_ui_data_periodically(&self) -> io::Result<()> {
        let inner = self.inner.clone();
        let ui_sender = self.ui_sender.clone();
        thread::spawn(move || -> io::Result<()> {
            let mut tallest_header_hash = HashId::default();
            let mut block_count = 0;
            loop {
                thread::sleep(std::time::Duration::from_secs(10));
                let controller = inner.read().map_err(to_io_err)?;
                let headers: Vec<&BlockHeader> = controller.get_best_headers(100);

                if controller.tallest_header.hash() != tallest_header_hash {
                    tallest_header_hash = controller.tallest_header.hash();
                    // update ui with last 100 headers
                    let data = table_data_from_headers(headers.clone());
                    ui_sender
                        .send(GtkMessage::UpdateTable((GtkTable::Headers, data)))
                        .map_err(to_io_err)?;
                }

                let curr_block_count = controller.valid_blocks.len();
                if curr_block_count > block_count {
                    // update ui with last best blocks
                    let blocks = controller.get_best_blocks(headers);
                    let data = table_data_from_blocks(blocks);
                    ui_sender
                        .send(GtkMessage::UpdateTable((GtkTable::Blocks, data)))
                        .map_err(to_io_err)?;

                    block_count = curr_block_count;
                }
            }
        });
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
        mut block: Block,
        config: &Config,
    ) -> io::Result<()> {
        let inner_read = t_inner.read().map_err(to_io_err)?;
        if inner_read.valid_blocks.contains_key(&block.hash())
            || inner_read.blocks_on_hold.contains_key(&block.hash())
        {
            // println!("This block has already been received before");
            return Ok(());
        }
        if block.validate().is_err() {
            // println!("invalid block, discarting");
            return Ok(());
        }
        block.save_to_file(config.get_blocks_file())?;

        drop(inner_read);
        let mut inner_write = t_inner.write().map_err(to_io_err)?;
        if let Some(previous_block) = inner_write.valid_blocks.get(&block.header.prev_block_hash) {
            block.header.height = previous_block.header.height + 1;
            if let Vacant(entry) = inner_write.headers.entry(block.hash()) {
                entry.insert(block.header);
            }
            let hash = block.hash();
            let block_header = block.header;
            inner_write.blocks_on_hold.insert(hash, block);

            if block_header.prev_block_hash == inner_write.tallest_header.hash() {
                inner_write.headers.insert(hash, block_header);
                inner_write.tallest_header = block_header;
            }

            inner_write.add_to_valid_blocks(hash);
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
        let most_recent_timestamp = inner_read.tallest_header.timestamp;

        // the closer the timestamp is to the current time, the more progress we have made
        let diff = (Utc::now().timestamp() - most_recent_timestamp as i64) as f64 / 1000000000.0;
        let progress = 1.0 - diff;

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

        Self::handle_headers_message_info(config, inner_read, ui_sender)?;

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

        for block in blocks{
            println!("Sending block to peer {:?}, block hash: {:?}", peer_addr, block.hash());
            let serialized_block = block.serialize_message()?;
            inner_write.nodes.send_to_specific(&peer_addr, &serialized_block, config)?;
        }
        Ok(())
    }

    fn handle_node_tx_message(
        t_inner: Arc<RwLock<NetworkController>>,
        tx: RawTransaction,
    ) -> io::Result<()> {
        t_inner.write().map_err(to_io_err)?.read_pending_tx(tx)
    }

    pub fn handle_get_headers_message(
        t_inner: Arc<RwLock<NetworkController>>,
        getheaders: GetHeader,
        peer_addr: SocketAddr,
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

    fn handle_get_data_message(
        t_inner: Arc<RwLock<NetworkController>>,
        getdata: GetData,
        peer_addr: SocketAddr,
        config: &Config,
    ) -> io::Result<()> {
        let mut inner_write = t_inner.write().map_err(to_io_err)?;
        if let Some(inventory_message) = inner_write.handle_getdata_message(getdata) {
            let _ = inner_write.nodes.send_to_specific(
                &peer_addr,
                &inventory_message.serialize()?,
                config,
            );
        }
        Ok(())
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
                    (peer_addr, Message::_GetHeader(get_headers)) => {
                        //println!("Received getheaders message: {:?}", get_headers);
                        Self::handle_get_headers_message(t_inner, get_headers, peer_addr, &config)
                    }
                    (_, Message::Block(block)) => {
                        Self::handle_node_block_message(t_inner, block, &config)
                    }
                    (peer_addr, Message::Inv(inventories)) => {
                        Self::handle_node_inv_message(t_inner, peer_addr, inventories, &config)
                    }
                    (_, Message::Transaction(tx)) => Self::handle_node_tx_message(t_inner, tx),
                    (peer_address, Message::_GetData(GetData)) => {
                        Self::handle_get_data_message(t_inner, GetData, peer_address, &config)
                    }
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
        let listener = TcpListener::bind(LOCALSERVER)?;
        let ui_sender = self.ui_sender.clone();
        let writer_channel = self.writer_chanel.clone();

        thread::spawn(move || -> io::Result<()> {
            println!("Listening on port {}", PORT);
            println!("Listener: {:?}", listener);
            for stream in listener.incoming() {
                println!("Incoming connection");
                println!("Stream: {:?}", stream);
                match stream {
                    Ok(mut stream) => {
                        let ui_sender = ui_sender.clone();
                        println!("New connectionn: {}", stream.peer_addr()?);
                        Node::inverse_handshake(&mut stream).unwrap();

                        let node =
                            Node::spawn(stream, writer_channel.clone(), ui_sender, config.clone())?;
                        // Network Controller should add the new node
                        println!("adding node: {:?}", node);
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
            println!("Starting sync");
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
        self.sync(config)?;
        self.update_ui_data_periodically()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::constants::config::LOCALSERVER;
    use crate::messages::version_message::Version;
    use crate::messages::{block_header, VerAck};
    use gtk::glib;
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::path::PathBuf;
    use std::sync::mpsc::SyncSender;
    use std::time::Duration;

    // La función que vamos a probar, que envía un getheaders al servidor
    fn run_client_getheaders(config: &Config) -> io::Result<Message> {
        // Realizar el handshake
        let mut socket = TcpStream::connect(LOCALSERVER).unwrap();

        //Envio un version
        let msg_version = Version::default_for_trans_addr(socket.peer_addr().unwrap());
        let payload = msg_version.serialize().unwrap();
        socket.write_all(&payload).unwrap();
        socket.flush().unwrap();

        //Leo el header del version message
        let version_header = MessageHeader::from_stream(&mut socket).unwrap();
        //Leo el payload del version message
        let payload_data = version_header.read_payload(&mut socket).unwrap();

        //Recibo un verack message
        let verack = VerAck::from_stream(&mut socket).unwrap();
        //Envio un verack message
        let payload = VerAck::new().serialize().unwrap();
        socket.write_all(&payload).unwrap();
        socket.flush().unwrap();

        // Enviar el getheaders message
        let block_header_hashes = vec![config.get_genesis()];
        let getheaders_message = GetHeader {
            version: 70015,
            hash_count: 1,
            block_header_hashes,
            stop_hash: HashId { hash: [0u8; 32] },
        };
        let payload = getheaders_message.serialize()?;
        socket.write_all(&payload)?;
        socket.flush()?;

        let headers_message = MessageHeader::from_stream(&mut socket)?;
        let payload = headers_message.read_payload(&mut socket)?;
        let headers_message: Message = Headers::deserialize(&payload)?;
        Ok(headers_message)
    }

    #[test]
    fn test_handle_getheaders_message_client_server_communication() {
        let (ui_sender, _) = glib::MainContext::sync_channel(glib::PRIORITY_HIGH, 100);
        let (writer_end, node_receiver) = mpsc::sync_channel(100);

        let config_file = "node.conf";
        let config_path: PathBuf = config_file.into(); // Convert &str to PathBuf

        let config = Config::from_file(config_path).unwrap();
        let config_clone = config.clone();
        // let outer_controller =
        //     OuterNetworkController::new(ui_sender.clone(), writer_end.clone(), config.clone())
        //         .unwrap();

        // outer_controller.sync(config.clone()).unwrap();
        let mut network_controller =
            NetworkController::new(ui_sender, writer_end, config.clone()).unwrap();
        network_controller.start_sync(&config);

        // Iniciar el servidor en un hilo separado
        let server_handle = thread::spawn(move || -> io::Result<()> {
            let listener = TcpListener::bind(LOCALSERVER)?;

            let connection = listener.accept()?;
            let mut socket: TcpStream = connection.0;

            Node::inverse_handshake(&mut socket).unwrap();

            let message_header_fromstream = MessageHeader::from_stream(&mut socket).unwrap();
            let payload_fromstream = message_header_fromstream.read_payload(&mut socket).unwrap();

            //construct getheaders message received from stream
            let getheaders_message = match GetHeader::deserialize(&payload_fromstream)? {
                Message::_GetHeader(get_header_message) => get_header_message,
                _ => panic!("Error"),
            };

            let getheaders_response = network_controller
                .handle_getheaders_message(getheaders_message)
                .unwrap();
            let payload = getheaders_response.serialize().unwrap();
            socket.write_all(&payload).unwrap();
            socket.flush().unwrap();
            Ok(())
        });

        // Ejecutar el cliente y enviar el getheaders al servidor (utiliza la función run_client_getheaders que definiste antes)
        let response = run_client_getheaders(&config_clone).unwrap();

        // Esperar a que termine la prueba
        server_handle.join().unwrap();

        //read the content of response enum
        let headers = match response {
            Message::Headers(headers) => {
                assert_eq!(headers.count,2000);
            }
            _ => panic!("Error"),
        };
    }

    #[test]
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

        let mut socket = TcpStream::connect(LOCALSERVER).unwrap();

        //Envio un version
        let msg_version = Version::default_for_trans_addr(socket.peer_addr().unwrap());
        let payload = msg_version.serialize().unwrap();
        socket.write_all(&payload).unwrap();
        socket.flush().unwrap();

        //Leo el header del version message
        let version_header = MessageHeader::from_stream(&mut socket).unwrap();
        //Leo el payload del version message
        let payload_data = version_header.read_payload(&mut socket).unwrap();

        //Recibo un verack message
        let verack = VerAck::from_stream(&mut socket).unwrap();
        println!("Verack message: {:?}", verack);
        //Envio un verack message
        let payload = VerAck::new().serialize().unwrap();
        socket.write_all(&payload).unwrap();
        socket.flush().unwrap();
    }

    #[test]
    fn test_handle_getheaders_message_genesis() {
        // Create a test NetworkController instance
        let (ui_sender, _) = glib::MainContext::sync_channel(glib::PRIORITY_HIGH, 100);
        let (writer_end, _) = std::sync::mpsc::sync_channel::<(SocketAddr, Message)>(100);
        let writer_end: SyncSender<(SocketAddr, Message)> = writer_end;

        let config_file = "node.conf";
        let config_path: PathBuf = config_file.into(); // Convert &str to PathBuf

        let config = Config::from_file(config_path).unwrap();
        let mut network_controller =
            NetworkController::new(ui_sender, writer_end, config.clone()).unwrap();
        network_controller.start_sync(&config);

        let block_header_hashes = vec![config.get_genesis()];
        let getheaders_message = GetHeader {
            version: 70015,
            hash_count: 1,
            block_header_hashes,
            stop_hash: HashId { hash: [0u8; 32] },
        };

        let headers = network_controller
            .handle_getheaders_message(getheaders_message)
            .unwrap();

        println!("headers: {:?}", headers);

        assert_eq!(headers.count, 2000);
    }

    #[test]
    fn test_gethandle_message_until_stophash_from_hash() {
        // Create a test NetworkController instance
        let (ui_sender, _) = glib::MainContext::sync_channel(glib::PRIORITY_HIGH, 100);
        let (writer_end, _) = std::sync::mpsc::sync_channel::<(SocketAddr, Message)>(100);
        let writer_end: SyncSender<(SocketAddr, Message)> = writer_end;

        let config_file = "node.conf";
        let config_path: PathBuf = config_file.into(); // Convert &str to PathBuf

        let config = Config::from_file(config_path).unwrap();
        let mut network_controller =
            NetworkController::new(ui_sender, writer_end, config.clone()).unwrap();
        network_controller.start_sync(&config);

        let start_hash_id = HashId {
            hash: [
                69, 36, 173, 236, 194, 35, 82, 55, 18, 238, 17, 97, 150, 136, 232, 247, 203, 192,
                154, 69, 33, 156, 91, 217, 99, 186, 219, 190, 0, 0, 0, 0,
            ],
        };

        let block_header_hashes = vec![start_hash_id];
        let getheaders_message = GetHeader {
            version: 70015,
            hash_count: 1,
            block_header_hashes,
            stop_hash: HashId {
                hash: [
                    21, 46, 64, 71, 214, 53, 190, 154, 214, 175, 106, 193, 63, 2, 161, 224, 9, 192,
                    11, 245, 202, 187, 120, 34, 61, 30, 86, 44, 0, 0, 0, 0,
                ],
            },
        };

        let headers = network_controller
            .handle_getheaders_message(getheaders_message)
            .unwrap();

        //println!("headers: {:?}", headers);
        assert_eq!(headers.count, 2);
    }

    #[test]
    fn test_handle_getheaders_message_unknown_hash() {
        // Create a test NetworkController instance
        let (ui_sender, _) = glib::MainContext::sync_channel(glib::PRIORITY_HIGH, 100);
        let (writer_end, _) = std::sync::mpsc::sync_channel::<(SocketAddr, Message)>(100);
        let writer_end: SyncSender<(SocketAddr, Message)> = writer_end;

        let config_file = "node.conf";
        let config_path: PathBuf = config_file.into(); // Convert &str to PathBuf

        let config = Config::from_file(config_path).unwrap();
        let mut network_controller =
            NetworkController::new(ui_sender, writer_end, config.clone()).unwrap();
        network_controller.start_sync(&config);

        let unknown_hash = HashId { hash: [1; 32] };

        let block_header_hashes = vec![unknown_hash];
        let getheaders_message = GetHeader {
            version: 70015,
            hash_count: 1,
            block_header_hashes,
            stop_hash: HashId { hash: [0u8; 32] },
        };

        let headers = network_controller
            .handle_getheaders_message(getheaders_message)
            .unwrap();

        assert_eq!(headers.count, 0);
        assert_eq!(headers.block_headers.len(), 0);
    }

    #[test]
    fn test_handle_getheaders_message_tallesthash() {
        let (ui_sender, _) = glib::MainContext::sync_channel(glib::PRIORITY_HIGH, 100);
        let (writer_end, _) = std::sync::mpsc::sync_channel::<(SocketAddr, Message)>(100);
        let writer_end: SyncSender<(SocketAddr, Message)> = writer_end;

        let config_file = "node.conf";
        let config_path: PathBuf = config_file.into();

        let config = Config::from_file(config_path).unwrap();
        let mut network_controller =
            NetworkController::new(ui_sender, writer_end, config.clone()).unwrap();
        network_controller.start_sync(&config);

        let block_header_hashes = vec![config.get_genesis()];
        let mut getheaders_message = GetHeader {
            version: 70015,
            hash_count: 1,
            block_header_hashes,
            stop_hash: HashId { hash: [0u8; 32] },
        };

        let mut headers = network_controller
            .handle_getheaders_message(getheaders_message.clone())
            .unwrap();

        while headers.count == 2000 {
            let last_header = headers.block_headers.last().unwrap();
            //let next_block_hash = last_header.next_block_hash.unwrap();
            getheaders_message.block_header_hashes = vec![last_header.hash];
            // println!("last_header: {:?}", last_header);
            // println!(
            //     "last header of nc: {:?}",
            //     network_controller.tallest_header.hash
            // );
            headers = network_controller
                .handle_getheaders_message(getheaders_message.clone())
                .unwrap();
        }

        let nc_tallest_header = &network_controller.tallest_header;
        let tallest_header = headers.block_headers.last().unwrap();

        //println!("tallest_header: {:?}", tallest_header.hash);

        assert_eq!(nc_tallest_header.hash, tallest_header.hash);
    }
}
