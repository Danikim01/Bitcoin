use gtk::prelude::BuilderExtManual;
use std::io;

mod overview;
mod top_bar;

pub fn init(builder: gtk::Builder) -> io::Result<gtk::Window> {
    let window: gtk::Window = builder.object("main_window").unwrap(); // add err handling
    top_bar::init(builder.clone())?;
    overview::init(builder)?;

    Ok(window)
}
