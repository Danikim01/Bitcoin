use crate::logger::log;
use crate::messages::constants::config::VERBOSE;
use gtk::glib;
use std::io;

mod args_parser;
mod config;
mod interface;
mod logger;
mod messages;
mod network_controller;
mod node;
mod node_controller;
mod raw_transaction;
mod utility;
mod utxo;
mod wallet;
use std::sync::mpsc;
use std::thread;

fn main() -> Result<(), io::Error> {
    let (ui_sender, receiver) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
    let (sender_aux, receiver_aux) = mpsc::channel();
    let (writer_end, node_receiver) = mpsc::channel();
    let config_file = args_parser::get_args();
    let config = config::Config::from_file(config_file)?;
    thread::spawn(move || -> Result<(), io::Error> {
        let outer_controller =
            network_controller::OuterNetworkController::new(ui_sender, writer_end, config.clone())?;
        log("Connected to network, starting sync", VERBOSE, &config);

        outer_controller.start_sync(node_receiver, receiver_aux, config)?;
        Ok(())
    });

    interface::init(receiver, sender_aux)?;
    Ok(())
}
