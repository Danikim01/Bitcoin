use crate::messages::{Message, Service};
use std::io::{Cursor, Read, Write};
use std::net::{Ipv6Addr, TcpStream};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use bitcoin_hashes::sha256;
use bitcoin_hashes::Hash;

#[derive(Debug)]
pub struct Version {
    version: i32,
    service: Service,
    timestamp: i64,
    addr_recv_services: u64,
    addr_recv_ip: Ipv6Addr,
    addr_recv_port: u16,
    addr_trans_services: u64,
    addr_trans_ip: Ipv6Addr,
    addr_trans_port: u16,
    nonce: u64,
    user_agent: String,
    start_height: i32,
    relay: bool,
}

impl std::default::Default for Version {
    fn default() -> Self {
        let version = 70015;
        let service = Service::Unnamed;
        let timestamp = match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(duration) => duration,
            Err(..) => Duration::default(),
        }
        .as_secs() as i64;
        let addr_recv_services = 0;
        let addr_recv_ip = Ipv6Addr::LOCALHOST;
        let addr_recv_port = 18333;
        let addr_trans_services = 0;
        let addr_trans_ip = Ipv6Addr::UNSPECIFIED;
        let addr_trans_port = 18333;
        let nonce = 0;
        let user_agent = "".to_string();
        let start_height = 0;
        let relay = false;
        Version::new(
            version,
            service,
            timestamp,
            addr_recv_services,
            addr_recv_ip,
            addr_recv_port,
            addr_trans_services,
            addr_trans_ip,
            addr_trans_port,
            nonce,
            user_agent,
            start_height,
            relay,
        )
    }
}

