use crate::block_header::BlockHeader;
use crate::config::Config;
use crate::messages::constants;
use crate::messages::{
    GetData, GetHeader, Headers, InvType, Inventory, Message, MessageHeader, VerAck, Version,
};
use crate::serialized_blocks::SerializedBlock;
use crate::utility::to_max_len_buckets;
use std::{
    io,
    io::ErrorKind,
    net::{SocketAddr, TcpStream, ToSocketAddrs},
};

fn find_nodes() -> Result<std::vec::IntoIter<SocketAddr>, io::Error> {
    let node_discovery_hostname = Config::from_file()?.get_hostname();
    node_discovery_hostname.to_socket_addrs()
}

fn handshake_version(stream: &mut TcpStream) -> Result<(), io::Error> {
    // send message
    let msg_version = Version::default();
    msg_version.send_to(stream)?;

    // El version SIEMPRE se envía antes que el verack según el estándar
    // y TCP asegura que los paquetes llegan en el orden en el que se enviaron
    let message_header = MessageHeader::from_stream(stream)?;
    let payload_data = message_header.read_payload(stream)?;
    let version_message = Version::from_bytes(&payload_data)?;

    if !msg_version.accepts(version_message) {
        return Err(io::Error::new(
            ErrorKind::Unsupported,
            "Version not supported",
        ));
    }
    Ok(())
}

fn handshake_verack(stream: &mut TcpStream) -> Result<(), io::Error> {
    VerAck::from_stream(stream)?;
    VerAck::new().send_to(stream)?;
    Ok(())
}

fn handle_headers_message(stream: &mut TcpStream) -> Result<Headers, io::Error> {
    //println!("\nSending self getBlocks (genesis) message...");
    let genesis_message = GetHeader::default();
    genesis_message.send_to(stream)?;

    let mut headers = Headers::from_stream(stream)?;
    headers.read_all_headers(stream)?;

    Ok(headers)
}

fn build_getdata(count: usize, block_headers: Vec<BlockHeader>) -> GetData {
    let mut inventory_vector: Vec<Inventory> = Vec::new();

    for block_header in block_headers {
        //println!("the header hash is {:?}",&header_hash);
        //println!("the header hash is {:?}",hash_to_bytes(&header_hash));
        inventory_vector.push(Inventory::new(
            InvType::MSGBlock,
            block_header.hash_block_header(),
        ));
    }

    GetData::new(count, inventory_vector)
}

fn handle_getdata_message(stream: &mut TcpStream, headers: Vec<BlockHeader>) -> Result<(), io::Error> {
    let get_data = build_getdata(headers.len(), headers);
    get_data.send_to(stream)?;
    let block_message = MessageHeader::read_until_command(stream, "block\0\0\0\0\0\0\0")?;
    let data_blocks = block_message.read_payload(stream)?;

    let block_message_data = SerializedBlock::from_bytes(&data_blocks)?;

    Ok(())
}

fn handshake_node(node_addr: SocketAddr) -> Result<TcpStream, io::Error> {
    // this should be implemented following https://developer.bitcoin.org/devguide/p2p_network.html#connecting-to-peers

    // connect to server
    let mut stream = TcpStream::connect(node_addr)?;
    println!("Connected: {:?}", stream);

    // send and receive VERSION
    // unsuported versions return error
    handshake_version(&mut stream)?;

    // send and recieve VERACK
    handshake_verack(&mut stream)?;
    Ok(stream)
}

pub fn connect_to_network() -> Result<Vec<TcpStream>, io::Error> {
    let ip_nodes = find_nodes()?;
    let mut nodes = Vec::new();
    for ip_addr in ip_nodes {
        if ip_addr.is_ipv4() {
            let stream = match handshake_node(ip_addr) {
                Ok(stream) => stream,
                Err(ref e) if e.kind() == ErrorKind::Unsupported => continue,
                Err(..) => continue,
            };
            nodes.push(stream);
        }
    }
    Ok(nodes)
}

pub fn find_best_chain(nodes: &mut Vec<TcpStream>) -> Result<Headers, io::Error> {
    let sync_node = &mut nodes[0];
    let mut headers = handle_headers_message(sync_node)?;

    for mut node in nodes {
        headers.read_all_headers(&mut node)?;
    }

    Ok(headers)
}

