use std::collections::{hash_map::Entry::Vacant, hash_map::Entry::Occupied, HashMap};
use std::io;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex, mpsc::{self, Receiver}};
use std::thread::{self, JoinHandle};
use bitcoin_hashes::Hash;
use gtk::glib::Sender;
use crate::config::Config;
use crate::interface::{GtkMessage, ModelRequest, TransactionDetails};
use crate::messages::{
    Block, BlockHeader, ErrorType, GetData, GetHeader, HashId, Hashable, Headers,
    InvType, Inventory, Message, MessageHeader, Serialize,
};
use crate::messages::constants::{
    commands::{PONG, TX},
    config::{MAGIC, VERBOSE},
    messages::GENESIS_HASHID,
};
use crate::node_controller::NodeController;
use crate::raw_transaction::{RawTransaction, TransactionOrigin};
use crate::utility::{_encode_hex, double_hash, into_hashmap, to_io_err};
use crate::utxo::UtxoSet;
use crate::wallet::Wallet;

pub type BlockSet = HashMap<HashId, Block>;

pub struct NetworkController {
    headers: HashMap<HashId, BlockHeader>,
    tallest_header: HashId,
    valid_blocks: BlockSet, // valid blocks downloaded so far
    blocks_on_hold: BlockSet, // downloaded blocks for which we don't have the previous block, and can't be added to valid_blocks just yet
    pending_blocks: HashMap<HashId, Vec<HashId>>, // Vec of downloaded blocks that have a common prev_block which hasn't been downloaded, for each prev_block_hash
    utxo_set: UtxoSet,
    nodes: NodeController,
    ui_sender: Sender<GtkMessage>,
    wallet: Wallet,
}

impl NetworkController {
    pub fn new(
        ui_sender: Sender<GtkMessage>,
        writer_end: mpsc::Sender<(SocketAddr, Message)>,
        config: Config,
    ) -> Result<Self, io::Error> {
        Ok(Self {
            headers: HashMap::new(),
            tallest_header: GENESIS_HASHID,
            valid_blocks: BlockSet::new(),
            blocks_on_hold: BlockSet::new(),
            pending_blocks: HashMap::new(),
            utxo_set: UtxoSet::new(),
            nodes: NodeController::connect_to_peers(writer_end, ui_sender.clone(), config)?,
            ui_sender,
            wallet: Wallet::login(),
        })
    }

    pub fn update_status_bar(&self, msg: String) -> io::Result<()> {
        self.ui_sender
            .send(GtkMessage::UpdateLabel(("status_bar".to_string(), msg)))
            .map_err(to_io_err)
    }

    fn read_wallet_balance(&self) -> io::Result<u64> {
        let balance = self.utxo_set.get_wallet_balance(&self.wallet.address);
        let pending_balance = self
            .utxo_set
            .get_pending_wallet_balance(&self.wallet.address);
        println!(
            "Wallet balance: {:?}\n       pending: {:?}",
            balance, pending_balance
        );
        self.ui_sender
            .send(GtkMessage::UpdateLabel((
                "balance_available_val".to_string(),
                format!("{:?}", balance),
            )))
            .map_err(to_io_err)?;

        Ok(balance)
    }

    fn read_backup_block(&mut self, block: Block) -> io::Result<()> {
        // if we have a clean backup, validating isn't necessary
        if self.validate_block(&block).is_err() {
            return Ok(()); // ignore invalid or duplicate blocks
        }
        if self.valid_blocks.contains_key(&block.header.prev_block_hash) {
            self.add_to_valid_blocks(block);
        } else {
            self.put_block_on_hold(block);
        }
        Ok(())
    }

    fn read_incoming_block(&mut self, block: Block, config: &Config) -> io::Result<()> {
        if self.validate_block(&block).is_err() {
            return Ok(()); // ignore invalid or duplicate blocks
        }
        block.save_to_file(config.get_blocks_file())?;
        if self.valid_blocks.contains_key(&block.header.prev_block_hash) {
            self.add_to_valid_blocks(block);
        } else {
            self.put_block_on_hold(block);
        }
        Ok(())
    }

