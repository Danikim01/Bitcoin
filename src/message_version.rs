use crate::message_header::{self, MessageHeader};
use crate::messages::{Message, Service};
use bitcoin_hashes::sha256;
use bitcoin_hashes::Hash;
use std::io::{Cursor, Read, Write};
use std::net::IpAddr;
use std::net::{Ipv4Addr, SocketAddr};
use std::net::{Ipv6Addr, TcpStream};
use std::str::FromStr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug)]
pub struct Version {
    message_header: MessageHeader,
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
        let message_header = MessageHeader::default();
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
            message_header,
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
        message_header: MessageHeader,
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
            message_header,
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
        let mut cursor = Cursor::new(bytes);

        // header
        let mut messageHeaderBytes = [0_u8; 24];
        cursor
            .read_exact(&mut messageHeaderBytes)
            .map_err(|error| error.to_string())?;
        let message_header =
            MessageHeader::from_bytes(&messageHeaderBytes).map_err(|error| error.to_string())?;

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
            message_header,
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

    fn build_payload(&self, stream: &mut TcpStream) -> std::io::Result<Vec<u8>> {
        // Get the transmitting node's IP address
        let addr_trans = stream.peer_addr()?.to_string();

        // y esto???
        // Parse the IP address
        let adr_trans_ip_addr = addr_trans
            .split(":")
            .next()
            .unwrap()
            .parse::<IpAddr>()
            .unwrap();

        // Convert IPv4 addresses to IPv6 if needed
        let ip_v6 = match adr_trans_ip_addr {
            IpAddr::V4(ip_v4) => {
                let ipv4_bytes = ip_v4.octets();
                let mut ipv6_bytes = [0; 16];
                ipv6_bytes[10] = 0xff;
                ipv6_bytes[11] = 0xff;
                ipv6_bytes[12..].copy_from_slice(&ipv4_bytes);
                Ipv6Addr::from(ipv6_bytes)
            }
            IpAddr::V6(ip_v6) => ip_v6,
        };
        // fin y esto ???

        // Convert the IPv6 address to a byte array
        let mut ip_bytes: [u8; 16] = ip_v6.octets();

        // Build payload
        // https://developer.bitcoin.org/reference/p2p_networking.html#version
        let mut payload = Vec::new();
        payload.extend(&self.version.to_le_bytes());
        let service_bytes: [u8; 8] = self.service.into();
        payload.extend(&service_bytes);

        payload.extend(&self.timestamp.to_le_bytes());
        payload.extend(&self.addr_recv_services.to_le_bytes());
        payload.extend(&self.addr_recv_ip.octets()); // should change to be?
        payload.extend(&self.addr_recv_port.to_be_bytes());
        payload.extend(&ip_bytes);
        payload.extend(&self.addr_trans_port.to_be_bytes());
        payload.extend(&[self.service as u8; 8]);

        // Add the IPv6 address to the payload
        payload.extend(&ip_bytes);
        Ok(payload)
    }
}

impl Message for Version {
    fn send_to(&self, stream: &mut TcpStream) -> std::io::Result<()> {
        let payload = self.build_payload(stream)?;
        let message = self.build_message("version".to_string(), Some(payload))?;

        stream.write_all(&message)?;
        stream.flush()?;
        Ok(())
    }
}

fn vec_to_arr<T, const N: usize>(v: Vec<T>) -> [T; N] {
    v.try_into()
        .unwrap_or_else(|v: Vec<T>| panic!("Expected a Vec of length {} but it was {}", N, v.len()))
}
