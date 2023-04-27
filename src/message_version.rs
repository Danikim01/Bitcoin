use crate::messages::{Message, Service};
use std::io::{Cursor, Read, Write};
use std::net::Ipv6Addr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug)]
pub struct Version {
    version: i32,
    service: Service,
    timestamp: i64,
    addr_recv_services: u64,
    addr_recv_ip: Ipv6Addr,
    addr_recv_port: u16,
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
        }.as_secs() as i64;
        let addr_recv_services = 0;
        let addr_recv_ip = Ipv6Addr::LOCALHOST;
        let addr_recv_port = 18333;
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
        let mut version = [0_u8; 4];
        let mut services = [0_u8];
        let mut timestamp = [0_u8; 8];
        let mut addr_recv_services = [0_u8; 8];
        let mut addr_recv_ip = [0_u8; 16];
        let mut addr_recv_port = [0_u8; 2];
        let mut addr_trans_port = [0_u8; 2];
        let mut nonce = [0_u8; 8];
        let user_agent_size: u64;
        let mut start_height = [0_u8; 4];
        let mut relay = [0_u8; 1];

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
            cursor.read_exact(&mut user_agent_bytes).map_err(|error| error.to_string())?;
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
            i32::from_be_bytes(version),
            Service::from(services),
            i64::from_be_bytes(timestamp),
            u64::from_be_bytes(addr_recv_services),
            Ipv6Addr::from(addr_recv_ip),
            u16::from_be_bytes(addr_recv_port),
            u16::from_be_bytes(addr_trans_port),
            u64::from_be_bytes(nonce),
            std::str::from_utf8(&user_agent)
                .map_err(|error| error.to_string())?
                .to_string(),
            i32::from_be_bytes(start_height),
            relay[0] != 0,
        ))
    }
}

impl Message for Version {
    fn send_to(&self, stream: &mut dyn Write) -> std::io::Result<()> {
        let mut output = Vec::new();

        output.extend(&self.version.to_be_bytes());
        output.extend(&[self.service as u8]);
        output.extend(&self.timestamp.to_be_bytes());
        output.extend(&self.addr_recv_services.to_be_bytes());
        output.extend(&self.addr_recv_ip.octets());
        output.extend(&self.addr_recv_port.to_be_bytes());
        output.extend(&self.addr_trans_port.to_be_bytes());
        output.extend(&self.nonce.to_be_bytes());
        output.extend(&(self.user_agent.len() as u32).to_be_bytes());
        output.extend(self.user_agent.as_bytes()); // what happends if user_agent_bytes is 0?
        output.extend(&self.start_height.to_be_bytes());
        output.extend(&[self.relay as u8]);

        stream.write_all(&output)?;
        Ok(())
    }
}


fn vec_to_arr<T, const N: usize>(v: Vec<T>) -> [T; N] {
    v.try_into()
        .unwrap_or_else(|v: Vec<T>| panic!("Expected a Vec of length {} but it was {}", N, v.len()))
}