    fn validate_block(&mut self, block: &Block) -> io::Result<()> {
        if self.valid_blocks.contains_key(&block.hash()) || self.blocks_on_hold.contains_key(&block.hash()) {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "This block has already been received before",
            ));
        }
        // validation only checks proof of work
        // Also saves and consumes UTXOs for later spending 
        block.validate(&mut self.utxo_set)?;
        Ok(())
    }

    fn add_to_valid_blocks(&mut self, block: Block) {
        let days_old = block.get_days_old();
        if days_old > 0 {
            let _ = self.update_status_bar(format!(
                "Reading blocks, {:?} days behind",
                block.get_days_old()
            ));
        } else {
            let _ = self.update_status_bar("Up to date".to_string());
        }
        let block_hash = block.hash();
        self.valid_blocks.insert(block_hash, block);
        // if there where blocks on hold waiting for this one, validate them
        if let Some(blocked_blocks) = self.pending_blocks.remove(&block_hash) {
            for block_hash in blocked_blocks {
                if let Some(block) = self.blocks_on_hold.remove(&block_hash) {
                    self.add_to_valid_blocks(block)
                }
            }
        }
    }

    fn put_block_on_hold(&mut self, block: Block) {
        // add to pending blocks the previous block, mark this block as blocked by the previous one
        match self.pending_blocks.entry(block.header.prev_block_hash) {
            Vacant(entry) => {entry.insert(vec![block.hash()]);},
            Occupied(mut entry) => entry.get_mut().push(block.hash())
        }
        self.blocks_on_hold.insert(block.hash(), block);
    }

    fn request_blocks(&mut self, headers: &mut Headers, config: &Config) -> io::Result<()> {
        if headers.count == 0 {
            return Ok(());
        }

        let chunks = headers.block_headers.chunks(20); // request 20 blocks at a time
        for chunk in chunks {
            let get_data = GetData::from_inv(chunk.len(), chunk.to_vec());
            self.nodes.send_to_any(&get_data.serialize()?, config)?;
        }
        config.get_logger().log("Requesting blocks, sent GetData message.", VERBOSE);
        Ok(())
    }

    fn retain_missing_headers(&self, headers: &mut Headers) {
        headers
            .block_headers
            .retain(|header| !self.valid_blocks.contains_key(&header.hash()));
    }

    /// requests block for headers after given timestamp
    fn request_blocks_from(
        &mut self,
        mut headers: Headers,
        timestamp: u32,
        config: &Config,
    ) -> io::Result<()> {
        headers.trim_timestamp(timestamp);
        self.retain_missing_headers(&mut headers);
        self.request_blocks(&mut headers, config)?;
        Ok(())
    }

    fn read_headers(&mut self, headers: Headers, config: &Config) -> io::Result<()> {
        let previous_header_count = self.headers.len();
        let init_tp_timestamp: u32 = config.get_start_timestamp();
        self.request_blocks_from(headers.clone(), init_tp_timestamp, config)?;
        // save values to variables before consuming headers
        let last_header = headers.last_header_hash();
        let is_paginated = headers.is_paginated();

        // store headers in hashmap, consuming headers
        let headers_hashmap = into_hashmap(headers.block_headers);
        for (header_hash, header) in headers_hashmap {
            if let Vacant(entry) = self.headers.entry(header_hash) {
                header.save_to_file(config.get_headers_file())?;
                entry.insert(header);
            }
        }
        if self.headers.len() == previous_header_count {
            return Ok(());
        }
        config.get_logger().log(
            &format!(
                "Received header. New header count: {:?}",
                self.headers.len()
            ),
            VERBOSE,
        );
        // request next headers, and blocks for recieved headers
        self.tallest_header = last_header;
        if is_paginated {
            self.request_headers(self.tallest_header, config)?;
        }
        Ok(())
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
        let mut txinv: Vec<Inventory> = Vec::new();
        for inventory in inventories {
            if inventory.inv_type == InvType::MSGTx {
                txinv.push(inventory);
            }
        }

        if txinv.is_empty() {
            return Ok(());
        }

        let getdata_message = GetData::new(txinv.len(), txinv);
        self.nodes
            .send_to_specific(&peer, &getdata_message.serialize()?, config)?;
        Ok(())
    }

    fn read_pending_tx(&mut self, transaction: RawTransaction) -> io::Result<()> {
        if transaction.address_is_involved(&self.wallet.address) {
            println!("Read a pending transaction involving this wallet!");
            transaction.generate_utxo(&mut self.utxo_set, TransactionOrigin::Pending)?;
        }
        Ok(())
    }

    fn read_ping(&mut self, peer_addr: SocketAddr, nonce: u64, config: &Config) -> io::Result<()> {
        let payload = nonce.to_le_bytes();
        let hash = double_hash(&payload);
        let checksum: [u8; 4] = [hash[0], hash[1], hash[2], hash[3]];
        let message_header =
            MessageHeader::new(MAGIC, PONG.to_string(), payload.len() as u32, checksum);
        let mut to_send = Vec::new();
        to_send.extend_from_slice(&message_header.serialize()?);
        to_send.extend_from_slice(&payload);
        self.nodes.send_to_specific(&peer_addr, &to_send, config)?;
        Ok(())
    }

    pub fn generate_transaction(
        &mut self,
        details: TransactionDetails,
        config: &Config,
    ) -> io::Result<()> {
        let (recv_addr, _label, amount) = details;

        let tx: RawTransaction =
            self.wallet
                .generate_transaction(&mut self.utxo_set, recv_addr, amount)?;

        // broadcast tx
        let tx_hash = double_hash(&tx.serialize()).to_byte_array();
        let tx_hash_str = _encode_hex(&tx_hash);
        println!("Generated transaction: {}, pending broadcast", tx_hash_str);

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

        Ok(())
    }

    pub fn start_sync(&mut self, config: &Config) -> io::Result<()> {
        // attempt to read blocks from backup file
        if let Ok(blocks) = Block::all_from_file(config.get_blocks_file()) {
            self.update_status_bar("Found blocks backup file, reading blocks...".to_string())?;
            for (_, block) in blocks.into_iter() {
                self.read_backup_block(block)?;
            }
            self.update_status_bar("Read blocks from backup file.".to_string())?;
        }

        // attempt to read headers from backup file
        if let Ok(headers) = Headers::from_file(config.get_headers_file()) {
            self.update_status_bar("Reading headers from backup file...".to_string())?;
            self.tallest_header = headers.last_header_hash();
            self.headers = into_hashmap(headers.clone().block_headers);

            // find headers for which we don't have the blocks and then request them
            let init_tp_timestamp: u32 = config.get_start_timestamp();
            self.request_blocks_from(headers, init_tp_timestamp, config)?;

            self.update_status_bar("Read headers from backup file.".to_string())?;
        } // Finally, catch up to blockchain doing IBD
        self.request_headers(self.tallest_header, config)?;
        Ok(())
    }
}

