use crate::config::Config;
use crate::messages::utility::StreamRead;
use crate::messages::GetData;
use crate::messages::{
    constants::commands, Block, ErrorType, Headers, Message, MessageHeader, Serialize, VerAck,
    Version,
};
use crate::raw_transaction::RawTransaction;
use crate::utility::to_io_err;
use std::io::{self, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::mpsc;
use std::thread::{self, JoinHandle};
use std::time::Duration;
// gtk imports
use crate::interface::GtkMessage;
use crate::messages::constants::config::VERBOSE;
use gtk::glib::Sender;

/// The Listener struct is responsible for listening to incoming messages from a peer and sending them to the writer thread.
pub struct Listener {
    socket_addr: SocketAddr,
    stream: TcpStream,
    writer_channel: mpsc::Sender<(SocketAddr, Message)>,
}

impl Listener {
    fn new(stream: TcpStream, writer_channel: mpsc::Sender<(SocketAddr, Message)>) -> Self {
        Self {
            socket_addr: stream.peer_addr().unwrap(), // handle this error
            stream,
            writer_channel,
        }
    }

    fn log_listen(mut self, config: &Config) -> io::Result<()> {
        match self.listen() {
            Ok(..) => Ok(()),
            Err(e) => {
                config
                    .get_logger()
                    .log(&format!("{:?}", e) as &str, VERBOSE);
                config.get_logger().log(
                    &format!("Listener for connection {:?} died.", self.stream) as &str,
                    VERBOSE,
                );
                Err(e)
            }
        }
    }

    fn ping_deserialize(payload: &[u8]) -> io::Result<Message> {
        let mut cursor = io::Cursor::new(payload);
        //println!("Received ping message with payload: {:?}", payload);
        let nonce = u64::from_le_stream(&mut cursor)?;
        //println!("Received ping with nonce: {}", nonce);
        Ok(Message::Ping(nonce))
    }

    fn process_message_payload(command_name: &str, payload: Vec<u8>) -> io::Result<Message> {
        let dyn_message: Message = match command_name {
            commands::HEADERS => match Headers::deserialize(&payload) {
                Ok(m) => m,
                Err(e) => {
                    println!("Invalid headers payload: {:?}, ignoring message", e);
                    // HERE WE MUST REQUEST THE BLOCK HEADERS AGAIN!
                    Message::Failure(ErrorType::HeaderError)
                }
            },
            commands::BLOCK => match Block::deserialize(&payload) {
                Ok(m) => m,
                Err(e) => {
                    println!("Invalid block payload: {:?}, ignoring message", e);
                    // HERE WE MUST REQUEST THE BLOCK AGAIN!
                    Message::Failure(ErrorType::BlockError)
                }
            },
            commands::INV => match GetData::deserialize(&payload) {
                Ok(m) => m,
                _ => Message::Ignore(), // bad luck if it fails, we can't request inv to another node
            },
            commands::TX => match RawTransaction::deserialize(&payload) {
                Ok(m) => m,
                _ => Message::Ignore(), // bad luck if it fails, we can't request tx to another node
            },
            commands::PING => match Self::ping_deserialize(&payload) {
                Ok(m) => m,
                _ => Message::Ignore(),
            },
            _ => Message::Ignore(),
        };
        Ok(dyn_message)
    }

    fn listen(&mut self) -> io::Result<()> {
        loop {
            let message_header = MessageHeader::from_stream(&mut self.stream)?;
            if message_header.validate_header().is_err() {
                println!(
                    "Invalid or unimplemented header: {:?}, ignoring message",
                    message_header
                );
                continue;
            }

            let payload = message_header.read_payload(&mut self.stream)?;
            match Self::process_message_payload(&message_header.command_name, payload) {
                Ok(Message::Ignore()) => continue,
                Ok(m) => {
                    self.writer_channel
                        .send((self.socket_addr, m))
                        .map_err(to_io_err)?;
                }
                _ => continue,
            }
        }
    }
}

/// The Node struct is responsible for spawning a listener thread and keeping track of the connection.
#[derive(Debug)]
pub struct Node {
    pub stream: TcpStream,
    pub address: SocketAddr,
    _listener: JoinHandle<io::Result<()>>,
}

impl Node {
    fn new(
        stream: TcpStream,
        listener: JoinHandle<io::Result<()>>,
        ui_sender: Sender<GtkMessage>,
        config: &Config,
    ) -> io::Result<Self> {
        let message = &format!("Established connection with node: {:?}", stream) as &str;
        config.get_logger().log(message, VERBOSE);

        // update ui // handle error
        let _ = ui_sender.send(GtkMessage::UpdateLabel((
            "status_bar".to_string(),
            message.to_string(),
        )));
        let address = stream.peer_addr()?;

        Ok(Self {
            stream,
            address,
            _listener: listener,
        })
    }

    fn spawn(
        stream: TcpStream,
        writer_channel: mpsc::Sender<(SocketAddr, Message)>,
        ui_sender: Sender<GtkMessage>,
        config: Config,
    ) -> io::Result<Self> {
        let listener = Listener::new(stream.try_clone()?, writer_channel);
        let config_clone = config.clone();
        let handle = thread::spawn(move || listener.log_listen(&config));
        Self::new(stream, handle, ui_sender, &config_clone)
    }

    fn _is_alive(&mut self, config: &Config) -> bool {
        let mut buf = [0u8; 1];
        config.get_logger().log("is_alive: peeking", VERBOSE);
        let bytes_read = self.stream.peek(&mut buf);
        config.get_logger().log("is_alive: done peeking", VERBOSE);
        match bytes_read {
            Ok(_) => true,
            Err(..) => false,
        }
    }

    /// This function is used to establish a connection with a node.
    pub fn try_from_addr(
        node_addr: SocketAddr,
        writer_channel: mpsc::Sender<(SocketAddr, Message)>,
        ui_sender: Sender<GtkMessage>,
        config: Config,
    ) -> io::Result<(SocketAddr, Node)> {
        if !node_addr.is_ipv4() {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "Ipv6 is not supported",
            ));
        }
        let tcp_timeout = config.get_tcp_timeout();
        let mut stream = TcpStream::connect_timeout(&node_addr, Duration::new(tcp_timeout, 0))?;
        Node::handshake(&mut stream)?;
        let node = Node::spawn(stream, writer_channel, ui_sender, config)?;
        Ok((node.address, node))
    }

    fn handshake(stream: &mut TcpStream) -> io::Result<()> {
        // send message
        let msg_version = Version::default_for_trans_addr(stream.peer_addr()?);
        let payload = msg_version.serialize()?;
        stream.write_all(&payload)?;
        stream.flush()?;
        let message_header = MessageHeader::from_stream(stream)?;
        let payload_data = message_header.read_payload(stream)?;
        let version_message = match Version::deserialize(&payload_data)? {
            Message::Version(version_message) => version_message,
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Expected Version message",
                ));
            }
        };

        if !msg_version.accepts(version_message) {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "Version not supported",
            ));
        }
        VerAck::from_stream(stream)?; // receive verack
        let payload = VerAck::new().serialize()?;
        stream.write_all(&payload)?; // send verack
        stream.flush()?;
        Ok(())
    }

    /// This function is used to send a message to a node a payload.
    pub fn send(&mut self, payload: &[u8]) -> io::Result<()> {
        self.stream.write_all(payload)?;
        self.stream.flush()?;
        Ok(())
    }
}
