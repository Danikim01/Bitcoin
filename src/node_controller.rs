use std::io;
use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::mpsc;
use crate::config::Config;
use crate::messages::Message;
use crate::node::Node;
use rand::random;

pub struct NodeController {
    nodes: Vec<Node>
}

fn find_nodes() -> Result<std::vec::IntoIter<SocketAddr>, io::Error> {
    let node_discovery_hostname = Config::from_file()?.get_hostname();
    node_discovery_hostname.to_socket_addrs()
}

impl NodeController {
    pub fn connect_to_peers(writer_end: mpsc::Sender<Message>) -> Result<Self, io::Error> {
        let node_addresses = find_nodes()?;
        let mut nodes = Vec::new();
        for node_addr in node_addresses {
            match Node::try_from_addr(node_addr, writer_end.clone()) {
                Ok(node) => nodes.push(node),
                Err(..) => continue,
            }
        }
        Ok(Self {
            nodes: nodes
        })
    }

    pub fn send_to_any(&mut self, payload: &Vec<u8>) -> io::Result<()> {
        let random_number: usize = random();
        let node_number = random_number % self.nodes.len();
        match self.nodes[node_number].send(payload) {
            Ok(k) => Ok(k),
            Err(e) => {
                println!("Error writing to TCPStream: {:?}. Trying a dif node", e);
                self.nodes.swap_remove(node_number);
                if self.nodes.len() == 0 {
                    return Err(io::Error::new(io::ErrorKind::NotConnected, "All connections were closed"));
                }
                self.send_to_any(payload)
            }
        }
    }

}
