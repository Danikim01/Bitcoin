use std::net::{TcpStream, ToSocketAddrs, SocketAddr};
use std::io::{Read, Write};

fn find_nodes() -> Result<std::vec::IntoIter<std::net::SocketAddr>, String> {
    // The port used by Bitcoin nodes to communicate with each other is:
    // 8333 in the mainnet
    // 18333 in the testnet
    // 38333 in the signet
    // 18444 in the regtest (local)
    let port = "18333";
    let node_discovery_hostname = "seed.testnet.bitcoin.sprovoost.nl".to_owned() + ":" + port;
    node_discovery_hostname.to_socket_addrs().map_err(|error| {error.to_string()})
}

fn handshake_node(ip_addr: SocketAddr) -> Result<TcpStream, String> {
    // in progress, should replace all unwraps by return Err()
    // this should be implemented following https://developer.bitcoin.org/devguide/p2p_network.html#connecting-to-peers
    
    let mut stream = TcpStream::connect(ip_addr).unwrap();
    let msg = b"this should be a version package containing version number and more";
    stream.write(msg).unwrap();
    let mut data = [0 as u8; 82]; // using 82 byte buffer
    stream.read_exact(&mut data).unwrap();

    // check if read data contains supported version, and has expected format
    // send and recieve VERACK
    Ok(stream)
}

pub fn connect_to_network() -> Result<(), String> {
    // for now, establishing connection to only 1 node
    let ip_addr = match find_nodes()?.next() {
        Some(addr) => addr,
        _ => return Err("Node discovery failed, no A/AAAA records found for DNS query".into())
    };
    handshake_node(ip_addr);
    Ok(())
}
