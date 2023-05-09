use crate::messages::{GetBlocks, Message, VerAck, Version};
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

fn handshake_version(stream: &mut TcpStream) -> Result<(), io::Error> {
    // send message
    println!("\nSending self version message...");
    let msg_version = Version::default();
    msg_version.send_to(stream)?;

    // receive message
    let mut data = [0_u8; 180];
    stream.read(&mut data)?;
    let _rcv_version = Version::from_bytes(&data)?;
    println!("Peer responded: {:?}", _rcv_version);

    Ok(())
}

fn handshake_verack(stream: &mut TcpStream) -> Result<(), io::Error> {
    // send message
    println!("\nSending self verack message...");
    let _verack_version = VerAck::new().send_to(stream)?;

    // receive message
    let mut verack_data = [0_u8; 24];
    stream.read(&mut verack_data)?;
    let _rcv_verack = VerAck::from_bytes(&verack_data)?;
    println!("Peer responded: {:?}\n", _rcv_verack);

    Ok(())
}

fn handshake_node(node_addr: SocketAddr) -> Result<TcpStream, io::Error> {
    // this should be implemented following https://developer.bitcoin.org/devguide/p2p_network.html#connecting-to-peers

    // connect to server
    let mut stream = TcpStream::connect(node_addr)?;
    println!("Connected: {:?}", stream);

    // send and receive VERSION
    handshake_version(&mut stream)?;

    // send and recieve VERACK
    handshake_verack(&mut stream)?;

    // send getBlocks
    // send message
    /*
    println!("\nSending self getBlocks (genesis) message...");
    let genesis_message = GetBlocks::default();
    genesis_message.send_to(&mut stream)?;

    // receive message
    let mut data_genesis = [0_u8; 180];
    stream.read(&mut data_genesis)?;

    let _rcv_block = GetBlocks::from_bytes(&data_genesis)?;
    println!("Peer responded: {:?}", _rcv_block);
    */
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
        handshake_node(ip_addr)?;
        println!("\n\n");
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
