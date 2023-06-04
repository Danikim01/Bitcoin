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
        if self.blocks.len() % 100 == 0 {
            log(
                &format!("Received block. New block count: {:?}", self.blocks.len()) as &str,
                QUIET,
            );
        } else {
            log(
                &format!("Received block. New block count: {:?}", self.blocks.len()) as &str,
                VERBOSE,
            );
        }
        // if prev_block_hash points to unvalidated block, validation should wait for the prev block,
        // probably adding cur block to a vec of blocks pending validation

        // validation does not yet include checks por UTXO spending, only checks proof of work
        block.validate(&mut self.utxo_set)?;
        self.blocks.insert(block.hash(), block);
        Ok(())
    }

    fn read_headers(&mut self, mut headers: Headers) -> io::Result<()> {
        log(
            &format!(
                "Received header. New header count: {:?}",
                self.headers.len()
            ),
            VERBOSE,
        );
        log(
            &format!(
                "Received header. New header count: {:?}",
                self.headers.len()
            ),
            VERBOSE,
        );
        // request more headers
        self.tallest_header = headers.last_header_hash();
        if headers.is_paginated() {
            self.request_headers(self.tallest_header)?;
        }

        // store headers in hashmap
        self.headers
            .extend(into_hashmap(headers.block_headers.clone()));

        // request blocks for headers after given date
        let init_tp_timestamp: u32 = Config::from_file()?.get_start_timestamp();
        headers.trim_timestamp(init_tp_timestamp)?;
        self.request_blocks(headers)?;
        Ok(())
    }

    fn request_headers(&mut self, header_hash: HashId) -> io::Result<()> {
        let getheader_message = GetHeader::from_last_header(header_hash);
        self.nodes.send_to_any(&getheader_message.serialize()?)?;
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
                            .send(GtkMessage::UpdateBotton(format!("{:?}", val)))
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
        node_receiver: mpsc::Receiver<Message>,
        ui_receiver: Receiver<ModelRequest>,
    ) -> io::Result<()> {
        self.recv_node_messages(node_receiver)?;
        self.recv_ui_messages(ui_receiver)?;
        self.sync()?;

        Ok(())
    }
}