impl Version {
    fn new(
        version: i32,
        service: Service,
        timestamp: i64,
        addr_recv_services: u64,
        addr_recv_ip: Ipv6Addr,
        addr_recv_port: u16,
        addr_trans_services: u64,
        addr_trans_ip: Ipv6Addr,
        addr_trans_port: u16,
        nonce: u64,
        user_agent: String,
        start_height: i32,
        relay: bool,
    ) -> Self {
        Self {
            version,
            service,
            timestamp,
            addr_recv_services,
            addr_recv_ip,
            addr_recv_port,
            addr_trans_services,
            addr_trans_ip,
            addr_trans_port,
            nonce,
            user_agent,
            start_height,
            relay,
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Version, String> {
        // since this is a method, change it to modify self instead of returning a new Message
        // println!("received: {:?}", bytes);

        let mut cursor = Cursor::new(bytes);

        // header
        let mut magic_bytes = [0_u8; 4];
        let mut command_name = [0_u8; 12];
        let mut payload_size = [0_u8; 4];
        let mut checksum = [0_u8; 4];

        // read header
        cursor
            .read_exact(&mut magic_bytes)
            .map_err(|error| error.to_string())?;
        cursor
            .read_exact(&mut command_name)
            .map_err(|error| error.to_string())?;
        cursor
            .read_exact(&mut payload_size)
            .map_err(|error| error.to_string())?;
        cursor
            .read_exact(&mut checksum)
            .map_err(|error| error.to_string())?;

        println!(
            "\nMagic bytes: {:02X?}\nCommand name: {:?}\nPayload size: {:?}\nChecksum: {:02X?}\n",
            magic_bytes,
            std::str::from_utf8(&command_name).map_err(|error| error.to_string())?,
            u32::from_le_bytes(payload_size),
            checksum
        );

        // payload
        let mut version = [0_u8; 4];
        let mut services = [0_u8; 8];
        let mut timestamp = [0_u8; 8];
        let mut addr_recv_services = [0_u8; 8];
        let mut addr_recv_ip = [0_u8; 16];
        let mut addr_recv_port = [0_u8; 2];
        let mut addr_trans_services = [0_u8; 8]; // not used
        let mut addr_trans_ip = [0_u8; 16]; // not used
        let mut addr_trans_port = [0_u8; 2];
        let mut nonce = [0_u8; 8];
        let user_agent_size: u64;
        let mut start_height = [0_u8; 4];
        let mut relay = [0_u8; 1];

        // read payload
        cursor
            .read_exact(&mut version)
            .map_err(|error| error.to_string())?;
        cursor
            .read_exact(&mut services)
            .map_err(|error| error.to_string())?;
        cursor
            .read_exact(&mut timestamp)
            .map_err(|error| error.to_string())?;
        cursor
            .read_exact(&mut addr_recv_services)
            .map_err(|error| error.to_string())?;
        cursor
            .read_exact(&mut addr_recv_ip)
            .map_err(|error| error.to_string())?;
        cursor
            .read_exact(&mut addr_recv_port)
            .map_err(|error| error.to_string())?;
        cursor
            .read_exact(&mut addr_trans_services)
            .map_err(|error| error.to_string())?;
        cursor
            .read_exact(&mut addr_trans_ip)
            .map_err(|error| error.to_string())?;
        cursor
            .read_exact(&mut addr_trans_port)
            .map_err(|error| error.to_string())?;
        cursor
            .read_exact(&mut nonce)
            .map_err(|error| error.to_string())?;

        let mut byte = [0_u8; 1];
        cursor
            .read_exact(&mut byte)
            .map_err(|error| error.to_string())?;
        if byte[0] < 0xFD {
            user_agent_size = byte[0] as u64;
        } else {
            let mut buffer_size = 0;
            match byte[0] {
                0xFF => buffer_size = 8,
                0xFE => buffer_size = 4,
                0xFD => buffer_size = 2,
                _ => {}
            };
            let mut user_agent_bytes = vec![0_u8; buffer_size];
            cursor
                .read_exact(&mut user_agent_bytes)
                .map_err(|error| error.to_string())?;
            user_agent_size = u64::from_be_bytes(vec_to_arr(user_agent_bytes));
        }
        let mut user_agent = vec![0_u8; user_agent_size as usize];
        cursor
            .read_exact(&mut user_agent)
            .map_err(|error| error.to_string())?;
        cursor
            .read_exact(&mut start_height)
            .map_err(|error| error.to_string())?;
        cursor
            .read_exact(&mut relay)
            .map_err(|error| error.to_string())?; // pending: this field should be optional

        Ok(Version::new(
            i32::from_le_bytes(version),
            Service::from(services),
            i64::from_le_bytes(timestamp),
            u64::from_le_bytes(addr_recv_services),
            Ipv6Addr::from(addr_recv_ip),
            u16::from_be_bytes(addr_recv_port),
            u64::from_le_bytes(addr_trans_services),
            Ipv6Addr::from(addr_trans_ip),
            u16::from_be_bytes(addr_trans_port),
            u64::from_le_bytes(nonce),
            std::str::from_utf8(&user_agent)
                .map_err(|error| error.to_string())?
                .to_string(),
            i32::from_le_bytes(start_height),
            relay[0] != 0,
        ))
    }
}

impl Message for Version {
    fn send_to(&self, stream: &mut TcpStream) -> std::io::Result<()> {
        println!("stream: {:?}", stream);
        let addr_trans = stream.peer_addr()?.to_string();

        // Build payload
        // https://developer.bitcoin.org/reference/p2p_networking.html#version
        let mut payload = Vec::new();
        payload.extend(&self.version.to_le_bytes());
        payload.extend(&[self.service as u8; 8]);
        payload.extend(&self.timestamp.to_le_bytes());
        payload.extend(&self.addr_recv_services.to_le_bytes());
        payload.extend(&self.addr_recv_ip.octets()); // should change to be?
        payload.extend(&self.addr_recv_port.to_be_bytes());
        payload.extend(&[self.service as u8; 8]); // addr_trans_services
        payload.extend(addr_trans.as_bytes()); // adr_trans_ip_addr
        payload.extend(&self.addr_trans_port.to_be_bytes());
        payload.extend(&self.nonce.to_le_bytes());
        payload.extend(&(self.user_agent.len() as u32).to_le_bytes());
        payload.extend(self.user_agent.as_bytes()); // what happends if user_agent_bytes is 0?
        payload.extend(&self.start_height.to_le_bytes());
        payload.extend(&[self.relay as u8]);

        // Build message header
        // https://developer.bitcoin.org/reference/p2p_networking.html#message-headers
        let magic_value: [u8; 4] = 0x0b110907u32.to_be_bytes();
        let command = b"version\0\0\0\0\0".to_owned();
        let payload_size: [u8; 4] = (payload.len() as u32).to_le_bytes();
        let mut checksum = sha256::Hash::hash(&payload).to_byte_array(); // first hash
        checksum = sha256::Hash::hash(&checksum).to_byte_array(); // second hash

        // Concat header and payload
        let message = [
            magic_value.to_vec(),
            command.to_vec(),
            payload_size.to_vec(),
            checksum[0..4].to_vec(),
            payload,
        ]
        .concat();

        stream.write_all(&message)?;
        stream.flush()?;
        // println!("write data: {:?}", message);
        Ok(())
    }
}

fn vec_to_arr<T, const N: usize>(v: Vec<T>) -> [T; N] {
    v.try_into()
        .unwrap_or_else(|v: Vec<T>| panic!("Expected a Vec of length {} but it was {}", N, v.len()))
}
