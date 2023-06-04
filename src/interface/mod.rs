use gtk::glib;
use gtk::glib::Receiver;
use gtk::prelude::*;
use std::io;
use std::sync::mpsc::Sender;

mod components;

pub enum GtkMessage {
    UpdateStatus(String),
    UpdateBotton(String),
}

pub enum ModelRequest {
    GetWalletBalance,
}

fn attach_rcv(receiver: Receiver<GtkMessage>, builder: gtk::Builder) {
    receiver.attach(None, move |msg| {
        match msg {
            GtkMessage::UpdateStatus(text) => {
                let status_bar: gtk::Label = builder.object("status_bar").unwrap(); // add err handling
                status_bar.set_text(text.as_str())
            }
            GtkMessage::UpdateBotton(text) => {
                let builder_aux = builder.clone();
                let get_balance_btn: gtk::Button = builder_aux.object("get_balance_btn").unwrap();
                get_balance_btn.connect_clicked(move |_| {
                    let balance_available_val: gtk::Label =
                        builder_aux.object("balance_available_val").unwrap(); // add err handling
                    balance_available_val.set_text(text.as_str());
                });
            }
        }

        // Returning false here would close the receiver
        // and have senders fail
        glib::Continue(true)
    });
}

pub fn init(receiver: Receiver<GtkMessage>, sender: Sender<ModelRequest>) -> io::Result<()> {
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

    // this only for example
    let get_balance_btn = builder.object::<gtk::Button>("get_balance_btn").unwrap();
    get_balance_btn.connect_clicked(move |_| {
        println!("click");
        sender.send(ModelRequest::GetWalletBalance).unwrap();
    });

    let window: gtk::Window = components::init(builder)?;
    window.show_all();

    gtk::main();
    Ok(())
}

// model -> view
// view -> model??
