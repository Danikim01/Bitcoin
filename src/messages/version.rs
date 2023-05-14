use crate::messages::{Message, Services};
use std::io::{self, Cursor, Read, Write};
use std::net::{IpAddr, Ipv6Addr, TcpStream};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use crate::config::Config;
use crate::messages::constants::version_constants::LATEST_VERSION;
use crate::messages::utility::{read_from_varint, EndianRead};

#[derive(Debug)]
pub struct Version {
    // message_header: MessageHeader,
    version: i32,
    services: Services,
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

impl Default for Version {
    fn default() -> Self {
        // let message_header = MessageHeader::default();
        let version = LATEST_VERSION;
        let services = Services::new(0_u64);
        let timestamp = match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(duration) => duration,
            Err(..) => Duration::default(),
        }
            .as_secs() as i64;
        let config = match Config::from_file(){
            Ok(config) => config,
            Err(..) => Config::default(),
        };

        let addr_recv_services = 0;
        let addr_recv_ip = Ipv6Addr::LOCALHOST;
        let addr_recv_port = *config.get_port();
        let addr_trans_services = 0;
        let addr_trans_ip = Ipv6Addr::UNSPECIFIED;
        let addr_trans_port = *config.get_port();
        let nonce = 0;
        let user_agent = "".to_string();
        let start_height = 0;
        let relay = false;
        Version::new(
            // message_header,
            version,
            services,
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
        // message_header: MessageHeader,
        version: i32,
        services: Services,
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
            // message_header,
            version,
            services,
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

    pub fn from_bytes(bytes: &[u8]) -> Result<Version, io::Error> {
        let mut cursor = Cursor::new(bytes);

        let version = Version::new(
            i32::from_le_stream(&mut cursor)?,
            Services::new(u64::from_le_stream(&mut cursor)?),
            i64::from_le_stream(&mut cursor)?,
            u64::from_le_stream(&mut cursor)?,
            Ipv6Addr::from(u128::from_be_stream(&mut cursor)?),
            u16::from_be_stream(&mut cursor)?,
            u64::from_le_stream(&mut cursor)?, // not used
            Ipv6Addr::from(u128::from_be_stream(&mut cursor)?),
            u16::from_be_stream(&mut cursor)?,
            u64::from_le_stream(&mut cursor)?,
            deser_user_agent(&mut cursor)?,
            i32::from_le_stream(&mut cursor)?,
            u8::from_le_stream(&mut cursor)? != 0 // pending: this field should be optional
        );

        Ok(version)
    }

    fn build_payload(&self, stream: &mut TcpStream) -> io::Result<Vec<u8>> {
        // Get the transmitting node's IP address
        // por qué lo estamos pasando a string para después pasarlo a IP????
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
        let ip_bytes: [u8; 16] = ip_v6.octets();

        // Build payload
        // https://developer.bitcoin.org/reference/p2p_networking.html#version
        let mut payload = Vec::new();
        payload.extend(&self.version.to_le_bytes());
        payload.extend::<[u8; 8]>(self.services.into());

        payload.extend(&self.timestamp.to_le_bytes());
        payload.extend(&self.addr_recv_services.to_le_bytes());
        payload.extend(&self.addr_recv_ip.octets()); // should change to be?
        payload.extend(&self.addr_recv_port.to_be_bytes());
        payload.extend(&ip_bytes);
        payload.extend(&self.addr_trans_port.to_be_bytes());
        // Add the IPv6 address to the payload
        payload.extend(&ip_bytes);
        Ok(payload)
    }

    pub fn accepts(&self, another_version: Version) -> bool {
        self.version <= another_version.version
    }
}

impl Message for Version {
    fn send_to(&self, stream: &mut TcpStream) -> io::Result<()> {
        let payload = self.build_payload(stream)?;
        let message = self.build_message("version", Some(payload))?;

        stream.write_all(&message)?;
        stream.flush()?;
        Ok(())
    }
}

fn deser_user_agent(cursor: &mut Cursor<&[u8]>) -> Result<String, io::Error> {
    let user_agent_size = read_from_varint(cursor)? as usize;
    let mut buffer = vec![0_u8; user_agent_size];
    cursor.read_exact(&mut buffer)?;

    match std::str::from_utf8(&buffer) {
        Ok(user_agent) => Ok(user_agent.to_string()),
        Err(e) => Err(io::Error::new(io::ErrorKind::InvalidData, e.to_string()))
    }
}