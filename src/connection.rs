use crate::config::Config;
use crate::messages::constants::{
    header_constants::MAX_HEADER,
    messages::{GENESIS_HASHID, MAX_INV_SIZE},
};
use crate::messages::{
    Block, BlockHeader, GetData, GetHeader, HashId, Hashable, Headers, Message, Serialize,
};
use crate::node::Node;
use crate::utility::{into_hashmap, to_buckets, to_io_err};
use std::collections::HashMap;
use std::io;
use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::mpsc;

pub struct NetworkController {
    headers: HashMap<HashId, BlockHeader>,
    tallest_header: HashId,
    blocks: HashMap<HashId, Block>,
    reader: mpsc::Receiver<Message>,
    nodes: Vec<Node>,
}

fn find_nodes() -> Result<std::vec::IntoIter<SocketAddr>, io::Error> {
    let node_discovery_hostname = Config::from_file()?.get_hostname();
    node_discovery_hostname.to_socket_addrs()
}

impl NetworkController {
    pub fn connect_to_network() -> Result<Self, io::Error> {
        let node_addresses = find_nodes()?;
        let mut nodes = Vec::new();
        let (writer_end, reader_end) = mpsc::channel();
        for node_addr in node_addresses {
            match Node::try_from_addr(node_addr, writer_end.clone()) {
                Ok(node) => nodes.push(node),
                Err(..) => continue,
            }
            break;
        }
        Ok(Self {
            headers: HashMap::new(),
            tallest_header: GENESIS_HASHID,
            blocks: HashMap::new(),
            reader: reader_end,
            nodes: nodes,
        })
    }

    fn recv_messages(&mut self) -> io::Result<()> {
        while true {
            println!("MAIN: Listening for incoming messages");
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
        Ok(())
    }

    pub fn read_block(&mut self, block: Block) -> io::Result<()> {
        println!("MAIN: Adding block to blocks hashmap");
        self.blocks.insert(block.hash(), block);
        println!(
            "MAIN: Got block message. Current blocks len: {:?}",
            self.blocks.len()
        );
        Ok(())
    }

    fn log_headers(&mut self, headers: Headers) {
        println!("LOG: old best hash:          {:?}", self.tallest_header);
        println!("LOG: first header prev hash: {:?}", headers.block_headers[0].prev_hash());
        println!("LOG: first header:           {:?}", headers.block_headers[0].hash());
        println!("LOG: last header:            {:?}", headers.block_headers[headers.count - 1].hash());
    }

    fn read_headers(&mut self, mut headers: Headers) -> io::Result<()> {
        self.log_headers(headers.clone());
        // request more headers
        self.tallest_header = headers.last_header_hash();
        if headers.is_paginated() {
            self.request_headers(self.tallest_header)?;
        }

        // store headers in hashmap
        self.headers.extend(into_hashmap(headers.block_headers.clone()));
        println!(
            "MAIN: Got headers message. New headers len: {:?}. Incoming headers len: {:?}",
            self.headers.len(),
            headers.count
        );
        
        // request blocks for headers after given date
        let init_tp_timestamp: u32 = Config::from_file()?.get_start_timestamp();
        headers.trim_timestamp(init_tp_timestamp)?;
        println!("MAIN: headers message count after trim: {:?}", headers.count);
        self.request_blocks(headers)?;
        Ok(())
    }

    fn request_headers(&mut self, header_hash: HashId) -> Result<(), io::Error> {
        println!("MAIN: Requesting headers");
        let sync_node = &mut self.nodes[0];
        let getheader_message = GetHeader::from_last_header(header_hash);
        sync_node.send(getheader_message.serialize()?)?;
        Ok(())
    }

    fn request_blocks(&mut self, headers: Headers) -> io::Result<()> {
        if headers.count == 0 {
            return Ok (())
        }
        let headers_buckets = to_buckets(headers.block_headers, self.nodes.len());
        for (node_number, bucket) in headers_buckets.into_iter().enumerate() {
            let chunks = bucket.chunks(MAX_INV_SIZE);
            let node = &mut self.nodes[node_number];
            for chunk in chunks {
                let get_data = GetData::from_inv(chunk.len(), chunk.to_vec());
                node.send(get_data.serialize()?)?;
            }
        }
        Ok(())
    }

    pub fn initial_block_download(&mut self) -> Result<(), io::Error> {
        if let Ok(headers) = Headers::from_file("tmp/headers_backup.dat") {
            self.headers = into_hashmap(headers.block_headers);
        }
        self.request_headers(self.tallest_header)?;
        self.recv_messages()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_nodes() {
        let nodes_it = find_nodes().unwrap();
        let mut nodes_vec: Vec<SocketAddr> = Vec::new();
        for node in nodes_it {
            println!("Established connection with node: {:?}", node);
            nodes_vec.push(node);
        }
        assert!(!nodes_vec.is_empty());
    }
}
