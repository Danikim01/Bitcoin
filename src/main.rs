mod connection;
mod header;
mod message_verack;
mod message_version;
mod messages;
mod message_getblocks;
mod message_header;

fn main() -> Result<(), String> {
    connection::connect_to_network()?;
    Ok(())
}
