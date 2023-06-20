use crate::config::Config;
use crate::logger::log;
use crate::messages::constants::config::QUIET;
use crate::messages::Message;
use crate::node::Node;
use rand::random;
use std::collections::HashMap;
use std::io;
use std::net::Shutdown;
use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::mpsc;
// gtk imports
use crate::interface::GtkMessage;
use gtk::glib::Sender;

pub struct NodeController {
    nodes: HashMap<SocketAddr, Node>,
}

fn find_nodes() -> Result<std::vec::IntoIter<SocketAddr>, io::Error> {
    let node_discovery_hostname = Config::from_file()?.get_hostname();
    node_discovery_hostname.to_socket_addrs()
}

impl NodeController {
    pub fn connect_to_peers(
        writer_end: mpsc::Sender<(SocketAddr, Message)>,
        sender: Sender<GtkMessage>,
    ) -> Result<Self, io::Error> {
        let node_addresses = find_nodes()?;
        let mut nodes = HashMap::new();
        for node_addr in node_addresses {
            match Node::try_from_addr(node_addr, writer_end.clone(), sender.clone()) {
                Ok((peer_addr, node)) => {
                    nodes.insert(peer_addr, node);
                    // break; // uncomment this to use a single node as peer
                }
                Err(..) => continue,
            }
        }
        Ok(Self { nodes })
    }

    pub fn send_to_any(&mut self, payload: &Vec<u8>) -> io::Result<()> {
        let random_number: usize = random();
        let node_number = random_number % self.nodes.len();
        let random_node = self.nodes.values_mut().nth(node_number).unwrap();
        match &mut random_node.send(payload) {
            Ok(_) => Ok(()),
            Err(e) => {
                log(
                    &format!("Error writing to TCPStream: {:?}. Trying a dif node", e) as &str,
                    QUIET,
                );
                // self.nodes.swap_remove(node_number); // IMPLEMENT ANOTHER WAY TO KILL THE NODE
                if self.nodes.is_empty() {
                    return Err(io::Error::new(
                        io::ErrorKind::NotConnected,
                        "All connections were closed",
                    ));
                }
                self.send_to_any(payload)
            }
        }
    }

    pub fn send_to_specific(&mut self, peer: &SocketAddr, payload: &[u8]) -> io::Result<()> {
        let node = match self.nodes.get_mut(peer) {
            Some(n) => n,
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::NotConnected,
                    "Peer not found",
                ))
            }
        };
        match &mut node.send(payload) {
            Ok(_) => Ok(()),
            Err(e) => {
                log(
                    &format!("Error writing to TCPStream: {:?}. Trying a dif node", e) as &str,
                    QUIET,
                );
                Err(io::Error::new(
                    io::ErrorKind::NotConnected,
                    "Failed to send message to peer",
                ))
            }
        }
    }

    pub fn send_to_all(&mut self, payload: &[u8]) -> io::Result<()> {
        for node in self.nodes.values_mut() {
            match &mut node.send(payload) {
                Ok(_) => continue,
                Err(e) => {
                    log(
                        &format!("Error writing to TCPStream: {:?}. Trying a dif node", e) as &str,
                        QUIET,
                    );
                    continue;
                }
            }
        }
        Ok(())
    }
}
