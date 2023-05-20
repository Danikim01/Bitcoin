use std::io;
mod block_header;
mod config;
mod connection;
mod messages;
mod raw_transaction;
mod serialized_blocks;
mod utility;
mod node;
fn main() -> Result<(), io::Error> {
    let mut nodes = connection::connect_to_network()?;
    connection::sync(&mut nodes)?;
    Ok(())
}
