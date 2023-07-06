use gtk::prelude::BuilderExtManual;
use std::io;
use std::sync::mpsc::Sender;

mod blocks_panel;
mod headers_panel;
pub mod overview_panel;
pub mod send_panel;
pub mod table;
mod top_bar;
mod transactions_panel;
pub mod utils;
pub mod wallet_switcher;

use crate::interface::ModelRequest;

/// Initializes the components of the interface and returns the main window.
pub fn init(builder: gtk::Builder, sender: Sender<ModelRequest>) -> io::Result<gtk::Window> {
    let window: gtk::Window = builder
        .object("main_window")
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Failed to initialize window"))?;
    top_bar::init(builder.clone())?;
    wallet_switcher::init(builder.clone(), sender.clone())?;
    overview_panel::init(builder.clone())?;
    send_panel::init(builder, sender)?;

    Ok(window)
}
