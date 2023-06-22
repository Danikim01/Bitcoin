use crate::config::Config;
use crate::messages::constants::{commands::VERSION, version_constants::LATEST_VERSION};
use crate::messages::utility::{read_from_varint, StreamRead};
use crate::messages::{Message, Serialize, Services};
use crate::utility::actual_timestamp_or_default;
use std::io::{self, Cursor, Read};
use std::net::{IpAddr, Ipv6Addr, SocketAddr};

/// Struct that contains a version message
#[derive(Debug, Clone)]
pub struct Version {
    // message_header: MessageHeader,
    version: i32,
    services: Services,
    timestamp: i64,
    addr_recv_services: u64,
    addr_recv_ip: Ipv6Addr,
    addr_recv_port: u16,
    _addr_trans_services: u64,
    addr_trans_ip: Ipv6Addr,
    addr_trans_port: u16,
    nonce: u64,
    _user_agent: String,
    _start_height: i32,
    _relay: bool,
}

impl Default for Version {
    fn default() -> Self {
        let version = LATEST_VERSION;
        let services = Services::new(0_u64);
        let timestamp = actual_timestamp_or_default();
        let config = Config::from_file_or_default();
        let addr_recv_services = 0;
        let addr_recv_ip = Ipv6Addr::LOCALHOST;
        let addr_recv_port = *config.get_port();
        let _addr_trans_services = 0;
        let addr_trans_ip = Ipv6Addr::LOCALHOST;
        let addr_trans_port = *config.get_port();
        let nonce = 0;
        let _user_agent = "".to_string();
        let _start_height = 0;
        let _relay = false;
        Version::new(
            // message_header,
            version,
            services,
            timestamp,
            addr_recv_services,
            addr_recv_ip,
            addr_recv_port,
            _addr_trans_services,
            addr_trans_ip,
            addr_trans_port,
            nonce,
            _user_agent,
            _start_height,
            _relay,
        )
    }
}

impl Version {
    #[allow(clippy::too_many_arguments)]
    fn new(
        version: i32,
        services: Services,
        timestamp: i64,
        addr_recv_services: u64,
        addr_recv_ip: Ipv6Addr,
        addr_recv_port: u16,
        _addr_trans_services: u64,
        addr_trans_ip: Ipv6Addr,
        addr_trans_port: u16,
        nonce: u64,
        _user_agent: String,
        _start_height: i32,
        _relay: bool,
    ) -> Self {
        Self {
            version,
            services,
            timestamp,
            addr_recv_services,
            addr_recv_ip,
            addr_recv_port,
            _addr_trans_services,
            addr_trans_ip,
            addr_trans_port,
            nonce,
            _user_agent,
            _start_height,
            _relay,
        }
    }

    /// Returns a new version message with the given address, other values will set to default
    pub fn default_for_trans_addr(address: SocketAddr) -> Self {
        Version {
            addr_trans_ip: match address.ip() {
                IpAddr::V4(ip4) => ip4.to_ipv6_compatible(),
                IpAddr::V6(ip6) => ip6,
            },
            addr_trans_port: address.port(),
            ..Default::default()
        }
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

    /// Returns true if the version is accepted by the other version (the other version is newer)
    pub fn accepts(&self, another_version: Version) -> bool {
        self.version <= another_version.version
    }
}

impl Serialize for Version {
    fn serialize(&self) -> io::Result<Vec<u8>> {
        let payload = self.build_payload()?;
        let message = self.build_message(VERSION, Some(payload))?;
        Ok(message)
    }

    fn deserialize(bytes: &[u8]) -> Result<Message, io::Error> {
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
        Ok(Message::Version(version))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::MessageHeader;
    #[test]
    fn test_version_header() {
        let header = [
            0x76, 0x65, 0x72, 0x73, 0x69, 0x6F, 0x6E, 0x00, 0x00, 0x00, 0x00, 0x00, 0x55, 0x00,
            0x00, 0x00, 0x66, 0x66, 0x66, 0x66,
        ];

        let message_header = MessageHeader::from_bytes(&header).unwrap();

        assert_eq!(message_header.command_name, "version\0\0\0\0\0".to_string());
        assert_eq!(message_header.payload_size, 85);
        assert_eq!(message_header.checksum, [0x66, 0x66, 0x66, 0x66]);
    }

    #[test]
    fn test_read_version() {
        //Ejemplo tomado de: https://en.bitcoin.it/wiki/Protocol_documentation#version
        let data_version: Vec<u8> = vec![
            127, 17, 1, 0, 13, 4, 0, 0, 0, 0, 0, 0, 145, 212, 106, 100, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 255, 255, 200, 105, 43, 35, 207, 40, 13, 4, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 212, 180, 39, 227,
            7, 96, 85, 98, 16, 47, 83, 97, 116, 111, 115, 104, 105, 58, 48, 46, 49, 54, 46, 51, 47,
            153, 38, 37, 0, 1,
        ];

        if let Message::Version(data) = Version::deserialize(&data_version).unwrap() {
            assert_eq!(data.version, 70015);
            assert_eq!(data.services, Services { bitmap: 1037 });
            assert_eq!(data.timestamp, 1684722833);
            assert_eq!(data.addr_recv_services, 0);
            let expected_addr = "::ffff:200.105.43.35"
                .parse::<Ipv6Addr>()
                .expect("Invalid IPv6 address");
            assert_eq!(data.addr_recv_ip.to_string(), expected_addr.to_string());
            assert_eq!(data.addr_recv_port, 53032);
            assert_eq!(data._addr_trans_services, 1037);
            assert_eq!(data.addr_trans_port, 0);
            assert_eq!(data.nonce, 7085675175729411284);
            assert_eq!(data._user_agent, "/Satoshi:0.16.3/".to_string());
            assert_eq!(data._start_height, 2434713);
            assert_eq!(data._relay, true);
        }
    }
}
