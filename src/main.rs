use std::io;
mod config;
mod connection;
mod messages;
mod node;
mod raw_transaction;
mod utility;
fn main() -> Result<(), io::Error> {
    let mut controller = connection::NetworkController::connect_to_network()?;
    println!("Connected to network, starting sync");
    // move this to another thread before adding gtk
    controller.initial_block_download()?;
    Ok(())
}
