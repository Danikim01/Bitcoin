use crate::config::Config;
use crate::logger::log;
use crate::messages::constants::config::{QUIET, VERBOSE};
use crate::messages::constants::messages::GENESIS_HASHID;
use crate::messages::{
    Block, BlockHeader, GetData, GetHeader, HashId, Hashable, Headers, Message, Serialize,
};
use crate::node_controller::NodeController;
use crate::utility::{double_hash, into_hashmap, to_io_err};
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
        println!("Reading block: {:?}", block.block_header.timestamp);
        let previous_block_count = self.blocks.len();

        // validation does not yet include checks por UTXO spending, only checks proof of work
        block.validate(&mut self.utxo_set)?;

        // self.blocks.insert(block.hash(), block);
        println!("New utxo set size: {:?}", self.utxo_set.len());

        if self.blocks.len() == previous_block_count {
            return Ok(());
        }

        // if prev_block_hash points to unvalidated block, validation should wait for the prev block,
        // probably adding cur block to a vec of blocks pending validation
        if self.blocks.len() % 100 == 0 {
            let msg = &format!("Received block. New block count: {:?}", self.blocks.len()) as &str;
            // log(msg, QUIET);
        } else {
            let msg = &format!("Received block. New block count: {:?}", self.blocks.len()) as &str;
            // log(msg, VERBOSE);
            self.ui_sender
                .send(GtkMessage::UpdateLabel((
                    "status_bar".to_string(),
                    msg.to_string(),
                )))
                .map_err(to_io_err)?;
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
        let chunks = headers.block_headers.chunks(20); // request 20 blocks at a time
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
            self.tallest_header = headers.last_header_hash();
            self.headers = into_hashmap(headers.block_headers);
        }

        // START OF FIX
        let init_tp_timestamp: u32 = Config::from_file()?.get_start_timestamp();
        let mut headers_trim = self.headers.clone();
        headers_trim = headers_trim
            .into_iter()
            .filter(|(_, v)| v.timestamp >= init_tp_timestamp)
            .collect();

        // now trim headers to only include the ones that are not in the blocks hashmap
        headers_trim = headers_trim
            .into_iter()
            .filter(|(k, _)| !self.blocks.contains_key(k))
            .collect();

        // get all values from headers trim into a vec
        let mut headers_trim_vec = Vec::new();
        for header in headers_trim.values() {
            headers_trim_vec.push(header.clone());
        }
        // send block requests
        let chunks = headers_trim_vec.chunks(16);
        for chunk in chunks {
            let get_data = GetData::from_inv(chunk.len(), chunk.to_vec());
            self.nodes.send_to_any(&get_data.serialize()?)?;
        }
        // END OF FIX

        self.request_headers(self.tallest_header)?;

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
                        println!("Received request for wallet balance");
                        let inner_lock = t_inner.lock().map_err(to_io_err)?;
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
                        t_inner.lock().map_err(to_io_err)?.read_headers(headers)
                    }
                    Message::Block(block) => {
                        t_inner.lock().map_err(to_io_err)?.read_block(block)
                    },
                    Message::Failure() => {
                        println!("Node is notifying me of a failure, should resend last request");
                        // aca deberiamos recibir el mensaje que fallo, y volver a enviarlo
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
        thread::spawn(move || -> io::Result<()> {
            inner.lock().map_err(to_io_err)?.start_sync()
        });
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
