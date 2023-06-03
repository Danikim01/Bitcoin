use crate::logger::log;
use crate::messages::constants::config::VERBOSE;
use std::io;

mod config;
mod logger;
mod merkle_tree;
mod messages;
mod network_controller;
mod node;
mod node_controller;
mod raw_transaction;
mod utility;
mod utxo;

fn main() -> Result<(), io::Error> {
    let mut controller = network_controller::NetworkController::new()?;
    log("Connected to network, starting sync", VERBOSE);
    // move this to another thread before adding gtk
    controller.start_sync()?;
    Ok(())
}
