use crate::config::Config;
use crate::logger::log;
use crate::messages::constants::config::QUIET;
use crate::messages::Message;
use crate::node::Node;
use rand::random;
use std::collections::HashMap;
use std::io;
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

    pub fn kill_node(&mut self, socket_addr: SocketAddr) -> io::Result<()> {
        self.nodes.remove(&socket_addr);
        if self.nodes.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::NotConnected,
                "All connections were closed",
            ));
        }
        Ok(())
    }

    pub fn send_to_any(&mut self, payload: &Vec<u8>) -> io::Result<()> {
        let random_number: usize = random();
        let node_number = random_number % self.nodes.len();
        let random_node = self.nodes.values_mut().nth(node_number).unwrap();
        let node_address = random_node.get_addr()?;
        match &mut random_node.send(payload) {
            Ok(_) => Ok(()),
            Err(e) => {
                log(
                    &format!(
                        "Error writing to ANY TCPStream: {:?}, Killing connection and retrying.",
                        e
                    ) as &str,
                    QUIET,
                );
                self.kill_node(node_address)?;
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
        let node_address = node.get_addr()?;
        match &mut node.send(payload) {
            Ok(_) => Ok(()),
            Err(e) => {
                log(
                    &format!("Error writing to TCPStream: {:?}, Killing connection.", e) as &str,
                    QUIET,
                );
                self.kill_node(node_address)?;
                Err(io::Error::new(
                    io::ErrorKind::NotConnected,
                    "Failed to send message to peer",
                ))
            }
        }
    }

    pub fn send_to_all(&mut self, payload: &[u8]) -> io::Result<()> {
        let mut dead_connections = vec![];
        for node in self.nodes.values_mut() {
            let node_address = node.get_addr()?;
            if let Err(e) = node.send(payload) {
                log(
                    &format!("Error writing to TCPStream: {:?}, Killing connection.", e) as &str,
                    QUIET,
                );
                dead_connections.push(node_address);
                continue;
            }
        }
        for node_address in dead_connections {
            self.kill_node(node_address)?;
        }
        Ok(())
    }
}
