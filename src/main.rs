mod connection;
mod messageVersion;
mod messages;

fn main() -> Result<(), String> {
    connection::connect_to_network()?;
    Ok(())
}
