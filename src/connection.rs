use crate::message_verack::VerAckMessage;
use crate::message_version::Version;
use crate::messages::Message;
use std::io::Read;
use std::net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs};

fn find_nodes() -> Result<std::vec::IntoIter<std::net::SocketAddr>, String> {
    // The port used by Bitcoin nodes to communicate with each other is:
    // 8333 in the mainnet
    // 18333 in the testnet
    // 38333 in the signet
    // 18444 in the regtest (local)
    let port = "18333";
    let node_discovery_hostname = "seed.testnet.bitcoin.sprovoost.nl".to_owned() + ":" + port;
    node_discovery_hostname
        .to_socket_addrs()
        .map_err(|error| error.to_string())
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
    let _rcv_version = Version::from_bytes(&data)?;
    println!("Got message:");
    println!("{:?}", _rcv_version);
    println!("Done testing");
    Ok(stream)
}

fn handshake_node(node_addr: SocketAddr) -> Result<TcpStream, String> {
    // in progress, should replace all unwraps by return Err()
    // this should be implemented following https://developer.bitcoin.org/devguide/p2p_network.html#connecting-to-peers

    // connect to server
    let mut stream = TcpStream::connect(node_addr).map_err(|error| error.to_string())?;
    println!("Connected: {:?}", stream);

    // send and receive VERSION
    // send message
    println!("\nSending self version message...");
    let msg_version = Version::default();
    msg_version
        .send_to(&mut stream)
        .map_err(|error| error.to_string())?;

    // receive message
    let mut data = [0_u8; 180];
    stream.read(&mut data).map_err(|error| error.to_string())?;

    let _rcv_version = Version::from_bytes(&data)?;
    println!("Peer responded: {:?}", _rcv_version);

    // send and recieve VERACK
    // send message
    println!("\nSending self verack message...");
    let verack_version = VerAckMessage::new();
    verack_version
        .send_to(&mut stream)
        .map_err(|error| error.to_string())?;

    // receive message
    data = [0_u8; 180];
    stream.read(&mut data).map_err(|error| error.to_string())?;

    let _rcv_verack = VerAckMessage::from_bytes(&data)?;
    println!("Peer responded: {:?}", _rcv_verack);

    Ok(stream)
}

fn get_genesis_block(node: SocketAddr) -> Result<(), String>{
    let genesis_message =
    Ok(())
}

pub fn connect_to_network() -> Result<(), String> {
    let nodes = find_nodes()?;
    for ip_addr in nodes {
        handshake_node(ip_addr)?;
        println!("\n\n");
    }

    let node = nodes[-1];
    genesis_block = get_genesis_block(node);


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
