mod connection;
mod message_verack;
mod message_version;
mod messages;

fn main() -> Result<(), String> {
    connection::connect_to_network()?;
    Ok(())
}
