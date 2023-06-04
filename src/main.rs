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
use std::sync::mpsc;

fn main() -> Result<(), io::Error> {
    let (sender, receiver) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
    let (sender_aux, receiver_aux) = mpsc::channel();

    thread::spawn(|| -> Result<(), io::Error>{
        let mut controller = network_controller::NetworkController::new(sender, receiver_aux)?;
        log("Connected to network, starting sync", VERBOSE);

        controller.start_sync()?;
        Ok(())
    });

    interface::init(receiver, sender_aux)?;

    Ok(())
}
