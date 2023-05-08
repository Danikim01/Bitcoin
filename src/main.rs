use std::io;
mod block_header;
mod connection;
mod messages;

fn main() -> Result<(), io::Error> {
    connection::connect_to_network()?;
    Ok(())
}
