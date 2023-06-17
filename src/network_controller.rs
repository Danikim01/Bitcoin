use crate::config::Config;
use crate::logger::log;
use crate::messages::constants::config::VERBOSE;
use crate::messages::constants::messages::GENESIS_HASHID;
use crate::messages::{
    Block, BlockHeader, BlockSet, GetData, GetHeader, HashId, Hashable, Headers, InvType,
    Inventory, Message, Serialize,
};
use crate::node_controller::NodeController;
use crate::utility::{into_hashmap, to_io_err};
use crate::utxo::UtxoSet;
use crate::wallet::Wallet;
use std::collections::HashMap;
use std::io;
use std::sync::mpsc::{self, Receiver};
use std::sync::Mutex;
// gtk imports
use crate::interface::{GtkMessage, ModelRequest};
use gtk::gdk::keys::constants::P;
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

    pub fn update_status_bar(&self, msg: String) -> io::Result<()> {
        self.ui_sender
            .send(GtkMessage::UpdateLabel(("status_bar".to_string(), msg)))
            .map_err(to_io_err)
    }

    fn read_wallet_balance(&self) -> io::Result<u64> {
        let balance = self.utxo_set.get_wallet_balance(&self.wallet.address)?;
        println!("Wallet balance: {:?}", balance);
        self.ui_sender
            .send(GtkMessage::UpdateLabel((
                "balance_available_val".to_string(),
                format!("{:?}", balance),
            )))
            .map_err(to_io_err)?;

        Ok(balance)
    }

    fn read_block(&mut self, block: Block) -> io::Result<()> {
        if self.blocks.contains_key(&block.hash()) {
            return Ok(());
        }

        let days_old = block.get_days_old();
        if days_old > 0 {
            self.update_status_bar(format!(
                "Reading blocks, {:?} days behind",
                block.get_days_old()
            ))?;
        } else {
            self.update_status_bar("Up to date".to_string())?;
        }

        // validation does not yet include checks por UTXO spending, only checks proof of work
        block.validate(&mut self.utxo_set)?;

        Ok(())
    }

    fn read_block_unsafe(&mut self, block: Block) -> io::Result<()> {
        if self.blocks.contains_key(&block.hash()) {
            return Ok(());
        }

        self.update_status_bar("Reading backup blocks".to_string())?;

        // validation does not yet include checks por UTXO spending, only checks proof of work
        block.validate_unsafe(&mut self.utxo_set)?;
        self.blocks.insert(block.hash(), block);

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
            self.update_status_bar("Found blocks backup file, reading blocks...".to_string())?;
            for (_, block) in blocks.into_iter() {
                self.read_block_unsafe(block)?;
            }
            self.update_status_bar("Read blocks from backup file.".to_string())?;
        }

        // attempt to read headers from backup file
        if let Ok(mut headers) = Headers::from_file("tmp/headers_backup.dat") {
            self.update_status_bar("Reading headers from backup file...".to_string())?;
            self.tallest_header = headers.last_header_hash();
            self.headers
                .extend(into_hashmap(headers.block_headers.clone()));
            self.read_headers(&headers)?;

            // find missing headers in blocks and then request them
            let init_tp_timestamp: u32 = Config::from_file()?.get_start_timestamp();
            headers.trim_timestamp(init_tp_timestamp)?;
            let missing_headers = self.get_missing_headers(&headers)?;

            if !missing_headers.is_empty() {
                self.update_status_bar(format!(
                    "Found {} missing blocks in backup file, requesting them...",
                    missing_headers.len()
                ))?;
                self.request_blocks(Headers::from_block_headers(missing_headers))?;
            }

            self.update_status_bar("Read headers from backup file.".to_string())?;
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

    fn recv_ui_messages(&self, ui_receiver: Receiver<ModelRequest>) -> io::Result<()> {
        let inner = self.inner.clone();
        thread::spawn(move || -> io::Result<()> {
            loop {
                let t_inner = inner.clone();
                match ui_receiver.recv().map_err(to_io_err)? {
                    ModelRequest::GetWalletBalance => {
                        // println!("Received request for wallet balance");
                        let inner_lock = t_inner.lock().map_err(to_io_err)?;
                        // println!("Got lock on ui receiver");
                        inner_lock.read_wallet_balance()
                    }
                }?;
                // println!("Freeing lock on ui receiver");
            }
        });
        Ok(())
    }

    fn recv_node_messages(
        &self,
        node_receiver: mpsc::Receiver<(SocketAddr, Message)>,
    ) -> io::Result<()> {
        let inner = self.inner.clone();
        thread::spawn(move || -> io::Result<()> {
            loop {
                let t_inner = inner.clone();
                match node_receiver.recv().map_err(to_io_err)? {
                    (_, Message::Headers(headers)) => {
                        t_inner.lock().map_err(to_io_err)?.read_headers(&headers)
                    }
                    (_, Message::Block(block)) => t_inner
                        .lock()
                        .map_err(to_io_err)?
                        .read_block_from_node(block),
                    (peer_addr, Message::Inv(inventories)) => t_inner
                        .lock()
                        .map_err(to_io_err)?
                        .read_inventories(peer_addr, inventories),
                    (peer_addr, Message::Failure()) => {
                        println!(
                            "Node {:?} is notifying me of a failure, should resend last request",
                            peer_addr
                        );
                        Ok(())
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

        Ok(())
    }
}