pub fn initial_sync(nodes: &mut Vec<TcpStream>) -> Result<(), io::Error> {
    let headers = find_best_chain(nodes)?;
    headers.save_to_file("tmp/headers.dat")?;

    println!(
        "Block headers read from file: {:?}",
        headers.block_headers.len()
    );
    handle_getdata_message(&mut nodes[0], headers.block_headers)?;

    ///todo this should be a parallel execution
    Ok(())
}

pub fn sync(nodes: &mut Vec<TcpStream>) -> Result<&mut Vec<TcpStream>, io::Error> {
    let mut headers = Headers::from_file("tmp/headers_backup.dat")?;

    // keep only headers than are more recent than specified timestamp
    let init_tp_timestamp: u32 = Config::from_file()?.get_start_timestamp();
    headers.trim_timestamp(init_tp_timestamp)?;

    let mut discrepancy_count = 0;
    for i in 1..headers.count{
        let prev_block_hash = headers.block_headers[i].prev_block_hash;
        let hash_block_header = headers.block_headers[i - 1].hash_block_header();

        if prev_block_hash != hash_block_header {
            discrepancy_count += 1;
        }
    }

    if discrepancy_count == 0 {
        println!("Todos los bloques cumplen con la igualdad.");
    } else {
        println!("Se encontraron {} discrepancias en la igualdad entre bloques.", discrepancy_count);
    }
    
    // send getdata messages with max 50k headers each (this should be changed to use a threadpool with a node per thread)
    let headers_buckets = to_max_len_buckets(headers.block_headers, constants::messages::MAX_INV_SIZE);
    for bucket in headers_buckets {
        handle_getdata_message(&mut nodes[0], bucket)?;
    }
    
    Ok(nodes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_nodes() {
        let nodes_it = find_nodes().unwrap();
        let mut nodes_vec: Vec<SocketAddr> = Vec::new();
        for node in nodes_it {
            println!("node: {}", node);
            nodes_vec.push(node);
        }
        assert!(!nodes_vec.is_empty());
    }

    #[test]
    fn test_handshake_version() -> Result<(), io::Error> {
        let listener = std::net::TcpListener::bind("127.0.0.1:18333").unwrap();
        let addr = listener.local_addr().unwrap();

        let mut stream_peer = TcpStream::connect(addr).unwrap();
        let response = Version::default();
        response.send_to(&mut stream_peer).unwrap();

        let (mut rcvr_stream, _addr) = listener.accept().unwrap();
        handshake_version(&mut rcvr_stream).unwrap();
        Ok(())
    }

    // #[test]
    // fn test_handshake() {
    //     // create listener
    //     let listener = TcpListener::bind("127.0.0.1:18333").unwrap();
    //     println!("Server listening on port 18333");

    //     // connect to server
    //     let my_addr = "127.0.0.1:18333"
    //         .to_socket_addrs()
    //         .map_err(|error| error.to_string())
    //         .unwrap()
    //         .next()
    //         .unwrap();
    //     let mut stream = TcpStream::connect(my_addr)
    //         .map_err(|error| error.to_string())
    //         .unwrap();

    //     // send message
    //     let msg_version = Version::default();
    //     msg_version
    //         .send_to(&mut stream)
    //         .map_err(|error| error.to_string())
    //         .unwrap();

    //     // receive connection
    //     let mut data = [0_u8; 90];
    //     let mut temp_stream;
    //     let temp_addr;
    //     (temp_stream, temp_addr) = listener.accept().unwrap();
    //     println!("New connection: {:?}", temp_addr);

    //     // receive message
    //     temp_stream
    //         .read(&mut data)
    //         .map_err(|error| error.to_string())
    //         .unwrap();

    //     println!("Sent message:");
    //     println!("{:?}", msg_version);
    //     let _rcv_version = Version::from_bytes(&data)
    //         .map_err(|error| error.to_string())
    //         .unwrap();
    //     println!("Got message:");
    //     println!("{:?}", _rcv_version);
    //     println!("Done testing");
    // }

    // #[test]
    // fn test_handshake_node() {
    //     // testing
    //     // test_handshake()?;
    //     assert!(true);
    // }

    // #[test]
    // fn test_connection_is_ok() {
    //     assert!(true);
    //     // assert!(connect_to_network().is_ok());
    // }
}
