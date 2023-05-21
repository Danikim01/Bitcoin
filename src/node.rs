use crate::messages::Block;
use crate::messages::{
    constants::commands, Headers, Message, MessageHeader, Serialize, VerAck, Version,
};
use crate::utility::to_io_err;
use std::io::{self, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::mpsc;
use std::thread::{self, JoinHandle};

pub struct Listener {
    stream: TcpStream,
    writer_channel: mpsc::Sender<Message>,
}

impl Listener {
    pub fn new(stream: TcpStream, writer_channel: mpsc::Sender<Message>) -> Self {
        Self {
            stream,
            writer_channel,
        }
    }

    pub fn listen(mut self) -> io::Result<()> {
        while true {
            let message_header = MessageHeader::from_stream(&mut self.stream)?;
            let payload = message_header.read_payload(&mut self.stream)?;
            let dyn_message: Message = match message_header.command_name.as_str() {
                commands::HEADERS => Headers::deserialize(&payload)?,
                commands::BLOCK => Block::deserialize(&payload)?,
                _ => continue,
            };
            self.writer_channel.send(dyn_message).map_err(to_io_err)?;
        }
        Ok(())
    }
}

pub struct Node {
    pub stream: TcpStream,
    listener: thread::JoinHandle<io::Result<()>>,
}

impl Node {
    fn new(stream: TcpStream, listener: JoinHandle<io::Result<()>>) -> Self {
        println!("Established connection with node: {:?}", stream);
        Self { stream, listener }
    }

    fn spawn(stream: TcpStream, writer_channel: mpsc::Sender<Message>) -> io::Result<Self> {
        let listener = Listener::new(stream.try_clone()?, writer_channel);
        let handle = thread::spawn(move || listener.listen());
        Ok(Self::new(stream, handle))
    }

    pub fn try_from_addr(
        node_addr: SocketAddr,
        writer_channel: mpsc::Sender<Message>,
    ) -> Result<Node, io::Error> {
        if !node_addr.is_ipv4() {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "Ipv6 is not supported",
            ));
        }
        let mut stream = TcpStream::connect(node_addr)?;
        Node::handshake(&mut stream).unwrap();
        Node::spawn(stream, writer_channel)
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

    pub fn send(&mut self, payload: Vec<u8>) -> Result<(), io::Error> {
        self.stream.write_all(&payload)?;
        self.stream.flush()?;
        Ok(())
    }
}
