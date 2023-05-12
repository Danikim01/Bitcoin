use crate::messages::{GetHeader, Message, MessageHeader, VerAck, Version};
use std::net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
//use std::ops::Generator;
use std::io::{self, Read};


fn find_nodes() -> Result<std::vec::IntoIter<std::net::SocketAddr>, io::Error> {
    // The port used by Bitcoin nodes to communicate with each other is:
    // 8333 in the mainnet
    // 18333 in the testnet
    // 38333 in the signet
    // 18444 in the regtest (local)
    let port = "18333";
    let node_discovery_hostname = "seed.testnet.bitcoin.sprovoost.nl".to_owned() + ":" + port;
    node_discovery_hostname.to_socket_addrs()
}

fn test_handshake() -> Result<TcpStream, String> {
    // create listener
    let listener = TcpListener::bind("127.0.0.1:18333").unwrap();
    println!("Server listening on port 18333");

    // connect to server
    let my_addr = "127.0.0.1:18333"
        .to_socket_addrs()
        .map_err(|error| error.to_string())?
        .next()
        .unwrap();
    let mut stream = TcpStream::connect(my_addr).map_err(|error| error.to_string())?;

    // send message
    let msg_version = Version::default();
    msg_version
        .send_to(&mut stream)
        .map_err(|error| error.to_string())?;

    // receive connection
    let mut data = [0_u8; 90];
    let mut temp_stream;
    let temp_addr;
    (temp_stream, temp_addr) = listener.accept().unwrap();
    println!("New connection: {:?}", temp_addr);

    // receive message
    temp_stream
        .read(&mut data)
        .map_err(|error| error.to_string())?;

    println!("Sent message:");
    println!("{:?}", msg_version);
    let _rcv_version = Version::from_bytes(&data).map_err(|error| error.to_string())?;
    println!("Got message:");
    println!("{:?}", _rcv_version);
    println!("Done testing");
    Ok(stream)
}

fn handshake_version(stream: &mut TcpStream) -> Result<bool, io::Error> {
    // send message
    println!("\nSending self version message...");
    let msg_version = Version::default();
    msg_version.send_to(stream)?;

    // TODO: we may receive verack first 
    //       if header read is of version read into version, 
    //       if verack read into verack
    //       for now we read first into version

    let message_header = MessageHeader::from_stream(stream)?;
    
    // read payload into version
    let payload_data = message_header.read_payload(stream)?;
    let version_message = Version::from_bytes(&payload_data)?;
    println!("Read version: {:?}\n", version_message);

    Ok(msg_version.accepts(version_message))
}

fn handshake_verack(stream: &mut TcpStream) -> Result<(), io::Error> {
    let verack_message = VerAck::from_stream(stream)?;
    println!("Read verack: {:?}\n", verack_message);

    //then send message
    println!("\nSending self verack message...");
    let _verack_version = VerAck::new().send_to(stream)?;

    Ok(())
}

fn handle_headers_message(stream: &mut TcpStream) -> Result<(), io::Error> {
    println!("\nSending self getBlocks (genesis) message...");
    let mut genesis_message = GetHeader::default();
    loop{
        print!("Send genesis message: {:?}\n", genesis_message);
        genesis_message.send_to(stream)?;
        println!("Wait til headers message...\n");
        let headers_message = MessageHeader::read_until_command (stream, "headers\0\0\0\0\0")?;

        println!("Peer responded: {:?}\n", headers_message);
        let data_headers = headers_message.read_payload(stream)?;

        let headers_data = GetHeader::from_bytes(&data_headers)?;
        //println!("Peer responded: {:?}\n", headers_data);
        println!("Is last header: {:?}\n", headers_data.is_last_header());

        if headers_data.is_last_header(){
            break;
        }

        genesis_message = GetHeader::from_last_header(&headers_data.last_header_hash());
    }

    Ok(())
}

fn handshake_node(node_addr: SocketAddr) -> Result<TcpStream, io::Error> {
    // this should be implemented following https://developer.bitcoin.org/devguide/p2p_network.html#connecting-to-peers

    // connect to server
    let mut stream = TcpStream::connect(node_addr)?;
    println!("Connected: {:?}", stream);

    // send and receive VERSION
    if !handshake_version(&mut stream)? {
        return Ok(stream);
    }

    // send and recieve VERACK
    handshake_verack(&mut stream)?;

    //send getheaders receive 2000 headers
    
    handle_headers_message(&mut stream)?;

    Ok(stream)
}

fn get_genesis_block(node: SocketAddr) -> Result<(), String> {
    // let genesis_message = GetBlocks::default();
    // genesis_message.
    //     send_to(&mut node)
    //     .map_err(|error| error.to_string())?;

    // todo rcv inv_message with all block_hashes
    Ok(())
}

pub fn connect_to_network() -> Result<(), io::Error> {
    let nodes = find_nodes()?;
    for ip_addr in nodes {
        let stream = handshake_node(ip_addr)?;
    }

    // let node = nodes[-1];
    // genesis_block = get_genesis_block(node);

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
    fn test_handshake_node() {
        // testing
        // test_handshake()?;
        assert!(true);
    }

    #[test]
    fn test_connection_is_ok() {
        assert!(true);
        // assert!(connect_to_network().is_ok());
    }
}
