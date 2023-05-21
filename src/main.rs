use std::io;
mod config;
mod connection;
mod messages;
mod node;
mod raw_transaction;
mod serialized_blocks;
mod utility;
fn main() -> Result<(), io::Error> {

    let controller = connection::NetworkController::connect_to_network()?;
    println!("Connected to network, starting sync");
    controller.initial_block_download()?;
    Ok(())
}
