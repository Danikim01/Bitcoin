use crate::config::Config;
use crate::logger::log;
use crate::messages::constants::{
    commands::{PONG, TX},
    config::{MAGIC, VERBOSE},
    messages::GENESIS_HASHID,
};
use crate::messages::{
    Block, BlockHeader, BlockSet, ErrorType, GetData, GetHeader, HashId, Hashable, Headers,
    InvType, Inventory, Message, MessageHeader, Serialize,
};
use crate::node_controller::NodeController;
use crate::raw_transaction::{RawTransaction, TransactionOrigin};
use crate::utility::{double_hash, into_hashmap, to_io_err};
use crate::utxo::UtxoSet;
use crate::wallet::Wallet;
use bitcoin_hashes::Hash;
use std::collections::{hash_map::Entry::Vacant, HashMap};
use std::io;
use std::sync::mpsc::{self, Receiver};
use std::sync::Mutex;

// gtk imports
use crate::interface::components::overview_panel::TransactionDisplayInfo;
use crate::interface::components::send_panel::TransactionInfo;
use crate::interface::{update_ui_label, update_ui_status_bar, GtkMessage, ModelRequest};
use gtk::glib::Sender;
use std::net::SocketAddr;
use std::sync::Arc;
use std::thread::{self, JoinHandle};

/// Structs of the network controller (main controller of the program)
pub struct NetworkController {
    headers: HashMap<HashId, BlockHeader>,
    tallest_header: HashId,
    blocks: BlockSet,
    utxo_set: UtxoSet,
    nodes: NodeController,
    ui_sender: Sender<GtkMessage>,
    wallet: Wallet,
}

impl NetworkController {
    /// Creates a new network controller from the given sender and writer
    pub fn new(
        ui_sender: Sender<GtkMessage>,
        writer_end: mpsc::Sender<(SocketAddr, Message)>,
    ) -> Result<Self, io::Error> {
        Ok(Self {
            headers: HashMap::new(),
            tallest_header: GENESIS_HASHID,
            blocks: HashMap::new(),
            utxo_set: UtxoSet::new(),
            nodes: NodeController::connect_to_peers(writer_end, ui_sender.clone())?,
            ui_sender,
            wallet: Wallet::login()?,
        })
    }

    fn update_ui_balance(&self) -> io::Result<()> {
        let (balance, pending) = self.read_wallet_balance()?;
        self.ui_sender
            .send(GtkMessage::UpdateBalance((balance, pending)))
            .map_err(to_io_err)
    }