pub struct OuterNetworkController {
    inner: Arc<Mutex<NetworkController>>,
}

impl OuterNetworkController {
    pub fn new(
        ui_sender: Sender<GtkMessage>,
        writer_end: mpsc::Sender<(SocketAddr, Message)>,
        config: Config,
    ) -> Result<Self, io::Error> {
        let inner = Arc::new(Mutex::new(NetworkController::new(
            ui_sender, writer_end, config,
        )?));
        Ok(Self { inner })
    }

    fn handle_ui_get_balance(t_inner: Arc<Mutex<NetworkController>>) -> io::Result<()> {
        let inner_lock = t_inner.lock().map_err(to_io_err)?;
        inner_lock.read_wallet_balance()?;
        Ok(())
    }

    fn handle_ui_generate_transaction(
        t_inner: Arc<Mutex<NetworkController>>,
        details: TransactionDetails,
        config: &Config,
    ) -> io::Result<()> {
        let mut inner_lock = t_inner.lock().map_err(to_io_err)?;
        inner_lock.generate_transaction(details, config)?;
        Ok(())
    }

    fn recv_ui_messages(
        &self,
        ui_receiver: Receiver<ModelRequest>,
        config: Config,
    ) -> io::Result<()> {
        let inner = self.inner.clone();
        thread::spawn(move || -> io::Result<()> {
            loop {
                let t_inner: Arc<Mutex<NetworkController>> = inner.clone();
                match ui_receiver.recv().map_err(to_io_err)? {
                    ModelRequest::GetWalletBalance => Self::handle_ui_get_balance(t_inner),
                    ModelRequest::GenerateTransaction(details) => {
                        Self::handle_ui_generate_transaction(t_inner, details, &config)
                    }
                }?;
            }
        });
        Ok(())
    }

