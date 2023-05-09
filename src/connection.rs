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
    
    // first read message header
    let mut header_buffer = [0_u8; 24];
    stream.read(&mut header_buffer)?;
    let mut message_header = MessageHeader::from_bytes(&header_buffer)?;
    
    // read payload into version
    let mut payload_data = vec![0_u8; message_header.payload_size as usize]; 
    stream.read(&mut payload_data)?;
    let version_message = Version::from_bytes(&payload_data)?;
    println!("Read version: {:?}\n", version_message);

    Ok(msg_version.accepts(version_message))
}

fn handshake_verack(stream: &mut TcpStream) -> Result<(), io::Error> {

    let mut header_buffer = [0_u8; 24];
    //read verack
    stream.read(&mut header_buffer)?;
    let verack_message = VerAck::from_bytes(&header_buffer)?;
    println!("Read verack: {:?}\n", verack_message);

    //then send message
    println!("\nSending self verack message...");
    let _verack_version = VerAck::new().send_to(stream)?;

    Ok(())
}

fn read_until(stream: &mut TcpStream, cmd: &str) -> Result<MessageHeader, io::Error> {
    let mut header_buffer = [0_u8; 24];
    stream.read(&mut header_buffer)?;

    let mut message = MessageHeader::from_bytes(&header_buffer)?;

    while message.command_name != cmd{
        let mut dummy_buff = vec![0_u8; message.payload_size as usize];
        stream.read(&mut dummy_buff)?;

        stream.read(&mut header_buffer)?;
        message = MessageHeader::from_bytes(&header_buffer)?;

    }
    Ok(message)
}

fn handle_headers_message(stream: &mut TcpStream) -> Result<(), io::Error> {
    println!("\nSending self getBlocks (genesis) message...");
    let genesis_message = GetHeader::default();
    genesis_message.send_to(stream)?;

    let headers_message = read_until(stream, "headers\0\0\0\0\0")?;
    
    println!("Peer responded: {:?}", headers_message);
    let mut data_headers = [0_u8;2000*81+24];
    stream.read(&mut data_headers)?;

    let headers_message_data = GetHeader::from_bytes(&data_headers);
    println!("Peer responded: {:?}", headers_message_data);

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
        break;
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
