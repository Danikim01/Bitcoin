use crate::logger::log;
use std::io;
use gtk::glib;
use crate::messages::constants::config::VERBOSE;

mod config;
mod interface;
mod logger;
mod merkle_tree;
mod messages;
mod network_controller;
mod node;
mod node_controller;
mod raw_transaction;
mod utility;
mod utxo;

use std::thread;

fn main() -> Result<(), io::Error> {
    let (sender, receiver) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);

    thread::spawn(|| -> Result<(), io::Error>{
        let mut controller = network_controller::NetworkController::new(sender)?;
        log("Connected to network, starting sync", VERBOSE);

        controller.start_sync()?;
        Ok(())
    });

    interface::init(receiver)?;

    Ok(())
}