    fn handle_node_block_message(
        t_inner: Arc<Mutex<NetworkController>>,
        block: Block,
        config: &Config,
    ) -> io::Result<()> {
        t_inner
            .lock()
            .map_err(to_io_err)?
            .read_incoming_block(block, config)
    }

    fn handle_node_headers_message(
        t_inner: Arc<Mutex<NetworkController>>,
        headers: Headers,
        config: &Config,
    ) -> io::Result<()> {
        t_inner
            .lock()
            .map_err(to_io_err)?
            .read_headers(headers, config)
    }

    fn handle_node_inv_message(
        t_inner: Arc<Mutex<NetworkController>>,
        peer_addr: SocketAddr,
        inventories: Vec<Inventory>,
        config: &Config,
    ) -> io::Result<()> {
        t_inner
            .lock()
            .map_err(to_io_err)?
            .read_inventories(peer_addr, inventories, config)
    }

    fn handle_node_tx_message(
        t_inner: Arc<Mutex<NetworkController>>,
        tx: RawTransaction,
    ) -> io::Result<()> {
        t_inner.lock().map_err(to_io_err)?.read_pending_tx(tx)
    }

    fn handle_node_failure_message(
        _t_inner: Arc<Mutex<NetworkController>>,
        peer_addr: SocketAddr,
        error: ErrorType,
    ) -> io::Result<()> {
        println!(
            "Node {:?} is notifying me of a failure, should resend last request, the error is {:?}",
            peer_addr, error
        );
        Ok(())
    }

    fn handle_node_ping_message(
        t_inner: Arc<Mutex<NetworkController>>,
        peer_addr: SocketAddr,
        nonce: u64,
        config: &Config,
    ) -> io::Result<()> {
        t_inner
            .lock()
            .map_err(to_io_err)?
            .read_ping(peer_addr, nonce, config)
    }

    fn recv_node_messages(
        &self,
        node_receiver: mpsc::Receiver<(SocketAddr, Message)>,
        config: Config,
    ) -> io::Result<JoinHandle<io::Result<()>>> {
        let inner = self.inner.clone();
        let handle = thread::spawn(move || -> io::Result<()> {
            loop {
                let t_inner: Arc<Mutex<NetworkController>> = inner.clone();
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
                    (peer_addr, Message::Failure(err)) => {
                        Self::handle_node_failure_message(t_inner, peer_addr, err)
                    }
                    (peer_addr, Message::Ping(nonce)) => {
                        Self::handle_node_ping_message(t_inner, peer_addr, nonce, &config)
                    }
                    _ => Ok(()), // unexpected messages were already filtered by node listeners
                } {
                    println!("Received unhandled error: {:?}", result);
                    return Err(result);
                }
            }
        });
        Ok(handle)
    }

    fn req_headers_periodically(&self, config: Config) -> io::Result<()> {
        let inner = self.inner.clone();
        thread::spawn(move || -> io::Result<()> {
            loop {
                println!("Requesting headers periodically...");
                let t_inner = inner.clone();
                let tallest_header = t_inner.lock().map_err(to_io_err)?.tallest_header;
                t_inner
                    .lock()
                    .map_err(to_io_err)?
                    .request_headers(tallest_header, &config)?;
                thread::sleep(std::time::Duration::from_secs(60));
            }
        });
        Ok(())
    }

    fn sync(&self, config: Config) -> io::Result<()> {
        let inner = self.inner.clone();
        thread::spawn(move || -> io::Result<()> {
            inner.lock().map_err(to_io_err)?.start_sync(&config)
        });
        Ok(())
    }

    pub fn start_sync(
        &self,
        node_receiver: mpsc::Receiver<(SocketAddr, Message)>,
        ui_receiver: Receiver<ModelRequest>,
        config: Config,
    ) -> io::Result<()> {
        self.recv_ui_messages(ui_receiver, config.clone())?;
        self.recv_node_messages(node_receiver, config.clone())?;
        self.sync(config.clone())?;
        self.req_headers_periodically(config)?;

        Ok(())
    }
}
