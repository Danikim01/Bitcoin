use std::io;
mod config;
mod connection;
mod messages;
mod node;
mod raw_transaction;
mod serialized_blocks;
mod utility;
fn main() -> Result<(), io::Error> {
    let (mut nodes, _mpsc_reader) = connection::connect_to_network()?;
    connection::sync(&mut nodes)?;
    Ok(())
}
