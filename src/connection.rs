use crate::messages::{Headers, GetData, GetHeader, Message, Serialize, BlockHeader, HashId};
use crate::messages::constants::{messages::{GENESIS_HASHID, MAX_INV_SIZE}, header_constants::MAX_HEADER};
use crate::utility::{to_io_err, to_n_chunks, to_max_len_buckets};
use crate::config::Config;
use crate::node::Node;
use std::collections::HashMap;
use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::mpsc;
use std::io;

pub struct NetworkController {
    headers: HashMap<HashId, BlockHeader>,
    tallest_header: HashId,
    reader: mpsc::Receiver<Message>,
    nodes: Vec<Node>
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
        }
        Ok(Self {
            headers: HashMap::new(), tallest_header: GENESIS_HASHID, reader: reader_end, nodes: nodes
        })
    }

    pub fn recv_messages(mut self) -> io::Result<()> {
        while true {
            match self.reader.try_recv().map_err(to_io_err)? {
                Message::Headers(headers) => self.read_headers(headers),
                _ => return Err(io::Error::new(io::ErrorKind::Other, "Received unsupported message")) 
            };
        }
        Ok(())
    }

    pub fn read_headers(mut self, mut headers: Headers) -> io::Result<()> {
        let mut sync_node = self.nodes[0];
        self.headers.extend(headers.into_hashmap());
        if headers.count == MAX_HEADER {
            let getheader_message = GetHeader::from_last_header(headers.last_header_hash());
            sync_node.send(getheader_message.serialize()?)?;
        }
        Ok(())
    }

    pub fn request_blocks(mut self) -> io::Result<()> {
        // this hould request only missing blocks, instead of all of them
        let headers_chunks = to_n_chunks(self.headers.values().copied(), self.nodes.len());
        for (node_number, chunk) in headers_chunks.into_iter().enumerate() {
            let buckets = to_max_len_buckets(chunk, MAX_INV_SIZE);
            let mut node = self.nodes[node_number];
            for bucket in buckets {
                let get_data = GetData::from_inv(bucket.len(), bucket);
                println!("Sending GetData message: {:?}", &get_data);
                node.send(get_data.serialize()?)?;
                // below code should be called from recv_messages() ⬇︎⬇︎
                
                //let block_message = MessageHeader::read_until_command(&mut node.stream, "block\0\0\0\0\0\0\0")?;
                //let data_blocks = block_message.read_payload(&mut node.stream)?;
            
                // save data_blocks to file
                // let mut save_stream = File::create("src/block_message_payload.dat")?;
                // save_stream.write_all(&data_blocks)?;
                //let _block_message_data = SerializedBlock::from_bytes(&data_blocks)?;
            }
        }
        Ok(())
    }
    
    fn request_headers(mut self, header_hash: HashId) -> Result<(), io::Error> {
        let mut sync_node = self.nodes[0];
        let getheader_message = GetHeader::from_last_header(header_hash);
        sync_node.send(getheader_message.serialize()?)?;
        Ok(())
    }
  
    pub fn initial_block_download(mut self) -> Result<(), io::Error> {
        if let Ok(headers) = Headers::from_file("tmp/headers_backup.dat") {
            self.headers = headers.into_hashmap();
        }
        self.request_headers(self.tallest_header);

        // move this logic to read_headers, it should call an async request_blocks() ⬇︎⬇︎
        let init_tp_timestamp: u32 = Config::from_file()?.get_start_timestamp();
        self.headers.trim_timestamp(init_tp_timestamp)?;
        
        self.request_blocks()?;
        println!("All blocks requested");

        // move this logic to read_headers, it should call an async request_blocks() ⬆︎⬆︎
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
