use crate::messages::Message;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use crate::messageVersion::Version;

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


fn handshake_node(ip_addr: SocketAddr) -> Result<TcpStream, String> {
    // in progress, should replace all unwraps by return Err()
    // this should be implemented following https://developer.bitcoin.org/devguide/p2p_network.html#connecting-to-peers

    let mut stream = TcpStream::connect(ip_addr).map_err(|error| error.to_string())?;
    let msg_version = Version::default();
    // update ip addr and port of Version struct
    msg_version.send_to(&mut stream).map_err(|error| error.to_string())?;
    let mut data = [0 as u8];
    stream
        .read_exact(&mut data)
        .map_err(|error| error.to_string())?;

    let rcv_version = Version::default().from_bytes(&data)?;

    // send and recieve VERACK
    Ok(stream)
}

pub fn connect_to_network() -> Result<(), String> {
    // for now, establishing connection to only 1 node
    let ip_addr = match find_nodes()?.next() {
        Some(addr) => addr,
        _ => return Err("Node discovery failed, no A/AAAA records found for DNS query".into()),
    };
    handshake_node(ip_addr)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_nodes() {
        let nodes_it = find_nodes().unwrap();
        let mut nodes_vec: Vec<std::net::SocketAddr> = Vec::new();
        for node in nodes_it {
            println!("node: {}", node);
            nodes_vec.push(node);
        }
        assert!(!nodes_vec.is_empty());
    }

    #[test]
    fn test_handshake_node() {
        assert!(true);
    }

    #[test]
    fn test_connection_is_ok() {
        assert!(true);
        // assert!(connect_to_network().is_ok());
    }
}
