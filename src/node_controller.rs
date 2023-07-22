use crate::config::Config;
use crate::messages::constants::config::QUIET;
use crate::messages::Message;
use crate::node::Node;
use std::collections::HashMap;
use std::io;
use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::mpsc;
// gtk imports
use crate::interface::GtkMessage;
use gtk::glib::SyncSender;

/// The NodeController struct is responsible for managing all the nodes and sending messages to them.
pub struct NodeController {
    nodes: HashMap<SocketAddr, Node>,
}

fn find_nodes(config: &Config) -> Result<std::vec::IntoIter<SocketAddr>, io::Error> {
    let node_discovery_hostname = config.get_hostname();
    node_discovery_hostname.to_socket_addrs()
}

impl NodeController {
    /// Creates a new NodeController and connects to the peers.
    pub fn connect_to_peers(
        writer_end: mpsc::SyncSender<(SocketAddr, Message)>,
        sender: SyncSender<GtkMessage>,
        config: Config,
    ) -> Result<Self, io::Error> {
        let node_addresses = find_nodes(&config)?;
        let mut nodes = HashMap::new();
        for node_addr in node_addresses {
            match Node::try_from_addr(
                node_addr,
                writer_end.clone(),
                sender.clone(),
                config.clone(),
            ) {
                Ok((peer_addr, node)) => {
                    nodes.insert(peer_addr, node);
                    // break; // uncomment this to use a single node as peer
                }
                Err(..) => continue,
            }
        }
        Ok(Self { nodes })
    }

    pub fn add_node(&mut self, node: Node) {
        self.nodes.insert(node.address, node);
    }

    /// Kills a node and removes it from the list of nodes given its peer address.
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

    /// Sends a message to a specific node given its peer address.
    pub fn send_to_specific(
        &mut self,
        peer: &SocketAddr,
        payload: &[u8],
        config: &Config,
    ) -> io::Result<()> {
        let node = match self.nodes.get_mut(peer) {
            Some(n) => n,
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::NotConnected,
                    "Peer not found",
                ))
            }
        };
        let node_address = node.address;
        match &mut node.send(payload) {
            Ok(_) => Ok(()),
            Err(e) => {
                config.log(
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

    /// Broadcasts a message to all the nodes.
    pub fn send_to_all(&mut self, payload: &[u8], config: &Config) -> io::Result<()> {
        let mut alive_nodes: Vec<SocketAddr> = vec![];
        for node in self.nodes.values_mut() {
            match node.send(payload) {
                Ok(_) => {
                    alive_nodes.push(node.address);
                }
                Err(e) => config.log(
                    &format!("Error writing to TCPStream: {:?}, Killing connection.", e) as &str,
                    QUIET,
                ),
            }
        }
        self.nodes.retain(|k, _v| alive_nodes.contains(k));
        Ok(())
    }
}
