use crate::config::Config;
use crate::logger::log;
use crate::messages::constants::config::QUIET;
use crate::messages::Message;
use crate::node::Node;
use rand::random;
use std::io;
use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::mpsc;

// gtk imports
use crate::interface::GtkMessage;
use gtk::glib::Sender;

pub struct NodeController {
    nodes: Vec<Node>,
}

fn find_nodes() -> Result<std::vec::IntoIter<SocketAddr>, io::Error> {
    let node_discovery_hostname = Config::from_file()?.get_hostname();
    node_discovery_hostname.to_socket_addrs()
}

impl NodeController {
    pub fn connect_to_peers(
        writer_end: mpsc::Sender<Message>,
        sender: Sender<GtkMessage>,
    ) -> Result<Self, io::Error> {
        let node_addresses = find_nodes()?;
        let mut nodes = Vec::new();
        for node_addr in node_addresses {
            match Node::try_from_addr(node_addr, writer_end.clone(), sender.clone()) {
                Ok(node) => {
                    nodes.push(node);
                    break; // uncomment this to use a single node as peer
                }
                Err(..) => continue,
            }
        }
        Ok(Self { nodes })
    }

    pub fn send_to_any(&mut self, payload: &Vec<u8>) -> io::Result<()> {
        let random_number: usize = random();
        let node_number = random_number % self.nodes.len();
        match self.nodes[node_number].send(payload) {
            Ok(k) => Ok(k),
            Err(e) => {
                log(
                    &format!("Error writing to TCPStream: {:?}. Trying a dif node", e) as &str,
                    QUIET,
                );
                self.nodes.swap_remove(node_number);
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

    pub fn _send_to_all(&mut self, payload: &[u8]) -> io::Result<()> {
        for node in self.nodes.iter_mut() {
            match node.send(payload) {
                Ok(k) => k,
                Err(e) => {
                    log(
                        &format!("Error writing to TCPStream: {:?}. Trying a dif node", e) as &str,
                        QUIET,
                    );
                    continue;
                }
            };
        }
        Ok(())
    }
}
