use crate::config::Config;
use crate::logger::log;
use crate::messages::constants::config::{QUIET, VERBOSE};
use crate::messages::constants::messages::GENESIS_HASHID;
use crate::messages::{
    Block, BlockHeader, GetData, GetHeader, HashId, Hashable, Headers, Message, Serialize,
};
use crate::node_controller::NodeController;
use crate::utility::{double_hash, encode_hex, into_hashmap, to_io_err};
use crate::utxo::Utxo;
use crate::utxo::UtxoSet;
use bitcoin_hashes::{sha256, Hash};
use gtk::gio::Resolver;
use std::borrow::Borrow;
use std::collections::HashMap;
use std::io;
use std::sync::mpsc::{self, Receiver};
use std::sync::Mutex;
// gtk imports
use crate::interface::{GtkMessage, ModelRequest};
use gtk::glib::Sender;
use std::sync::Arc;
use std::thread;

pub struct NetworkController {
    headers: HashMap<HashId, BlockHeader>,
    tallest_header: HashId,
    blocks: HashMap<HashId, Block>,
    utxo_set: UtxoSet,
    nodes: NodeController,
    ui_sender: Sender<GtkMessage>,
}

impl NetworkController {
    pub fn new(
        ui_sender: Sender<GtkMessage>,
        writer_end: mpsc::Sender<Message>,
    ) -> Result<Self, io::Error> {
        Ok(Self {
            headers: HashMap::new(),
            tallest_header: GENESIS_HASHID,
            blocks: HashMap::new(),
            utxo_set: HashMap::new(),
            nodes: NodeController::connect_to_peers(writer_end, ui_sender.clone())?,
            ui_sender,
        })
    }

    // HARDCODED NEEDS TO BE DYNAMIC
    fn _read_wallet_balance(&self) -> io::Result<i64> {
        let address = "myudL9LPYaJUDXWXGz5WC6RCdcTKCAWMUX";

        let mut balance = 0;

        match self.utxo_set.get(address) {
            Some(utxos) => {
                for (_, utxo) in utxos.into_iter() {
                    balance += utxo._value;
                }
            }
            None => {
                return Ok(balance);
            }
        }

        println!("Wallet balance: {:?}", balance);

        Ok(balance)
    }

    fn read_block(&mut self, block: Block) -> io::Result<()> {
        let ui_msg = format!("Reading blocks, {:?} days behind", block.get_days_old());
        self.ui_sender
            .send(GtkMessage::UpdateLabel((
                "status_bar".to_string(),
                ui_msg,
            )))
            .map_err(to_io_err)?;

        // validation does not yet include checks por UTXO spending, only checks proof of work
        block.validate(&mut self.utxo_set)?;

        // self.blocks.insert(block.hash(), block);
        // println!("New utxo set size: {:?}", self.utxo_set.len());

        // check if block is on disk, if not save it
        let block_header = encode_hex(&block.block_header.hash());
        let path = format!("tmp/blocks/block_{}.dat", block_header);
        if std::fs::read(path).is_err() {
            block.save_to_file()?;
        }

        Ok(())
    }

    fn read_headers(&mut self, mut headers: Headers) -> io::Result<()> {
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

        // request blocks for headers after given date
        let init_tp_timestamp: u32 = Config::from_file()?.get_start_timestamp();
        headers.trim_timestamp(init_tp_timestamp)?;
        self.request_blocks(headers)?;
        Ok(())
    }

    fn request_headers(&mut self, header_hash: HashId) -> io::Result<()> {
        let getheader_message = GetHeader::from_last_header(header_hash);
        self.nodes.send_to_any(&getheader_message.serialize()?)?;
        // self.nodes.send_to_all(&getheader_message.serialize()?)?;
        Ok(())
    }

    fn request_blocks(&mut self, headers: Headers) -> io::Result<()> {
        if headers.count == 0 {
            return Ok(());
        }

        // try to get blocks from disk first
        let mut non_disk_blocks: Vec<BlockHeader> = Vec::new();
        for header in headers.block_headers {
            let file_name = format!("block_{}.dat", encode_hex(&header.hash()));
            if let Ok(block) = Block::from_file(file_name) {
                self.read_block(block)?;
            } else {
                non_disk_blocks.push(header);
            }
        }

        // blocks not found on disk are requested to nodes
        let chunks = non_disk_blocks.chunks(20); // request 20 blocks at a time
        for chunk in chunks {
            let get_data = GetData::from_inv(chunk.len(), chunk.to_vec());
            self.nodes.send_to_any(&get_data.serialize()?)?;
        }
        log("Requesting blocks, sent GetData message.", VERBOSE);
        Ok(())
    }

    pub fn start_sync(&mut self) -> io::Result<()> {
        self.ui_sender
            .send(GtkMessage::UpdateLabel((
                "status_bar".to_string(),
                "Connected to network, starting sync".to_string(),
            )))
            .map_err(to_io_err)?;

        if let Ok(headers) = Headers::from_file("tmp/headers_backup.dat") {
            let last_header_hash = headers.last_header_hash();
            self.read_headers(headers)?;
            self.request_headers(last_header_hash)?;
        } else {
            self.request_headers(self.tallest_header)?;
        }

        Ok(())
    }
}

pub struct OuterNetworkController {
    inner: Arc<Mutex<NetworkController>>,
}

impl OuterNetworkController {
    pub fn new(
        ui_sender: Sender<GtkMessage>,
        writer_end: mpsc::Sender<Message>,
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
                        let val = inner_lock._read_wallet_balance()?;
                        inner_lock
                            .ui_sender
                            .send(GtkMessage::UpdateLabel((
                                "balance_available_val".to_string(),
                                format!("{:?}", val),
                            )))
                            .map_err(to_io_err)
                    }
                }?;
                // println!("Freeing lock on ui receiver");
            }
        });
        Ok(())
    }

    fn recv_node_messages(&self, node_receiver: mpsc::Receiver<Message>) -> io::Result<()> {
        let inner = self.inner.clone();
        thread::spawn(move || -> io::Result<()> {
            loop {
                let t_inner = inner.clone();
                match node_receiver.recv().map_err(to_io_err)? {
                    Message::Headers(headers) => {
                        // println!("Got lock on node receiver : read headers");
                        t_inner.lock().map_err(to_io_err)?.read_headers(headers)
                    }
                    Message::Block(block) => {
                        // println!("Got lock on node receiver : read block");
                        t_inner.lock().map_err(to_io_err)?.read_block(block)
                    }
                    Message::Failure() => {
                        // println!("Got lock on node receiver : read failure");
                        println!("Node is notifying me of a failure, should resend last request");
                        // aca deberiamos recibir el mensaje que fallo, y volver a enviarlo
                        Ok(())
                    }
                    _ => Err(io::Error::new(
                        io::ErrorKind::Other,
                        "Received unsupported message",
                    )),
                }?;
                // println!("Freeing lock on node receiver");
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
        node_receiver: mpsc::Receiver<Message>,
        ui_receiver: Receiver<ModelRequest>,
    ) -> io::Result<()> {
        self.recv_ui_messages(ui_receiver)?;
        self.recv_node_messages(node_receiver)?;
        self.sync()?;

        Ok(())
    }
}
