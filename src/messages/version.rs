use crate::config::Config;
use crate::messages::constants::{version_constants::LATEST_VERSION, commands::VERSION};
use crate::messages::utility::{read_from_varint, EndianRead};
use crate::messages::{Message, Services};
use std::io::{self, Cursor, Read};
use std::net::{IpAddr, Ipv6Addr, SocketAddr};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

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
        let config = match Config::from_file() {
            Ok(config) => config,
            Err(..) => Config::default(),
        };

        let addr_recv_services = 0;
        let addr_recv_ip = Ipv6Addr::LOCALHOST;
        let addr_recv_port = *config.get_port();
        let addr_trans_services = 0;
        let addr_trans_ip = Ipv6Addr::LOCALHOST;
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

    pub fn default_for_trans_addr(address: SocketAddr) -> Self {
        let mut version = Self::default();
        version.addr_trans_ip = match address.ip() {
            IpAddr::V4(ip4) => ip4.to_ipv6_compatible(),
            IpAddr::V6(ip6) => ip6,
        };
        version.addr_trans_port = address.port();
        version
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
            u8::from_le_stream(&mut cursor)? != 0, // pending: this field should be optional
        );

        Ok(version)
    }

    fn build_payload(&self) -> io::Result<Vec<u8>> {
        let mut payload = Vec::new();
        payload.extend(&self.version.to_le_bytes());
        payload.extend::<[u8; 8]>(self.services.into());

        payload.extend(&self.timestamp.to_le_bytes());
        payload.extend(&self.addr_recv_services.to_le_bytes());
        payload.extend(&self.addr_recv_ip.octets());
        payload.extend(&self.addr_recv_port.to_be_bytes());
        payload.extend(&self.addr_recv_services.to_le_bytes());
        payload.extend(&self.addr_trans_ip.octets());
        payload.extend(&self.addr_trans_port.to_be_bytes());
        payload.extend(&self.nonce.to_le_bytes());
        Ok(payload)
    }

    pub fn accepts(&self, another_version: Version) -> bool {
        self.version <= another_version.version
    }
}

impl Message for Version {
    fn serialize(&self) -> std::io::Result<Vec<u8>> {
        let payload = self.build_payload()?;
        let message = self.build_message(VERSION, Some(payload))?;
        Ok(message)
    }
}

fn deser_user_agent(cursor: &mut Cursor<&[u8]>) -> Result<String, io::Error> {
    let user_agent_size = read_from_varint(cursor)? as usize;
    let mut buffer = vec![0_u8; user_agent_size];
    cursor.read_exact(&mut buffer)?;

    match std::str::from_utf8(&buffer) {
        Ok(user_agent) => Ok(user_agent.to_string()),
        Err(e) => Err(io::Error::new(io::ErrorKind::InvalidData, e.to_string())),
    }
}
