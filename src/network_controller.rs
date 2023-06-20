use crate::config::Config;
use crate::logger::log;
use crate::messages::constants::commands::{PONG, TX};
use crate::messages::constants::config::{MAGIC, VERBOSE};
use crate::messages::constants::messages::GENESIS_HASHID;
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
use std::collections::HashMap;
use std::io;
use std::sync::mpsc::{self, Receiver};
use std::sync::Mutex;
// gtk imports
use crate::interface::components::send_panel::TransactionInfo;
use crate::interface::{update_ui_label, GtkMessage, ModelRequest};
use gtk::glib::Sender;
use std::net::SocketAddr;
use std::sync::Arc;
use std::thread;
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
            wallet: Wallet::login(),
        })
    }

    pub fn update_ui_status_bar(&self, msg: String) -> io::Result<()> {
        update_ui_label(self.ui_sender.clone(), "status_bar".to_string(), msg)
    }

    fn update_ui_balance(&self) -> io::Result<()> {
        let (balance, pending) = self.read_wallet_balance()?;
        self.ui_sender
            .send(GtkMessage::UpdateBalance((balance, pending)))
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
            return Ok(());
        }

        let days_old = block.get_days_old();
        if days_old > 0 {
            self.update_ui_status_bar(format!(
                "Reading blocks, {:?} days behind",
                block.get_days_old()
            ))?;
        } else {
            self.update_ui_status_bar("Up to date".to_string())?;
        }

        // validation does not yet include checks por UTXO spending, only checks proof of work
        block.validate(&mut self.utxo_set)?;

        // get wallet balance and update UI
        self.update_ui_balance()?;
        Ok(())
    }

    fn read_block_unsafe(&mut self, block: Block) -> io::Result<()> {
        if self.blocks.contains_key(&block.hash()) {
            return Ok(());
        }

        self.update_ui_status_bar("Reading backup blocks...".to_string())?;

        // validation does not yet include checks por UTXO spending, only checks proof of work
        block.validate_unsafe(&mut self.utxo_set)?;
        self.blocks.insert(block.hash(), block);

        // get wallet balance and update UI
        self.update_ui_balance()?;

        Ok(())
    }

    fn read_block_from_node(&mut self, block: Block) -> io::Result<()> {
        self.read_block(block.clone())?;

        block.save_to_file("tmp/blocks_backup.dat")?;
        self.blocks.insert(block.hash(), block);

        Ok(())
    }

    fn request_blocks(&mut self, headers: Headers) -> io::Result<()> {
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

    /// requests block for headers after given timestamp
    fn request_blocks_from(&mut self, mut headers: Headers, timestamp: u32) -> io::Result<()> {
        headers.trim_timestamp(timestamp)?;

        let mut needed_blocks = Vec::new();

        for header in headers.block_headers.clone() {
            if !self.blocks.contains_key(&header.hash()) {
                needed_blocks.push(header);
            }
        }

        self.request_blocks(Headers::from_block_headers(needed_blocks))?;
        Ok(())
    }

    fn read_headers(&mut self, headers: &Headers) -> io::Result<()> {
        let previous_header_count = self.headers.len();

        // store headers in hashmap
        self.headers
            .extend(into_hashmap(headers.block_headers.clone()));

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

        // request more headers
        self.tallest_header = headers.last_header_hash(); // last to get doesn't have to be tallest -- check this
        if headers.is_paginated() {
            self.request_headers(self.tallest_header)?;
        }

        // save headers to file
        headers.save_to_file("tmp/headers_backup.dat")?;

        // request blocks
        let init_tp_timestamp: u32 = Config::from_file()?.get_start_timestamp();
        self.request_blocks_from(headers.clone(), init_tp_timestamp)?;

        Ok(())
    }

    fn request_headers(&mut self, header_hash: HashId) -> io::Result<()> {
        let getheader_message = GetHeader::from_last_header(header_hash);
        self.nodes.send_to_any(&getheader_message.serialize()?)?;
        // self.nodes.send_to_all(&getheader_message.serialize()?)?;
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
            transaction.generate_utxo(&mut self.utxo_set, TransactionOrigin::Pending)?;

            // get wallet balance and update UI
            self.update_ui_balance()?;
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

    pub fn generate_transaction(&mut self, details: TransactionInfo) -> io::Result<()> {
        let tx: RawTransaction = self
            .wallet
            .generate_transaction(&mut self.utxo_set, details)?;

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

        Ok(())
    }

    fn get_missing_headers(&self, headers: &Headers) -> io::Result<Vec<BlockHeader>> {
        let mut missing_headers = Vec::new();

        for header in headers.clone().block_headers {
            if !self.blocks.contains_key(&header.hash()) {
                missing_headers.push(header);
            }
        }

        Ok(missing_headers)
    }

    pub fn start_sync(&mut self) -> io::Result<()> {
        // attempt to read blocks from backup file
        if let Ok(blocks) = Block::all_from_file("tmp/blocks_backup.dat") {
            self.update_ui_status_bar("Found blocks backup file, reading blocks...".to_string())?;
            for (_, block) in blocks.into_iter() {
                self.read_block_unsafe(block)?;
            }
            self.update_ui_status_bar("Read blocks from backup file.".to_string())?;
        }

        // attempt to read headers from backup file
        if let Ok(mut headers) = Headers::from_file("tmp/headers_backup.dat") {
            self.update_ui_status_bar("Reading headers from backup file...".to_string())?;
            self.tallest_header = headers.last_header_hash();
            self.headers
                .extend(into_hashmap(headers.block_headers.clone()));
            self.read_headers(&headers)?;

            // find missing headers in blocks and then request them
            let init_tp_timestamp: u32 = Config::from_file()?.get_start_timestamp();
            headers.trim_timestamp(init_tp_timestamp)?;
            let missing_headers = self.get_missing_headers(&headers)?;

            if !missing_headers.is_empty() {
                self.update_ui_status_bar(format!(
                    "Found {} missing blocks in backup file, requesting them...",
                    missing_headers.len()
                ))?;
                self.request_blocks(Headers::from_block_headers(missing_headers))?;
            }

            self.update_ui_status_bar("Read headers from backup file.".to_string())?;
        } // else init ibd

        self.request_headers(self.tallest_header)?; // with ibd this goes on the else clause

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
    ) -> Result<Self, io::Error> {
        let inner = Arc::new(Mutex::new(NetworkController::new(ui_sender, writer_end)?));
        Ok(Self { inner })
    }

    fn handle_ui_generate_transaction(
        t_inner: Arc<Mutex<NetworkController>>,
        transaction_info: TransactionInfo,
    ) -> io::Result<()> {
        let mut inner_lock = t_inner.lock().map_err(to_io_err)?;
        inner_lock.generate_transaction(transaction_info)?;
        Ok(())
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
        t_inner.lock().map_err(to_io_err)?.read_headers(&headers)
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
    ) -> io::Result<()> {
        let inner = self.inner.clone();
        thread::spawn(move || -> io::Result<()> {
            loop {
                let t_inner: Arc<Mutex<NetworkController>> = inner.clone();
                match node_receiver.recv().map_err(to_io_err)? {
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
                }?;
            }
        });
        Ok(())
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
