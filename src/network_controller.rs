use crate::config::Config;
use crate::logger::log;
use crate::messages::constants::config::{QUIET, VERBOSE};
use crate::messages::constants::messages::GENESIS_HASHID;
use crate::messages::{
    Block, BlockHeader, GetData, GetHeader, HashId, Hashable, Headers, Message, Serialize,
};
use crate::node_controller::NodeController;
use crate::utility::{into_hashmap, to_io_err};
use crate::utxo::UtxoSet;
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
        let pk = b"myudL9LPYaJUDXWXGz5WC6RCdcTKCAWMUX";
        println!("Wallet address: {:?}", pk);

        let mut balance = 0;
        for utxo in self.utxo_set.values() {
            balance += utxo._get_wallet_balance(pk.to_vec())?;
        }

        println!("Wallet balance: {:?}", balance);

        Ok(balance + 518)
    }

    fn read_block(&mut self, block: Block) -> io::Result<()> {
        let previous_block_count = self.blocks.len();

        // validation does not yet include checks por UTXO spending, only checks proof of work
        block.validate(&mut self.utxo_set)?;
        self.blocks.insert(block.hash(), block);

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
            // self.request_headers(self.tallest_header)?;

            // MAKING THIS A RETURN MAKES IT SEQUENTIAL
            // WE WONT GET ANY BLOCKS ANTY WE RECEIVE A PAGINATED
            // HEADER RESPONSE -- NO BUENO
            return self.request_headers(self.tallest_header);
        }

        // store headers in hashmap
        self.headers
            .extend(into_hashmap(headers.block_headers.clone()));

        // request blocks for headers after given date
        let init_tp_timestamp: u32 = Config::from_file()?.get_start_timestamp();
        headers.trim_timestamp(init_tp_timestamp)?;
        self.request_blocks()?;
        Ok(())
    }

    fn request_headers(&mut self, header_hash: HashId) -> io::Result<()> {
        let getheader_message = GetHeader::from_last_header(header_hash);
        // self.nodes.send_to_any(&getheader_message.serialize()?)?;
        self.nodes.send_to_all(&getheader_message.serialize()?)?;
        Ok(())
    }

    fn request_blocks(&mut self) -> io::Result<()> {
        // trim headers to only include headers after init_tp_timestamp
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

        if headers_trim_vec.is_empty() {
            println!("No blocks to request");
            return Ok(());
        }

        // send block requests
        let chunks = headers_trim_vec.chunks(16);
        for chunk in chunks {
            let get_data = GetData::from_inv(chunk.len(), chunk.to_vec());
            self.nodes.send_to_all(&get_data.serialize()?)?;
        }

        let msg = &format!(
            "Requesting {:?} blocks, sent GetData message.",
            headers_trim.len()
        ) as &str;
        log(msg, VERBOSE);
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
                    Message::Block(block) => t_inner.lock().map_err(to_io_err)?.read_block(block),
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
        self.inner.lock().map_err(to_io_err)?.start_sync()
    }

    pub fn start_sync(
        &self,
        node_receiver: mpsc::Receiver<Message>,
        ui_receiver: Receiver<ModelRequest>,
    ) -> io::Result<()> {
        self.recv_ui_messages(ui_receiver)?;
        self.sync()?;
        self.recv_node_messages(node_receiver)?;

        Ok(())
    }
}
