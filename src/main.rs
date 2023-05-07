use std::io;
mod connection;
mod header;
mod messages;

fn main() -> Result<(), io::Error> {
    connection::connect_to_network()?;
    Ok(())
}
