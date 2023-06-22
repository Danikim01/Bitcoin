use gtk::prelude::BuilderExtManual;
use std::io;
use std::sync::mpsc::Sender;

mod overview;
pub mod send_panel;
mod top_bar;

use crate::interface::ModelRequest;


/// Initializes the components of the interface and returns the main window.
pub fn init(builder: gtk::Builder, sender: Sender<ModelRequest>) -> io::Result<gtk::Window> {
    let window: gtk::Window = builder
        .object("main_window")
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Failed to initialize window"))?;
    top_bar::init(builder.clone())?;
    overview::init(builder.clone())?;
    send_panel::init(builder, sender)?;

    Ok(window)
}
