use crate::messages::{Message, MessageHeader, VerAck, Version};
use std::io::{self, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::mpsc;
use std::thread::{self, JoinHandle};

pub struct Node {
    pub stream: TcpStream,
    listener: thread::JoinHandle<()>,
}

struct Listener {
    stream: TcpStream,
    writer_channel: mpsc::Sender<u8>,
}

impl Listener {
    fn new(stream: TcpStream, writer_channel: mpsc::Sender<u8>) -> Self {
        Self {
            stream,
            writer_channel,
        }
    }

    fn listen(self) -> () {}
}

impl Node {
    fn new(stream: TcpStream, listener: JoinHandle<()>) -> Self {
        Self { stream, listener }
    }

    fn spawn(stream: TcpStream, writer_channel: mpsc::Sender<u8>) -> Result<Self, io::Error> {
        let listener = Listener::new(stream.try_clone()?, writer_channel);
        let handle = thread::spawn(move || listener.listen());
        Ok(Self::new(stream, handle))
    }

    pub fn try_from_addr(
        node_addr: SocketAddr,
        writer_channel: mpsc::Sender<u8>,
    ) -> Result<Node, io::Error> {
        if !node_addr.is_ipv4() {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "Ipv6 is not supported",
            ));
        }
        let mut stream = TcpStream::connect(node_addr)?;
        Node::handshake(&mut stream)?;
        Node::spawn(stream, writer_channel)
    }

    fn handshake(stream: &mut TcpStream) -> io::Result<()> {
        // send message
        let msg_version = Version::default();
        let payload = msg_version.serialize()?;
        stream.write_all(&payload)?;
        stream.flush()?;

        let message_header = MessageHeader::from_stream(stream)?;
        let payload_data = message_header.read_payload(stream)?;
        let version_message = Version::from_bytes(&payload_data)?;

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

    pub fn send(&mut self, payload: Vec<u8>) -> Result<(), io::Error> {
        self.stream.write_all(&payload)?;
        self.stream.flush()?;
        Ok(())
    }
}
