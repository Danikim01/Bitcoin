use crate::config::Config;
use crate::messages::constants::messages::GENESIS_HASHID;
use crate::messages::{
    Block, BlockHeader, GetData, GetHeader, HashId, Hashable, Headers, Message, Serialize,
};
use crate::node_controller::NodeController;
use crate::utility::{into_hashmap, to_io_err};
use crate::utxo::{Utxo, UtxoId};
use std::collections::HashMap;
use std::io;
use std::sync::mpsc;

pub struct NetworkController {
    headers: HashMap<HashId, BlockHeader>,
    tallest_header: HashId,
    blocks: HashMap<HashId, Block>,
    utxo_set: HashMap<UtxoId, Utxo>,
    reader: mpsc::Receiver<Message>,
    nodes: NodeController,
}

impl NetworkController {
    pub fn new() -> Result<Self, io::Error> {
        let (writer_end, reader_end) = mpsc::channel();
        Ok(Self {
            headers: HashMap::new(),
            tallest_header: GENESIS_HASHID,
            blocks: HashMap::new(),
            utxo_set: HashMap::new(),
            reader: reader_end,
            nodes: NodeController::connect_to_peers(writer_end)?,
        })
    }

    fn recv_messages(&mut self) -> io::Result<()> {
        loop {
            match self.reader.recv().map_err(to_io_err)? {
                Message::Headers(headers) => self.read_headers(headers),
                Message::Block(block) => self.read_block(block),
                _ => {
                    return Err(io::Error::new(
                        io::ErrorKind::Other,
                        "Received unsupported message",
                    ))
                }
            }?;
        }
    }

    fn read_block(&mut self, block: Block) -> io::Result<()> {
        println!("Received block. New block count: {:?}", self.blocks.len());
        // if prev_block_hash points to unvalidated block, validation should wait for the prev block,
        // probably adding cur block to a vec of blocks pending validation
    
        // validation does not yet include checks por UTXO spending, only checks proof of work
        block.validate(&mut self.utxo_set)?;
        self.blocks.insert(block.hash(), block);
        Ok(())
    }

    fn read_headers(&mut self, mut headers: Headers) -> io::Result<()> {
        println!(
            "Received header. New header count: {:?}",
            self.headers.len()
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
        println!("Requesting blocks, sent GetData message.");
        Ok(())
    }

    pub fn start_sync(&mut self) -> io::Result<()> {
        if let Ok(headers) = Headers::from_file("tmp/headers_backup.dat") {
            self.tallest_header = headers.last_header_hash();
            self.headers = into_hashmap(headers.block_headers);
        }
        self.request_headers(self.tallest_header)?;
        self.recv_messages()?;
        Ok(())
    }
}