    fn update_ui_overview(&self, transaction: RawTransaction) -> io::Result<()> {
        let transaction_info: TransactionDisplayInfo =
            transaction.transaction_info_for(&self.wallet.address, transaction.lock_time);
        self.ui_sender
            .send(GtkMessage::UpdateOverviewTransactions((
                transaction_info,
                TransactionOrigin::Pending,
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

    fn read_block(&mut self, block: Block) -> io::Result<()> {
        if self.blocks.contains_key(&block.hash()) {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "This block has already been received before",
            ));
        }

        let days_old = block.get_days_old();
        if days_old > 0 {
            update_ui_status_bar(
                &self.ui_sender,
                format!("Reading blocks, {:?} days behind", block.get_days_old()),
            )?;
        } else {
            update_ui_status_bar(&self.ui_sender, "Up to date".to_string())?;
        }

        block.validate(
            &mut self.utxo_set,
            Some(&self.ui_sender),
            Some(&self.wallet.address),
        )?;

        self.update_ui_balance()?;
        Ok(())
    }

    fn read_block_from_backup(&mut self, block: Block) -> io::Result<()> {
        if self.blocks.contains_key(&block.hash()) {
            return Ok(());
        }

        update_ui_status_bar(&self.ui_sender, "Reading backup blocks...".to_string())?;

        block.validate_from_backup(
            &mut self.utxo_set,
            Some(&self.ui_sender),
            Some(&self.wallet.address),
        )?;

        self.blocks.insert(block.hash(), block);

        self.update_ui_balance()?;

        Ok(())
    }

    fn read_block_from_node(&mut self, block: Block) -> io::Result<()> {
        if self.read_block(block.clone()).is_err() {
            return Ok(()); // ignore invalid blocks
        }

        block.save_to_file("tmp/blocks_backup.dat")?;
        self.blocks.insert(block.hash(), block);
        Ok(())
    }

    fn request_blocks(&mut self, headers: &mut Headers) -> io::Result<()> {
        if headers.count == 0 {
            return Ok(());
        }

        let chunks = headers.block_headers.chunks(20); // request 20 blocks at a time
        for chunk in chunks {
            let get_data = GetData::from_inv(chunk.len(), chunk.to_vec());
            self.nodes.send_to_any(&get_data.serialize()?)?;
        }
        log("Requesting blocks, sent GetData message.", VERBOSE);
        Ok(())
    }

    fn retain_missing_headers(&self, headers: &mut Headers) {
        headers
            .block_headers
            .retain(|header| !self.blocks.contains_key(&header.hash()));
    }

    /// requests block for headers after given timestamp
    fn request_blocks_from(&mut self, mut headers: Headers, timestamp: u32) -> io::Result<()> {
        headers.trim_timestamp(timestamp);
        self.retain_missing_headers(&mut headers);
        self.request_blocks(&mut headers)?;
        Ok(())
    }

    fn read_headers(&mut self, headers: Headers) -> io::Result<()> {
        let previous_header_count = self.headers.len();
        let init_tp_timestamp: u32 = Config::from_file()?.get_start_timestamp();
        self.request_blocks_from(headers.clone(), init_tp_timestamp)?;
        // save values to variables before consuming headers
        let last_header = headers.last_header_hash();
        let is_paginated = headers.is_paginated();

        // store headers in hashmap, consuming headers
        let headers_hashmap = into_hashmap(headers.block_headers);
        for (header_hash, header) in headers_hashmap {
            if let Vacant(entry) = self.headers.entry(header_hash) {
                header.save_to_file("tmp/headers_backup.dat")?;
                entry.insert(header);
            }
        }
        if self.headers.len() == previous_header_count {
            return Ok(());
        }
        log(
            &format!(
                "Received header. New header count: {:?}",
                self.headers.len()
            ),
            VERBOSE,
        );
        // request next headers, and blocks for recieved headers
        self.tallest_header = last_header;
        if is_paginated {
            self.request_headers(self.tallest_header)?;
        }
        Ok(())
    }

    fn request_headers(&mut self, header_hash: HashId) -> io::Result<()> {
        let getheader_message = GetHeader::from_last_header(header_hash);
        self.nodes.send_to_all(&getheader_message.serialize()?)?;
        Ok(())
    }

    /// read inv message from peer, if it contains tx invs, request txs to same peer
    fn read_inventories(
        &mut self,
        peer: SocketAddr,
        inventories: Vec<Inventory>,
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
            .send_to_specific(&peer, &getdata_message.serialize()?)?;
        Ok(())
    }

    fn read_pending_tx(&mut self, transaction: RawTransaction) -> io::Result<()> {
        if transaction.address_is_involved(&self.wallet.address) {
            transaction.generate_utxo(
                &mut self.utxo_set,
                TransactionOrigin::Pending,
                Some(&self.ui_sender),
                None,
            )?;

            // get wallet balance and update UI
            self.update_ui_balance()?;

            // add transaction to overview
            self.update_ui_overview(transaction)?;
        }
        Ok(())
    }

    fn read_ping(&mut self, peer_addr: SocketAddr, nonce: u64) -> io::Result<()> {
        let payload = nonce.to_le_bytes();
        let hash = double_hash(&payload);
        let checksum: [u8; 4] = [hash[0], hash[1], hash[2], hash[3]];
        let message_header =
            MessageHeader::new(MAGIC, PONG.to_string(), payload.len() as u32, checksum);
        let mut to_send = Vec::new();
        to_send.extend_from_slice(&message_header.serialize()?);
        to_send.extend_from_slice(&payload);
        self.nodes.send_to_specific(&peer_addr, &to_send)?;
        Ok(())
    }

    /// Generates a transaction and broadcasts it to all peers given the transaction details
    pub fn generate_transaction(
        &mut self,
        details: TransactionInfo,
    ) -> io::Result<TransactionInfo> {
        let tx: RawTransaction = self
            .wallet
            .generate_transaction(&mut self.utxo_set, details.clone())?;

        // broadcast tx
        let tx_hash = double_hash(&tx.serialize()).to_byte_array();

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
        self.nodes.send_to_all(&bytes)?;

        Ok(details)
    }

    /// Starts the sync process by requesting headers from all peers from the last known header (or genesis block) to the current time
    /// If a backup file is found, it will read the blocks and headers from the backup file
    pub fn start_sync(&mut self) -> io::Result<()> {
        // attempt to read blocks from backup file
        if let Ok(blocks) = Block::all_from_file("tmp/blocks_backup.dat") {
            update_ui_status_bar(
                &self.ui_sender,
                "Found blocks backup file, reading blocks...".to_string(),
            )?;
            for (_, block) in blocks.into_iter() {
                self.read_block_from_backup(block)?;
            }
            update_ui_status_bar(&self.ui_sender, "Read blocks from backup file.".to_string())?;
        }

        // attempt to read headers from backup file
        if let Ok(mut headers) = Headers::from_file("tmp/headers_backup.dat") {
            update_ui_status_bar(
                &self.ui_sender,
                "Reading headers from backup file...".to_string(),
            )?;
            self.tallest_header = headers.last_header_hash();
            self.headers = into_hashmap(headers.clone().block_headers);

            // find headers for which we don't have the blocks and then request them
            let init_tp_timestamp: u32 = Config::from_file()?.get_start_timestamp();
            headers.trim_timestamp(init_tp_timestamp);

            update_ui_status_bar(
                &self.ui_sender,
                "Read headers from backup file.".to_string(),
            )?;
        }

        self.request_headers(self.tallest_header)?; // with ibd this goes on the else clause

        Ok(())
    }
}

/// NetworkController is a wrapper around the inner NetworkController in order to allow for safe multithreading
pub struct OuterNetworkController {
    inner: Arc<Mutex<NetworkController>>,
}

impl OuterNetworkController {
    /// Creates a new OuterNetworkController given a ui_sender and a writer
    pub fn new(
        ui_sender: Sender<GtkMessage>,
        writer_end: mpsc::Sender<(SocketAddr, Message)>,
    ) -> Result<Self, io::Error> {
        let inner = Arc::new(Mutex::new(NetworkController::new(ui_sender, writer_end)?));
        Ok(Self { inner })
    }

    fn handle_ui_generate_transaction(
        t_inner: Arc<Mutex<NetworkController>>,
        transaction_info: TransactionInfo,
    ) -> io::Result<()> {
        let mut inner_lock = t_inner.lock().map_err(to_io_err)?;
        let result: Result<TransactionInfo, io::Error> =
            inner_lock.generate_transaction(transaction_info);
        inner_lock
            .ui_sender
            .send(GtkMessage::TransactionInfo(result))
            .map_err(to_io_err)
    }

    fn recv_ui_messages(&self, ui_receiver: Receiver<ModelRequest>) -> io::Result<()> {
        let inner = self.inner.clone();
        thread::spawn(move || -> io::Result<()> {
            loop {
                let t_inner: Arc<Mutex<NetworkController>> = inner.clone();
                match ui_receiver.recv().map_err(to_io_err)? {
                    ModelRequest::GenerateTransaction(transaction_info) => {
                        Self::handle_ui_generate_transaction(t_inner, transaction_info)
                    }
                }?;
            }
        });
        Ok(())
    }

    fn handle_node_block_message(
        t_inner: Arc<Mutex<NetworkController>>,
        block: Block,
    ) -> io::Result<()> {
        t_inner
            .lock()
            .map_err(to_io_err)?
            .read_block_from_node(block)
    }

    fn handle_node_headers_message(
        t_inner: Arc<Mutex<NetworkController>>,
        headers: Headers,
    ) -> io::Result<()> {
        t_inner.lock().map_err(to_io_err)?.read_headers(headers)
    }

    fn handle_node_inv_message(
        t_inner: Arc<Mutex<NetworkController>>,
        peer_addr: SocketAddr,
        inventories: Vec<Inventory>,
    ) -> io::Result<()> {
        t_inner
            .lock()
            .map_err(to_io_err)?
            .read_inventories(peer_addr, inventories)
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
    ) -> io::Result<()> {
        t_inner
            .lock()
            .map_err(to_io_err)?
            .read_ping(peer_addr, nonce)
    }

    fn recv_node_messages(
        &self,
        node_receiver: mpsc::Receiver<(SocketAddr, Message)>,
    ) -> io::Result<JoinHandle<io::Result<()>>> {
        let inner = self.inner.clone();
        let handle = thread::spawn(move || -> io::Result<()> {
            loop {
                let t_inner: Arc<Mutex<NetworkController>> = inner.clone();
                if let Err(result) = match node_receiver.recv().map_err(to_io_err)? {
                    (_, Message::Headers(headers)) => {
                        Self::handle_node_headers_message(t_inner, headers)
                    }
                    (_, Message::Block(block)) => Self::handle_node_block_message(t_inner, block),
                    (peer_addr, Message::Inv(inventories)) => {
                        Self::handle_node_inv_message(t_inner, peer_addr, inventories)
                    }
                    (_, Message::Transaction(tx)) => Self::handle_node_tx_message(t_inner, tx),
                    (peer_addr, Message::Failure(err)) => {
                        Self::handle_node_failure_message(t_inner, peer_addr, err)
                    }
                    (peer_addr, Message::Ping(nonce)) => {
                        Self::handle_node_ping_message(t_inner, peer_addr, nonce)
                    }
                    _ => Err(io::Error::new(
                        io::ErrorKind::Other,
                        "Received unsupported message",
                    )),
                } {
                    // closes all threads if it fails to read from channel
                    println!("Result: {:?}", result);
                }
            }
        });
        Ok(handle)
    }

    fn req_headers_periodically(&self) -> io::Result<()> {
        let inner = self.inner.clone();
        thread::spawn(move || -> io::Result<()> {
            loop {
                let t_inner = inner.clone();
                let tallest_header = t_inner.lock().map_err(to_io_err)?.tallest_header;
                t_inner
                    .lock()
                    .map_err(to_io_err)?
                    .request_headers(tallest_header)?;
                thread::sleep(std::time::Duration::from_secs(60));
            }
        });
        Ok(())
    }

    fn sync(&self) -> io::Result<()> {
        let inner = self.inner.clone();
        thread::spawn(move || -> io::Result<()> { inner.lock().map_err(to_io_err)?.start_sync() });
        Ok(())
    }

    /// Starts the sync process and requests headers periodically.
    pub fn start_sync(
        &self,
        node_receiver: mpsc::Receiver<(SocketAddr, Message)>,
        ui_receiver: Receiver<ModelRequest>,
    ) -> io::Result<()> {
        self.recv_ui_messages(ui_receiver)?;
        self.recv_node_messages(node_receiver)?;
        self.sync()?;
        self.req_headers_periodically()?;

        Ok(())
    }
}
