use crate::block_header::BlockHeader;
use crate::config::Config;
use crate::messages::{
    constants::commands::HEADER, GetData, GetHeader, Headers, InvType, Inventory, Message,
    MessageHeader, VerAck, Version,
};
use std::io;
use std::io::Read;
use std::{
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

    let headers_message = MessageHeader::read_until_command(stream, HEADER)?;

    println!("Peer responded with headers message of payload size: {:?}", headers_message.payload_size);
    let data_headers = headers_message.read_payload(stream)?;
    Headers::from_bytes(&data_headers)
}

fn build_getdata(count: &usize, block_hashes: &Vec<BlockHeader>) -> GetData {
    //println!("{} - {:?}",count,block_hashes[0]);
    let mut inventory_vector: Vec<Inventory> = Vec::new();
    for hash in block_hashes {
        inventory_vector.push(Inventory::new(InvType::MSGBlock, hash.prev_block_hash));
    }
    //println!("{:?}",&inventory_vector);
    GetData::new(*count, inventory_vector)
}

fn handle_getdata_message(stream: &mut TcpStream, header: &Headers) -> Result<(), io::Error> {
    println!("esperando a mensaje inv");
    let inv_message = MessageHeader::read_until_command(stream, "inv\0\0\0\0\0\0\0\0\0")?;
    println!("Peer responded: {:?}", inv_message);

    println!("Building getdata message");
    let get_data = build_getdata(&header.count, &header.block_headers);
    println!("Sending GetData message: {:?}", &get_data);
    get_data.send_to(stream)?;

    let headers_message = MessageHeader::read_until_command(stream, "block\0\0\0\0\0\0\0")?;

    println!("Peer responded: {:?}", headers_message);

    let mut data_blocks = [0_u8; 2000 * 81 + 24];
    stream.read(&mut data_blocks)?;

    let block_message_data = GetData::from_bytes(&data_blocks);

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

pub fn connect_to_network() -> Result<(), io::Error> {
    let nodes = find_nodes()?;
    for ip_addr in nodes {
        let mut stream = match handshake_node(ip_addr) {
            Ok(stream) => stream,
            Err(ref e) if e.kind() == io::ErrorKind::Unsupported => continue,
            Err(e) => return Err(e),
        };

        //send getheaders receive 2000 headers
        let headers = handle_headers_message(&mut stream)?;
        handle_getdata_message(&mut stream, &headers)?;
        break; // for now, sync against only one node
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
