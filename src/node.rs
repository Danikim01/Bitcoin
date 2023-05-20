use std::net::{TcpStream, SocketAddr};
use std::io::{self};
use std::sync::mpsc;
use crate::messages::{Version, VerAck, Message, MessageHeader};
use std::thread::{self, JoinHandle};

pub struct Node {
    stream: TcpStream,
    listener: thread::JoinHandle<()>
}

pub struct Listener {
    stream: TcpStream,
    writer_channel: mpsc::Sender<u8>
}

impl Listener {
    fn new(stream: TcpStream, writer_channel: mpsc::Sender<u8>) -> Self {
        Self {
            stream,
            writer_channel
        }
    }

    fn listen(self) -> () {

    }
}

impl Node {
    fn new(stream: TcpStream, listener: JoinHandle<()>) -> Self {
        Self {
            stream,
            listener
        }
    }

    fn spawn(stream: TcpStream, writer_channel: mpsc::Sender<u8>) -> Result<Self, io::Error> {
        let listener = Listener::new(stream.try_clone()?, writer_channel);        
        let handle = thread::spawn(move || {
            listener.listen()
        });
        Ok(Self::new(stream, handle))
    }

    pub fn try_from_addr(node_addr: SocketAddr, writer_channel: mpsc::Sender<u8>) -> Result<Node, io::Error> {
        if !node_addr.is_ipv4() {
            return Err(io::Error::new(io::ErrorKind::Unsupported, "Ipv6 is not supported"));
        }
        let mut stream = TcpStream::connect(node_addr)?;
        Node::handshake(&mut stream)?;
        Node::spawn(stream, writer_channel)
    }

    fn handshake(stream: &mut TcpStream) -> io::Result<()> {
        // send message
        let msg_version = Version::default();
        msg_version.send_to(stream)?;

        let message_header = MessageHeader::from_stream(stream)?;
        let payload_data = message_header.read_payload(stream)?;
        let version_message = Version::from_bytes(&payload_data)?;

        if !msg_version.accepts(version_message) {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "Version not supported",
            ));
        }

        VerAck::from_stream(stream)?;
        VerAck::new().send_to(stream)?;
        Ok(())
    }
}
