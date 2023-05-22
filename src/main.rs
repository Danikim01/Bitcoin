use std::io;
mod config;
mod messages;
mod network_controller;
mod node_controller;
mod node;
mod raw_transaction;
mod utility;

fn main() -> Result<(), io::Error> {
    let mut controller = network_controller::NetworkController::new()?;
    println!("Connected to network, starting sync");
    // move this to another thread before adding gtk
    controller.start_sync()?;
    Ok(())
}
