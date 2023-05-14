use std::io;
mod block_header;
mod config;
mod connection;
mod messages;
mod raw_transaction;
mod serialized_blocks;

fn main() -> Result<(), io::Error> {
    connection::connect_to_network()?;
    Ok(())
}
