use crate::config::Config;
use crate::messages::constants;
use crate::messages::{BlockHeader, GetData, GetHeader, Headers, Message, MessageHeader};
use crate::node::Node;
use crate::serialized_blocks::SerializedBlocks;
use crate::utility::to_max_len_buckets;
use std::sync::mpsc;
use std::{
    io,
    net::{SocketAddr, ToSocketAddrs},
};

fn find_nodes() -> Result<std::vec::IntoIter<SocketAddr>, io::Error> {
    let node_discovery_hostname = Config::from_file()?.get_hostname();
    node_discovery_hostname.to_socket_addrs()
}

fn handle_headers_message(node: &mut Node) -> Result<Headers, io::Error> {
    //println!("\nSending self getBlocks (genesis) message...");
    let genesis_message = GetHeader::default();
    node.send(genesis_message.serialize()?)?;

    let mut headers = Headers::from_stream(&mut node.stream)?;
    headers.read_all_headers(node)?;
    Ok(headers)
}

fn handle_getdata_message(node: &mut Node, headers: Vec<BlockHeader>) -> Result<(), io::Error> {
    let get_data = GetData::from_inv(headers.len(), headers);
    // println!("Sending GetData message: {:?}", &get_data);
    node.send(get_data.serialize()?)?;
    let block_message = MessageHeader::read_until_command(&mut node.stream, "block\0\0\0\0\0\0\0")?;
    let data_blocks = block_message.read_payload(&mut node.stream)?;

    // save data_blocks to file
    // let mut save_stream = File::create("src/block_message_payload.dat")?;
    // save_stream.write_all(&data_blocks)?;

    let block_message_data = SerializedBlocks::from_bytes(&data_blocks)?;
    println!("Block message data: {:?}", block_message_data);

    Ok(())
}

pub fn connect_to_network() -> Result<(Vec<Node>, mpsc::Receiver<u8>), io::Error> {
    let node_addresses = find_nodes()?;
    let mut nodes = Vec::new();
    let (writer_end, reader_end) = mpsc::channel();
    for node_addr in node_addresses {
        match Node::try_from_addr(node_addr, writer_end.clone()) {
            Ok(node) => nodes.push(node),
            Err(..) => continue,
        }
    }
    Ok((nodes, reader_end))
}

pub fn find_best_chain(nodes: &mut Vec<Node>) -> Result<Headers, io::Error> {
    let sync_node = &mut nodes[0];
    let mut headers = handle_headers_message(sync_node)?;

    for node in nodes {
        headers.read_all_headers(node)?;
    }

    Ok(headers)
}

pub fn initial_header_download(nodes: &mut Vec<Node>) -> Result<Headers, io::Error> {
    let headers = find_best_chain(nodes)?;
    headers.save_to_file("tmp/headers.dat")?;
    Ok(headers)
}

pub fn sync(nodes: &mut Vec<Node>) -> Result<(), io::Error> {
    let mut headers = match Headers::from_file("tmp/headers_backup.dat") {
        Ok(headers) => headers,
        Err(..) => initial_header_download(nodes)?,
    };

    // keep only headers that are more recent than specified timestamp
    headers.remove_older_than(1681095600); // project start date 2023-04-10

    // send getdata messages with max 50k headers each (this should be changed to use a threadpool with a node per thread)
    let headers_buckets =
        to_max_len_buckets(headers.block_headers, constants::messages::MAX_INV_SIZE);
    for bucket in headers_buckets {
        handle_getdata_message(&mut nodes[0], bucket)?;
    }
    Ok(())
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
