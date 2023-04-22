use std::net::{TcpStream, ToSocketAddrs, SocketAddr, Ipv6Addr};
use std::io::{Read, Write};
use std::time::{SystemTime, UNIX_EPOCH};
use varint;

enum Service {
    Unnamed,
    NodeNetwork,
    NodeGetUtxo,
    NodeBloom,
    NodeWitness,
    NodeXthin,
    NodeNetworkLimited,
}

trait Message {
    fn send_to(&self, stream: &mut dyn Write) -> std::io::Result<()>;
    fn receive_to(&self, stream: &dyn Read) -> std::io::Result<()>;   
}

pub struct Version {
    version: i32,
    services: Service,
    timestamp: i64,
    addr_recv_services: u64,
    addr_recv_ip: [u8; 16],
    addr_recv_port: u16,
    addr_trans_port: u16,
    nonce: u64,
    user_agent_bytes: u64, //varsize
    user_agent: &str,
    start_height: i32,
    relay: bool
}

impl Version {
    fn new() -> Self {
    }
}

impl Message for Version {
    fn send_to(&self,stream: &mut dyn Write) -> std::io::Result<()> {       
        // Convertir campos a bytes
        let version_bytes = self.version.to_be_bytes();
        let services_bytes = self.services as u64;
        let timestamp_bytes = self.timestamp.to_be_bytes();
        let addr_recv_services_bytes = self.addr_recv_services.to_be_bytes();
        let addr_recv_ip_bytes = self.addr_recv_ip;
        let addr_recv_port_bytes = self.addr_recv_port.to_be_bytes();
        let addr_trans_port_bytes = self.addr_trans_port.to_be_bytes();
        let nonce_bytes = self.nonce.to_be_bytes();
        let user_agent_bytes = self.user_agent.as_bytes();
        let user_agent_bytes_len = encode_varint(user_agent_bytes.len() as u64);
        let start_height_bytes = self.start_height.to_be_bytes();
        let relay_bytes = [self.relay as u8];

        // Escribir campos en el stream
        stream.write_all(&version_bytes)?;
        stream.write_all(&services_bytes.encode_varint())?;
        stream.write_all(&timestamp_bytes)?;
        stream.write_all(&addr_recv_services_bytes)?;
        stream.write_all(&addr_recv_ip_bytes)?;
        stream.write_all(&addr_recv_port_bytes)?;
        stream.write_all(&addr_trans_port_bytes)?;
        stream.write_all(&nonce_bytes)?;
        stream.write_all(&user_agent_bytes_len)?;
        stream.write_all(&user_agent_bytes)?;
        stream.write_all(&start_height_bytes)?;
        stream.write_all(&relay_bytes)?;

        Ok(())
    }

    fn rcv_from(stream: &mut dyn Read) -> std::io::Result<Version> {       
        let mut data = [0 as u8; 82]; // using 82 byte buffer
        stream.read_exact(&mut data).unwrap();
    
        Ok(())
    }
    
}

impl std::default::Default for Version {
    fn default() -> Self {
        let version = 70015;
        let service = Service::Unnamed;
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;
        let addr_recv_services = 0;
        let addr_recv_ip = Ipv6Addr::new(0,0,0,0,0,0,0,1);
        let addr_recv_port = 18333;
        let addr_trans_port = 18333;
        let nonce = 0;
        let user_agent_bytes = 0;
        let user_agent = "";
        let start_height = 0;
        let relay = 0;
        Version::new(
            version,
            service,
            timestamp,
            addr_recv_services,
            addr_recv_ip,
            addr_recv_port,
            addr_trans_port,
            nonce,
            user_agent_bytes,
            user_agent,
            start_height,
            relay
        )
    }
}
