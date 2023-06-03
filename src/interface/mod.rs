use gtk::glib;
use gtk::glib::Receiver;
use gtk::prelude::*;
use std::io;

mod components;

pub enum GtkMessage {
    UpdateStatus(String),
}

fn attach_rcv(receiver: Receiver<GtkMessage>, builder: gtk::Builder) {
    // let status_bar: gtk::Label = builder.object("status_bar").unwrap(); // add err handling

    receiver.attach(None, move |msg| {
        match msg {
            GtkMessage::UpdateStatus(text) => {
                let status_bar: gtk::Label = builder.object("status_bar").unwrap(); // add err handling
                status_bar.set_text(text.as_str())
            }
        }
        // Returning false here would close the receiver
        // and have senders fail
        glib::Continue(true)
    });
}

pub fn init(receiver: Receiver<GtkMessage>) -> io::Result<()> {
    if gtk::init().is_err() {
        println!("Failed to initialize GTK.");
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "GTK init failed.",
        ));
    }

    let glade_src = include_str!("./res/ui.glade");
    let builder = gtk::Builder::from_string(glade_src);

    attach_rcv(receiver, builder.clone());

    let window: gtk::Window = builder.object("main_window").unwrap(); // add err handling - move to components
    components::init(builder)?;

    window.show_all();

    gtk::main();

    Ok(())
}
